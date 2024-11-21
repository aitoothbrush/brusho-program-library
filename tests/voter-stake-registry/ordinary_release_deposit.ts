import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, createRealm, newSigner, VSR_PROGRAM, lockupDayily, LockupTimeDuration, newTokenAccount, mintTokenToAccount, createRegistrar, createVoter, defaultDepositConfig, defaultVotingConfig, CONNECTION } from "../helper";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";


describe("ordinary_release_deposit!", () => {
  const depositAmount = new anchor.BN(1e9); // 1000 tokens

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
    // console.log(`depositToken: ${depositToken.toBase58()}`)
  })

  async function deposit(depositEntryIndex: number, duration: LockupTimeDuration) {
    await mintTokenToAccount(mint, authority, depositToken, depositAmount)
    await VSR_PROGRAM.methods
      .ordinaryDeposit(depositEntryIndex, depositAmount, duration)
      .accounts({
        registrar,
        voter,
        vault,
        depositToken,
        depositAuthority: voterAuthority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([voterAuthority])
      .rpc();
  }

  it("with_incorrect_registrar_should_fail", async () => {
    let [invalidRegistrar] = await createRegistrar(realm, authority, councilMint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), authority);

    await assertThrowsAnchorError('ConstraintSeeds', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(1, 2, new anchor.BN(1e9))
        .accounts({
          registrar: invalidRegistrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
        }).signers([voterAuthority])
        .rpc();
    },
      (anchorErr) => {
        if (anchorErr.error.comparedValues) {
          assert.equal(voter.toString(), anchorErr.error.comparedValues[0].toString())
        }
      },
    );
  });

  it("with_incorrect_voter_authority_should_fail", async () => {
    const invalidVoterAuthority = await newSigner();

    await assertThrowsAnchorError('ConstraintSeeds', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(1, 2, new anchor.BN(1e9))
        .accounts({
          registrar,
          voter,
          voterAuthority: invalidVoterAuthority.publicKey,
        }).signers([invalidVoterAuthority])
        .rpc();
    },
      (anchorErr) => {
        if (anchorErr.error.comparedValues) {
          assert.equal(voter.toString(), anchorErr.error.comparedValues[0].toString())
        }
      },
    );
  });

  it("with_incorrect_args_should_fail", async () => {
    await assertThrowsAnchorError('ZeroAmount', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(1, 2, new anchor.BN(0)) // zero amount
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
        }).signers([voterAuthority])
        .rpc();
    },
      undefined,
      false
    );

    await assertThrowsAnchorError('NodeDepositReservedEntryIndex', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(0, 1, new anchor.BN(1e7)) // deposit entry index 0 is reserved for node deposit 
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
        }).signers([voterAuthority])
        .rpc();
    },
      undefined,
      false
    );

    await assertThrowsAnchorError('NodeDepositReservedEntryIndex', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(1, 0, new anchor.BN(1e7)) // deposit entry index 0 is reserved for node deposit 
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
        }).signers([voterAuthority])
        .rpc();
    },
      undefined,
      false
    );
  });

  it("with_inactive_deposit_entry_index_should_fail", async () => {
    await assertThrowsAnchorError('InactiveDepositEntry', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(1, 1, new anchor.BN(1e7)) // zero amount
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
        }).signers([voterAuthority])
        .rpc();
    },
      undefined,
      false
    );
  });

  it("with_insufficient_deposit_amount_should_fail", async () => {
    await deposit(1, lockupDayily(15));

    await assertThrowsAnchorError('InsufficientLockedTokens', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(1, 2, depositAmount.addn(1e6))
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
        }).signers([voterAuthority])
        .rpc();
    },
      undefined,
      false
    );
  });

  it("with_active_target_entry_index_should_fail", async () => {
    const targetDepositEntryIndex = 2;
    await deposit(targetDepositEntryIndex, lockupDayily(15));

    await assertThrowsAnchorError('ActiveDepositEntryIndex', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(1, 2, depositAmount)
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
        }).signers([voterAuthority])
        .rpc();
    },
      undefined,
      false
    );
  });

  it("verify_ordinary_release_deposit", async () => {
    const depositEntryIndex = 3;
    const targetDepositEntryIndex = 4;
    await deposit(depositEntryIndex, lockupDayily(15));

    let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    const prevPermanentlyLockedAmount = registrarData.permanentlyLockedAmount;

    const releaseAmount = depositAmount.divn(2);
    const txId = await VSR_PROGRAM.methods
      .ordinaryReleaseDeposit(depositEntryIndex, targetDepositEntryIndex, releaseAmount)
      .accounts({
        registrar,
        voter,
        voterAuthority: voterAuthority.publicKey,
      }).signers([voterAuthority])
      .rpc({commitment: "confirmed"});

    const voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    const originDepositEntry = voterData.deposits.at(depositEntryIndex);
    // console.log(JSON.stringify(originDepositEntry, undefined, 2))
    assert.isTrue(originDepositEntry.isActive == 1)
    assert.isTrue(originDepositEntry.amountDepositedNative.eq(depositAmount.sub(releaseAmount)))
    assert.isTrue(originDepositEntry.amountInitiallyLockedNative.eq(depositAmount.sub(releaseAmount)))
    // assert lockup kind remains unchanged 
    assert.isTrue(originDepositEntry.lockup.kind.duration != undefined) // assert lockup kind is constant
    assert.equal(originDepositEntry.lockup.kind.duration.periods.toNumber(), 15) // assert periods of lockup time duration is 15
    assert.isTrue((originDepositEntry.lockup.kind.duration.unit as any).day != undefined) // assert unit of lockup time duration is Day

    const targetDepositEntry = voterData.deposits.at(targetDepositEntryIndex);
    // console.log(JSON.stringify(targetDepositEntry, undefined, 2))
    assert.isTrue(targetDepositEntry.isActive == 1)
    assert.isTrue(targetDepositEntry.amountDepositedNative.eq(releaseAmount))
    assert.isTrue(targetDepositEntry.amountInitiallyLockedNative.eq(releaseAmount))
    assert.isTrue(targetDepositEntry.lockup.kind.kind.daily != undefined) // assert lockup kind is daily
    assert.equal(targetDepositEntry.lockup.kind.duration.periods.toNumber(), 15) // assert periods of lockup time duration is 15

    // verify registrar data
    registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    const tx = await CONNECTION.getTransaction(txId, {commitment: 'confirmed'});
    const txTime = registrarData.timeOffset.add(new anchor.BN(tx.blockTime.toString()));
    assert.equal(registrarData.rewardAccrualTs.toString(), txTime.toString());
    assert.equal(registrarData.permanentlyLockedAmount.toString(), prevPermanentlyLockedAmount.sub(releaseAmount).toString());
    assert.equal(registrarData.rewardIndex.toString(), voterData.rewardIndex.toString())
  });

  it("verify_ordinary_release_deposit_all", async () => {
    const depositEntryIndex = 5;
    await deposit(depositEntryIndex, lockupDayily(15));

    await VSR_PROGRAM.methods
      .ordinaryReleaseDeposit(depositEntryIndex, depositEntryIndex, depositAmount)
      .accounts({
        registrar,
        voter,
        voterAuthority: voterAuthority.publicKey,
      }).signers([voterAuthority])
      .rpc();

    const voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    const depositEntry = voterData.deposits.at(depositEntryIndex);
    // console.log(JSON.stringify(originDepositEntry, undefined, 2))
    assert.isTrue(depositEntry.isActive == 1)
    assert.isTrue(depositEntry.amountDepositedNative.eq(depositAmount))
    assert.isTrue(depositEntry.amountInitiallyLockedNative.eq(depositAmount))
    // assert lockup kind remains unchanged 
    assert.isTrue(depositEntry.lockup.kind.kind.daily != undefined) // assert lockup kind is daily
    assert.equal(depositEntry.lockup.kind.duration.periods.toNumber(), 15) // assert periods of lockup time duration is 15
  });

  it("with_non_constant_deposit_entry_should_fail", async () => {
    const depositEntryIndex = 5; // After previous test, The lockup kind of deposit entry at index 5 is 'daily' now.

    await assertThrowsAnchorError('NotOrdinaryDepositEntry', async () => {
      await VSR_PROGRAM.methods
        .ordinaryReleaseDeposit(depositEntryIndex, 6, depositAmount)
        .accounts({
          registrar,
          voter,
          voterAuthority: voterAuthority.publicKey,
        }).signers([voterAuthority])
        .rpc();
    },
      undefined,
      false
    );

  });
});
