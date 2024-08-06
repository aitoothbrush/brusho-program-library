import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, CONNECTION, createRealm, createRegistrar, createVoter, defaultDepositConfig, defaultVotingConfig, delay, fastup, lockupDayily, LockupTimeDuration, lockupTimeDurationSeconds, mintTokenToAccount, newSigner, newTokenAccount, VSR_PROGRAM } from "./helper";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";


describe("withdraw!", () => {
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

  async function deposit(depositEntryIndex: number, duration: LockupTimeDuration, amount: anchor.BN) {
    await mintTokenToAccount(mint, authority, voterTokenAccount, amount)
    await VSR_PROGRAM.methods
      .ordinaryDeposit(depositEntryIndex, amount, duration)
      .accounts({
        registrar,
        voter,
        vault,
        depositToken: voterTokenAccount,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc({commitment: "confirmed"});
  }

  it("with_incorrect_voter_authority_should_fail", async () => {

    const incorrectVoterAuthority = await newSigner();
    await assertThrowsAnchorError('ConstraintSeeds', async () => {
      await VSR_PROGRAM.methods
        .withdraw(1, new anchor.BN(1e9))
        .accounts({
          registrar,
          voter,
          voterAuthority: incorrectVoterAuthority.publicKey,
          tokenOwnerRecord,
          voterWeightRecord,
          vault,
          destination: voterTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([incorrectVoterAuthority])
        .rpc()
    }, undefined, false)
  });

  it("with_incorrect_registrar_should_fail", async () => {
    const [incorrectRegistrar] = await createRegistrar(realm, authority, councilMint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), authority);
    // console.log(`incorrectRegistrar : ${incorrectRegistrar.toBase58()}`)

    await assertThrowsAnchorError('ConstraintSeeds', async () => {
      await VSR_PROGRAM.methods
        .withdraw(1, new anchor.BN(1e9))
        .accounts({
          registrar: incorrectRegistrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
          tokenOwnerRecord,
          voterWeightRecord,
          vault,
          destination: voterTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([voterAuthority])
        .rpc()
    },
      (anchorErr) => {
        if (anchorErr.error.comparedValues) {
          assert.equal(voter.toString(), anchorErr.error.comparedValues[0].toString())
        }
      },
      false);
  });

  it("with_incorrect_voter_weight_record_should_fail", async () => {
    const [_voterAuthority, _voter, incorrectVoterWeightRecord, _vault, _tokenOwnerRecord] = await createVoter(realm, mint, registrar, authority);
    // console.log(`incorrectVoterWeightRecord : ${incorrectVoterWeightRecord.toBase58()}`)

    await assertThrowsAnchorError('ConstraintSeeds', async () => {
      await VSR_PROGRAM.methods
        .withdraw(1, new anchor.BN(1e9))
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
          tokenOwnerRecord,
          voterWeightRecord: incorrectVoterWeightRecord,
          vault,
          destination: voterTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([voterAuthority])
        .rpc()
    },
      (anchorErr) => {
        if (anchorErr.error.comparedValues) {
          assert.equal(voterWeightRecord.toString(), anchorErr.error.comparedValues[1].toString())
        }
      },
      false);
  });

  it("with_incorrect_vault_should_fail", async () => {
    const [_voterAuthority, _voter, _incorrectVoterWeightRecord, incorrectVault, _tokenOwnerRecord] = await createVoter(realm, mint, registrar, authority);
    // console.log(`incorrectVault : ${incorrectVault.toBase58()}`)

    await assertThrowsAnchorError('ConstraintTokenOwner', async () => {
      await VSR_PROGRAM.methods
        .withdraw(1, new anchor.BN(1e9))
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
          tokenOwnerRecord,
          voterWeightRecord,
          vault: incorrectVault,
          destination: voterTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([voterAuthority])
        .rpc()
    },
      (anchorErr) => {
        if (anchorErr.error.comparedValues) {
          assert.equal(voter.toString(), anchorErr.error.comparedValues[1].toString())
        }
      },
      false);
  });

  it("with_incorrect_desctination_should_fail", async () => {
    const incorrectDestination = await newTokenAccount(councilMint, voterAuthority);
    // console.log(`incorrectDestination : ${incorrectDestination.toBase58()}`)

    await assertThrowsAnchorError('ConstraintTokenMint', async () => {
      await VSR_PROGRAM.methods
        .withdraw(1, new anchor.BN(1e9))
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
          tokenOwnerRecord,
          voterWeightRecord,
          vault,
          destination: incorrectDestination,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([voterAuthority])
        .rpc()
    },
      (anchorErr) => { },
      false);
  });

  it("with_incorrect_deposit_entry_index_should_fail", async () => {
    // Make sure that 'vault' has enough funds
    await mintTokenToAccount(mint, authority, vault, new anchor.BN(1e9));

    await assertThrowsAnchorError('OutOfBoundsDepositEntryIndex', async () => {
      await VSR_PROGRAM.methods
        .withdraw(100, new anchor.BN(1e8))
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
          tokenOwnerRecord,
          voterWeightRecord,
          vault,
          destination: voterTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([voterAuthority])
        .rpc()
    },
      (anchorErr) => { },
      false);
  });

  it("with_inactive_deposit_entry_should_fail", async () => {
    // Make sure that 'vault' has enough funds
    await mintTokenToAccount(mint, authority, vault, new anchor.BN(1e9));

    await assertThrowsAnchorError('InactiveDepositEntry', async () => {
      await VSR_PROGRAM.methods
        .withdraw(1, new anchor.BN(1e8))
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
          tokenOwnerRecord,
          voterWeightRecord,
          vault,
          destination: voterTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([voterAuthority])
        .rpc()
    },
      (anchorErr) => { },
      false);
  });

  it("with_insufficient_unlocked_amount_should_fail", async () => {
    const depositEntryIndex = 1;
    const lockupDuration = lockupDayily(15);
    const depositAmount = new anchor.BN(1e8);
    const withdrawAmount = new anchor.BN(1e7);
    await deposit(depositEntryIndex, lockupDuration, depositAmount);

    await assertThrowsAnchorError('InsufficientUnlockedTokens', async () => {
      await VSR_PROGRAM.methods
        .withdraw(depositEntryIndex, withdrawAmount)
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
          tokenOwnerRecord,
          voterWeightRecord,
          vault,
          destination: voterTokenAccount,
          tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([voterAuthority])
        .rpc()
    },
      (anchorErr) => { },
      false);
  });


  it("verify_withdraw_data", async () => {
    const depositEntryIndex = 2;
    const releaseEntryIndex = 3;
    const lockupDuration = lockupDayily(15);
    const depositAmount = new anchor.BN(1e8);
    const withdrawAmount = new anchor.BN(1e7);

    let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    const prevPermanentlyLockedAmount = registrarData.permanentlyLockedAmount;

    // deposit
    await deposit(depositEntryIndex, lockupDuration, depositAmount);
    // release deposit
    await VSR_PROGRAM.methods
      .ordinaryReleaseDeposit(depositEntryIndex, releaseEntryIndex, depositAmount)
      .accounts({
        registrar,
        voter,
        voterAuthority: voterAuthority.publicKey,
      }).signers([voterAuthority])
      .rpc({commitment: "confirmed"});
    // fastup 15 days
    await fastup(registrar, authority, lockupTimeDurationSeconds(lockupDuration), "confirmed");
    
    // withdraw 
    const txId = await VSR_PROGRAM.methods
      .withdraw(releaseEntryIndex, withdrawAmount)
      .accounts({
        registrar,
        voter,
        voterAuthority: voterAuthority.publicKey,
        tokenOwnerRecord,
        voterWeightRecord,
        vault,
        destination: voterTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID
      })
      .signers([voterAuthority])
      .rpc({commitment: "confirmed"})
    const tx = await CONNECTION.getTransaction(txId, {commitment: 'confirmed'});

    // verify voter account
    let voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    let releaseEntry = voterData.deposits.at(releaseEntryIndex);
    assert.isTrue(releaseEntry.isActive)
    assert.isTrue(releaseEntry.amountDepositedNative.eq(depositAmount.sub(withdrawAmount)))
    assert.isTrue(releaseEntry.amountInitiallyLockedNative.eq(depositAmount))

    // verify registrar data
    registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);

    const txTime = registrarData.timeOffset.add(new anchor.BN(tx.blockTime.toString()));
    assert.equal(registrarData.rewardAccrualTs.toString(), txTime.toString());
    assert.equal(registrarData.permanentlyLockedAmount.toString(), prevPermanentlyLockedAmount.toString());
    assert.equal(registrarData.rewardIndex.v.toString(), voterData.rewardIndex.v.toString())

    // withdraw remains, deposit entry should have been deactivated.
    await VSR_PROGRAM.methods
      .withdraw(releaseEntryIndex, depositAmount.sub(withdrawAmount))
      .accounts({
        registrar,
        voter,
        voterAuthority: voterAuthority.publicKey,
        tokenOwnerRecord,
        voterWeightRecord,
        vault,
        destination: voterTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID
      })
      .signers([voterAuthority])
      .rpc({commitment: "confirmed"})

    // verify voter account
    voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    releaseEntry = voterData.deposits.at(releaseEntryIndex);
    assert.isFalse(releaseEntry.isActive)
  });
});


