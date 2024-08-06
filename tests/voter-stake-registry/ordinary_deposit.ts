import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { mintTokenToWallet, assertThrowsAnchorError, assertThrowsSendTransactionError, createRealm, newSigner, VSR_PROGRAM, getTokenAccount, fastup, lockupDayily, lockupMonthly, defaultDepositConfig, createRegistrar, defaultVotingConfig, newTokenAccount, mintTokenToAccount, createVoter, CONNECTION } from "./helper";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

describe("ordinary_deposit!", () => {
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
  let voterTokenAccount: web3.PublicKey;

  before(async () => {
    authority = await newSigner();
    [mint, councilMint, realm] = await createRealm(authority);
    // create registrar
    [registrar] = await createRegistrar(realm, authority, mint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), authority);
    [voterAuthority, voter, voterWeightRecord, vault, tokenOwnerRecord] = await createVoter(realm, mint, registrar, authority);
    voterTokenAccount = await newTokenAccount(mint, voterAuthority);

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
    // console.log(`voterTokenAccount: ${voterTokenAccount.toBase58()}`)
  })

  it("with_incorrect_registrar_should_fail", async () => {
    let [invalidRegistrar] = await createRegistrar(realm, authority, councilMint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), authority);

    await assertThrowsAnchorError('ConstraintSeeds', async () => {
      await VSR_PROGRAM.methods
        .ordinaryDeposit(1, new anchor.BN(1e9), lockupDayily(15))
        .accounts({
          registrar: invalidRegistrar,
          voter,
          vault,
          depositToken: voterTokenAccount,
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
        .ordinaryDeposit(1, new anchor.BN(1e9), lockupDayily(15))
        .accounts({
          registrar,
          voter,
          vault: invalidVault,
          depositToken: voterTokenAccount,
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
        .ordinaryDeposit(1, new anchor.BN(1e9), lockupDayily(15))
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
        assert.equal(voterTokenAccount.toString(), anchorErr.error.comparedValues[1].toString())
      }, false);
  });

  it("with_incorrect_deposit_authority_should_fail", async () => {
    const invalidDepositAuthority = await newSigner();
    await assertThrowsAnchorError('ConstraintTokenOwner', async () => {
      await VSR_PROGRAM.methods
        .ordinaryDeposit(1, new anchor.BN(1e9), lockupDayily(15))
        .accounts({
          registrar,
          voter,
          vault,
          depositToken: voterTokenAccount,
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
    const balance = new anchor.BN(1e10);
    await mintTokenToAccount(mint, authority, voterTokenAccount, balance)

    const depositAmount = balance.add(new anchor.BN(1e9)); // > balance
    // 0x1 represents TokenError::InsufficientFunds
    await assertThrowsSendTransactionError('custom program error: 0x1', async () => {
      await VSR_PROGRAM.methods
        .ordinaryDeposit(1, depositAmount, lockupDayily(15))
        .accounts({
          registrar,
          voter,
          vault,
          depositToken: voterTokenAccount,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    },
      (sendTxErr) => { },
      false);
  });


  it("with_incorrect_args_should_fail", async () => {
    await assertThrowsAnchorError('NodeDepositReservedEntryIndex', async () => {
      await VSR_PROGRAM.methods
        .ordinaryDeposit(0, new anchor.BN(1e9), lockupDayily(15)) // index 0 is reserved for node deposit
        .accounts({
          registrar,
          voter,
          vault,
          depositToken: voterTokenAccount,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    },
      (anchorErr) => { },
      false);

    await assertThrowsAnchorError('InvalidLockupDuration', async () => {
      await VSR_PROGRAM.methods
        .ordinaryDeposit(1, new anchor.BN(1e9), lockupDayily(14)) // lockup duration is short than min duration
        .accounts({
          registrar,
          voter,
          vault,
          depositToken: voterTokenAccount,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    },
      (anchorErr) => { },
      false);
  });

  it("verify_ordinary_deposit_data", async () => {
    const depositAmount = new anchor.BN(500e6); // 500
    await mintTokenToAccount(mint, authority, voterTokenAccount, depositAmount);
    const oldVoterTokenAccountBalance = new anchor.BN((await getTokenAccount(voterTokenAccount)).amount.toString()); 

    const depositEntryIndex = 1;
    const txId = await VSR_PROGRAM.methods
      .ordinaryDeposit(depositEntryIndex, depositAmount, lockupDayily(15))
      .accounts({
        registrar,
        voter,
        vault,
        depositToken: voterTokenAccount,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc({commitment: "confirmed"});

    const voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    const depositEntry = voterData.deposits.at(depositEntryIndex);
    assert.isTrue(depositEntry.isActive)
    assert.isTrue(depositEntry.amountDepositedNative.eq(depositAmount))
    assert.isTrue(depositEntry.amountInitiallyLockedNative.eq(depositAmount))
    assert.isTrue(depositEntry.lockup.kind.constant != undefined) // assert lockup kind is constant
    assert.equal(depositEntry.lockup.kind.constant![0].periods, 15) // assert periods of lockup time duration is 15
    assert.isTrue((depositEntry.lockup.kind.constant![0].unit as any).day != undefined) // assert unit of lockup time duration is Day

    // verify deposit token account
    const newVoterTokenAccountBalance = new anchor.BN((await getTokenAccount(voterTokenAccount)).amount.toString()); 
    assert.isTrue(oldVoterTokenAccountBalance.sub(depositAmount).eq(newVoterTokenAccountBalance))

    // verify vault account
    const vaultAccount = await getTokenAccount(vault);
    assert.isTrue(depositAmount.eq(new anchor.BN(vaultAccount.amount.toString())))

    // verify registrar data
    const registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    const tx = await CONNECTION.getTransaction(txId, {commitment: 'confirmed'});
    const txTime = registrarData.timeOffset.add(new anchor.BN(tx.blockTime.toString()));
    assert.equal(registrarData.rewardAccrualTs.toString(), txTime.toString());
    assert.equal(registrarData.permanentlyLockedAmount.toString(), depositAmount.toString());
    assert.equal(registrarData.rewardIndex.v.toString(), voterData.rewardIndex.v.toString())
  });

  it("deposit_multi_times_should_work", async () => {
    await mintTokenToAccount(mint, authority, voterTokenAccount, new anchor.BN(10000e6)); // 10000 token
    // const oldVoterTokenAccountBalance = new anchor.BN((await getTokenAccount(voterTokenAccount)).amount.toString()); 

    const depositEntryIndex = 2;
    const depositAmount = new anchor.BN(500e6); // 500

    // first time
    await VSR_PROGRAM.methods
      .ordinaryDeposit(depositEntryIndex, depositAmount, lockupDayily(16))
      .accounts({
        registrar,
        voter,
        vault,
        depositToken: voterTokenAccount,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc();

    let voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    let depositEntry = voterData.deposits.at(depositEntryIndex);
    const lockupStartTs = depositEntry.lockup.startTs;

    // fastup time
    await fastup(registrar, authority, new anchor.BN(86400));

    // sencond time
    await VSR_PROGRAM.methods
      .ordinaryDeposit(depositEntryIndex, depositAmount, lockupDayily(16)) // keep lockup duration unchanged
      .accounts({
        registrar,
        voter,
        vault,
        depositToken: voterTokenAccount,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc();

    let totalDepositAmount = depositAmount.muln(2);
    voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    depositEntry = voterData.deposits.at(depositEntryIndex);
    let newlockupStartTs = depositEntry.lockup.startTs;

    assert.isTrue(depositEntry.isActive)
    assert.isTrue(depositEntry.amountDepositedNative.eq(totalDepositAmount))
    assert.isTrue(depositEntry.amountInitiallyLockedNative.eq(totalDepositAmount))
    assert.isTrue(depositEntry.lockup.kind.constant != undefined) // assert lockup kind is constant
    assert.equal(depositEntry.lockup.kind.constant![0].periods, 16) // assert periods of lockup time duration is 6
    assert.isTrue((depositEntry.lockup.kind.constant![0].unit as any).day != undefined) // assert unit of lockup time duration is Month
    assert.isTrue(newlockupStartTs.gte(lockupStartTs)) // The start ts of lockup moves

    // fastup time
    await fastup(registrar, authority, new anchor.BN(86400));

    // third time, 0 amount
    await VSR_PROGRAM.methods
      .ordinaryDeposit(depositEntryIndex, new anchor.BN(0), lockupMonthly(6)) // change lockup duration to 6 month
      .accounts({
        registrar,
        voter,
        vault,
        depositToken: voterTokenAccount,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc();

    voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    depositEntry = voterData.deposits.at(depositEntryIndex);
    newlockupStartTs = depositEntry.lockup.startTs;

    assert.isTrue(depositEntry.isActive)
    assert.isTrue(depositEntry.amountDepositedNative.eq(totalDepositAmount))
    assert.isTrue(depositEntry.amountInitiallyLockedNative.eq(totalDepositAmount))
    assert.isTrue(depositEntry.lockup.kind.constant != undefined) // assert lockup kind is constant
    assert.equal(depositEntry.lockup.kind.constant![0].periods, 6) // assert periods of lockup time duration is 6
    assert.isTrue((depositEntry.lockup.kind.constant![0].unit as any).month != undefined) // assert unit of lockup time duration is Month
    assert.isTrue(newlockupStartTs.gte(lockupStartTs)) // The start ts of lockup moves

    await assertThrowsAnchorError('CanNotShortenLockupDuration', async () => {
      await VSR_PROGRAM.methods
        .ordinaryDeposit(depositEntryIndex, new anchor.BN(1e9), lockupDayily(16)) // shorten lockup periods 
        .accounts({
          registrar,
          voter,
          vault,
          depositToken: voterTokenAccount,
          depositAuthority: voterAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID
        }).signers([voterAuthority])
        .rpc();
    },
      (anchorErr) => { },
      false);
  });
});
