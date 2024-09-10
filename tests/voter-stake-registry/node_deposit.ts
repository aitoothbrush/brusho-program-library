import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { mintTokenToWallet, assertThrowsAnchorError, assertThrowsSendTransactionError, newSigner, VSR_PROGRAM, getTokenAccount, fastup, lockupDayily, lockupMonthly, createRealm, createRegistrar, createVoter, defaultDepositConfig, defaultVotingConfig, newTokenAccount, mintTokenToAccount, CONNECTION } from "./helper";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

describe("node_deposit!", () => {
  let authority: web3.Keypair;
  let mint: web3.PublicKey;
  let councilMint: web3.PublicKey;
  let realm: web3.PublicKey;
  let registrar: web3.PublicKey;
  let voterAuthority: web3.Keypair;
  let voter: web3.PublicKey;
  let voterWeightRecord: web3.PublicKey;
  let vault: web3.PublicKey;
  let tokenOwnerRecord: web3.PublicKey;
  let depositToken: web3.PublicKey;

  before(async () => {
    authority = await newSigner();
    [mint, councilMint, realm] = await createRealm(authority);
    // create registrar
    [registrar] = await createRegistrar(realm, authority, mint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), authority);
    [voterAuthority, voter, voterWeightRecord, vault, tokenOwnerRecord] = await createVoter(realm, mint, registrar, authority);
    depositToken = await newTokenAccount(mint, voterAuthority);

    // console.log(`authority: ${authority.publicKey.toBase58()}`)
    // console.log(`mint: ${mint.toBase58()}`)
    // console.log(`councilMint: ${councilMint.toBase58()}`)
    // console.log(`realm: ${realm.toBase58()}`)
    // console.log(`registrar: ${registrar.toBase58()}`)
    // console.log(`voterAuthority: ${voterAuthority.publicKey.toBase58()}`)
    // console.log(`voter: ${voter.toBase58()}`)
    // console.log(`voterWeightRecord: ${voterWeightRecord.toBase58()}`)
    // console.log(`vault: ${vault.toBase58()}`)
    // console.log(`tokenOwnerRecord: ${tokenOwnerRecord.toBase58()}`)
    // console.log(`voterTokenAccount: ${depositToken.toBase58()}`)
  })


  it("with_incorrect_registrar_should_fail", async () => {
    let [invalidRegistrar] = await createRegistrar(realm, authority, councilMint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), authority);

    await assertThrowsAnchorError('ConstraintSeeds', async () => {
      await VSR_PROGRAM.methods
        .nodeDeposit()
        .accounts({
          registrar: invalidRegistrar,
          voter,
          vault,
          depositToken,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    },
      (anchorErr) => {
        if (anchorErr.error.comparedValues) {
          assert.equal(voter.toString(), anchorErr.error.comparedValues[0].toString())
        }
      },
      false);
  });

  it("with_incorrect_vault_should_fail", async () => {
    const invalidVoter = web3.Keypair.generate();
    const invalidVault = await mintTokenToWallet(mint, authority, invalidVoter.publicKey, new anchor.BN(1e10))

    await assertThrowsAnchorError('ConstraintTokenOwner', async () => {
      await VSR_PROGRAM.methods
        .nodeDeposit()
        .accounts({
          registrar,
          voter,
          vault: invalidVault,
          depositToken,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    },
      (anchorErr) => {
        if (anchorErr.error.comparedValues) {
          assert.equal(invalidVoter.publicKey.toString(), anchorErr.error.comparedValues[0].toString())
        }
      },
      false);
  });

  it("with_incorrect_deposit_token_should_fail", async () => {
    const invalidDepositToken = await mintTokenToWallet(councilMint, authority, voterAuthority.publicKey, new anchor.BN(1e10))
    await assertThrowsAnchorError('ConstraintAssociated', async () => {
      await VSR_PROGRAM.methods
        .nodeDeposit()
        .accounts({
          registrar,
          voter,
          vault,
          depositToken: invalidDepositToken,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    },
      (anchorErr) => {
          assert.equal(depositToken.toString(), anchorErr.error.comparedValues[1].toString())
      }, false);
  });

  it("with_incorrect_deposit_authority_should_fail", async () => {
    const invalidDepositAuthority = await newSigner();
    await assertThrowsAnchorError('ConstraintTokenOwner', async () => {
      await VSR_PROGRAM.methods
        .nodeDeposit()
        .accounts({
          registrar,
          voter,
          vault,
          depositToken,
          depositAuthority: invalidDepositAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([invalidDepositAuthority])
        .rpc();
    },
      (anchorErr) => {
          assert.equal(voterAuthority.publicKey.toString(), anchorErr.error.comparedValues[0].toString())
      },
      false);
  });


  it("with_insufficient_balance_should_fail", async () => {
    // 0x1 represents TokenError::InsufficientFunds
    await assertThrowsSendTransactionError('custom program error: 0x1', async () => {
      await VSR_PROGRAM.methods
        .nodeDeposit()
        .accounts({
          registrar,
          voter,
          vault,
          depositToken,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    },
      (sendTxErr) => { },
      false);
  });

  it("verify_node_deposit_data", async () => {
    const nodeSecurityDeposit = defaultDepositConfig().nodeSecurityDeposit;
    // mint tokens to depositToken account 
    await mintTokenToAccount(mint, authority, depositToken, nodeSecurityDeposit);
    const oldDepositTokenBalance = new anchor.BN((await getTokenAccount(depositToken)).amount.toString()); 

    const nodeDepositEntryIndex = 0;
    const txId = await VSR_PROGRAM.methods
      .nodeDeposit()
      .accounts({
        registrar,
        voter,
        vault,
        depositToken,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc({commitment: "confirmed"});

    const voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    
    const depositEntry = voterData.deposits.at(nodeDepositEntryIndex);
    assert.isTrue(depositEntry.isActive == 1)
    assert.isTrue(depositEntry.amountDepositedNative.eq(nodeSecurityDeposit))
    assert.isTrue(depositEntry.amountInitiallyLockedNative.eq(nodeSecurityDeposit))
    assert.isTrue(depositEntry.lockup.kind.kind.constant != undefined) // assert lockup kind is constant
    assert.equal(depositEntry.lockup.kind.duration.periods.toNumber(), 6) // assert periods of lockup time duration is 6
    assert.isTrue((depositEntry.lockup.kind.duration.unit as any).month != undefined) // assert unit of lockup time duration is 'Month'


    // verify deposit token account
    const newDepositTokenBalance = new anchor.BN((await getTokenAccount(depositToken)).amount.toString()); 
    assert.isTrue(oldDepositTokenBalance.sub(nodeSecurityDeposit).eq(newDepositTokenBalance))

    // verify vault account
    const vaultAccount = await getTokenAccount(vault);
    assert.isTrue(nodeSecurityDeposit.eq(new anchor.BN(vaultAccount.amount.toString())))

    // verify registrar data
    const registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    const tx = await CONNECTION.getTransaction(txId, {commitment: 'confirmed'});
    const txTime = registrarData.timeOffset.add(new anchor.BN(tx.blockTime.toString())).toString();
    assert.equal(registrarData.rewardAccrualTs.toString(), txTime.toString());
    assert.equal(registrarData.permanentlyLockedAmount.toString(), nodeSecurityDeposit.toString());
    assert.equal(registrarData.rewardIndex.toString(), voterData.rewardIndex.toString())
  });

  it("deposit_twice_should_fail", async () => {
    const nodeDepositEntryIndex = 0;
    let voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    let depositEntry = voterData.deposits.at(nodeDepositEntryIndex);
    assert.isTrue(depositEntry.isActive == 1)

    // fastup time
    await fastup(registrar, authority, new anchor.BN(86400));

    const nodeSecurityDeposit = defaultDepositConfig().nodeSecurityDeposit;
    await mintTokenToAccount(mint, authority, depositToken, nodeSecurityDeposit);

    await assertThrowsAnchorError("DuplicateNodeDeposit", async () => {
      await VSR_PROGRAM.methods
        .nodeDeposit()
        .accounts({
          registrar,
          voter,
          vault,
          depositToken,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    }, undefined, false)
  });
});

