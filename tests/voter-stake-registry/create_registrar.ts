import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, CIRCUIT_BREAKER_PROGRAM, CONNECTION, createRealm, defaultDepositConfig, defaultVotingConfig, DepositConfig, EXP_SCALE, getTokenAccount, GOV_PROGRAM_ID, lockupDayily, lockupMonthly, newMint, newSigner, SECS_PER_DAY, SECS_PER_YEAR, TOTAL_REWARD_AMOUNT, VotingConfig, VSR_PROGRAM } from "./helper";
import { assert } from "chai";
import { getAccount, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { getMaxVoterWeightRecord, getVoterWeightRecord } from "@solana/spl-governance";

async function createRegistrar(
  bump: number,
  registrar: web3.PublicKey,
  vault: web3.PublicKey,
  circuitBreaker: web3.PublicKey,
  maxVoterWeightRecord: web3.PublicKey,
  realm: web3.PublicKey,
  mint: web3.PublicKey,
  realmAuthority: web3.Keypair,
  payer: web3.Keypair,
  votingConfig?: VotingConfig,
  depositConfig?: DepositConfig,
  circuit_breaker_threshold?: anchor.BN,
) {
  if (votingConfig == undefined) {
    votingConfig = defaultVotingConfig();
  }

  if (depositConfig == undefined) {
    depositConfig = defaultDepositConfig();
  }

  if (circuit_breaker_threshold == undefined) {
    circuit_breaker_threshold = new anchor.BN(1e10);
  }

  return await VSR_PROGRAM.methods.createRegistrar(
    bump,
    votingConfig,
    depositConfig,
    circuit_breaker_threshold
  ).accounts({
    registrar,
    vault,
    circuitBreaker,
    maxVoterWeightRecord,
    realm: realm,
    governanceProgramId: GOV_PROGRAM_ID,
    circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId,
    realmGoverningTokenMint: mint,
    realmAuthority: realmAuthority.publicKey,
    payer: payer.publicKey,
  }).signers([payer, realmAuthority])
    .rpc({ commitment: "confirmed" })
}

describe("create_registrar!", () => {
  // describe("PDA Verification", () => {
  //   it("with_incorrect_governing_token_mint_should_fail", async () => {
  //     const realmAuthority = await newSigner();
  //     let [mint, councilMint, realm] = await createRealm(realmAuthority);

  //     const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
  //     const [registrar, bump] = anchor.web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

  //     const vault = getAssociatedTokenAddressSync(mint, registrar, true);
  //     const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
  //     const [circuitBreaker, circuitBreakerBump] = anchor.web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

  //     const maxVoterWeightRecordSeeds = [realm.toBytes(), Buffer.from("max-voter-weight-record"), mint.toBytes()];
  //     const [maxVoterWeightRecord, maxVoterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(maxVoterWeightRecordSeeds, VSR_PROGRAM.programId);

  //     await assertThrowsAnchorError('ConstraintSeeds', async () => {
  //       // use councilMint 
  //       await createRegistrar(bump, registrar, vault, circuitBreaker, maxVoterWeightRecord, realm, councilMint, realmAuthority, realmAuthority);
  //     })
  //   });

  //   it("with_incorrect_bump_should_fail", async () => {
  //     const realmAuthority = await newSigner();
  //     let [mint, councilMint, realm] = await createRealm(realmAuthority);

  //     const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
  //     const [registrar, bump] = anchor.web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

  //     const vault = getAssociatedTokenAddressSync(mint, registrar, true);
  //     const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
  //     const [circuitBreaker, circuitBreakerBump] = anchor.web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

  //     const maxVoterWeightRecordSeeds = [realm.toBytes(), Buffer.from("max-voter-weight-record"), mint.toBytes()];
  //     const [maxVoterWeightRecord, maxVoterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(maxVoterWeightRecordSeeds, VSR_PROGRAM.programId);

  //     await assertThrowsAnchorError('RequireEqViolated', async () => {
  //       await createRegistrar(bump - 1, registrar, vault, circuitBreaker, maxVoterWeightRecord, realm, mint, realmAuthority, realmAuthority);
  //     });
  //   });
  // });

  // describe("Args verification", () => {
  //   it("with_zero_lockup_saturation_secs_should_fail", async () => {
  //     const realmAuthority = await newSigner();
  //     let [mint, councilMint, realm] = await createRealm(realmAuthority);

  //     const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
  //     const [registrar, bump] = anchor.web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

  //     const vault = getAssociatedTokenAddressSync(mint, registrar, true);
  //     const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
  //     const [circuitBreaker, circuitBreakerBump] = anchor.web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

  //     const maxVoterWeightRecordSeeds = [realm.toBytes(), Buffer.from("max-voter-weight-record"), mint.toBytes()];
  //     const [maxVoterWeightRecord, maxVoterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(maxVoterWeightRecordSeeds, VSR_PROGRAM.programId);

  //     const votingConfig = {
  //       baselineVoteWeightScaledFactor: new anchor.BN(1e9),
  //       maxExtraLockupVoteWeightScaledFactor: new anchor.BN(0),
  //       lockupSaturationSecs: new anchor.BN(0), // zero value
  //     };

  //     await assertThrowsAnchorError('LockupSaturationMustBePositive', async () => {
  //       await createRegistrar(bump, registrar, vault, circuitBreaker, maxVoterWeightRecord, realm, mint, realmAuthority, realmAuthority, votingConfig);
  //     });
  //   });

  //   it("with_zero_node_security_deposit_should_fail", async () => {
  //     const realmAuthority = await newSigner();
  //     let [mint, councilMint, realm] = await createRealm(realmAuthority);

  //     const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
  //     const [registrar, bump] = anchor.web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

  //     const vault = getAssociatedTokenAddressSync(mint, registrar, true);
  //     const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
  //     const [circuitBreaker, circuitBreakerBump] = anchor.web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

  //     const maxVoterWeightRecordSeeds = [realm.toBytes(), Buffer.from("max-voter-weight-record"), mint.toBytes()];
  //     const [maxVoterWeightRecord, maxVoterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(maxVoterWeightRecordSeeds, VSR_PROGRAM.programId);

  //     const depositConfig = {
  //       ordinaryDepositMinLockupDuration: lockupDayily(15),
  //       nodeDepositLockupDuration: lockupMonthly(6),
  //       nodeSecurityDeposit: new anchor.BN(0), // zero value
  //     };

  //     await assertThrowsAnchorError('NodeSecurityDepositMustBePositive', async () => {
  //       await createRegistrar(bump, registrar, vault, circuitBreaker, maxVoterWeightRecord, realm, mint, realmAuthority, realmAuthority, undefined, depositConfig);
  //     });
  //   });
  // });

  // describe("Realm verification", () => {
  //   it("with_incorrect_realm_authority_should_fail", async () => {
  //     const realmAuthority = await newSigner();
  //     let [mint, councilMint, realm] = await createRealm(realmAuthority);

  //     const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
  //     const [registrar, bump] = web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

  //     const vault = getAssociatedTokenAddressSync(mint, registrar, true);
  //     const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
  //     const [circuitBreaker, circuitBreakerBump] = anchor.web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

  //     const maxVoterWeightRecordSeeds = [realm.toBytes(), Buffer.from("max-voter-weight-record"), mint.toBytes()];
  //     const [maxVoterWeightRecord, maxVoterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(maxVoterWeightRecordSeeds, VSR_PROGRAM.programId);

  //     const invalidRealmAuthority = await newSigner();
  //     await assertThrowsAnchorError('InvalidRealmAuthority', async () => {
  //       await createRegistrar(bump, registrar, vault, circuitBreaker, maxVoterWeightRecord, realm, mint, invalidRealmAuthority, realmAuthority);
  //     })
  //   });

  // });

  it("verify_registrar_data", async () => {
    const realmAuthority = await newSigner();
    let [mint, councilMint, realm] = await createRealm(realmAuthority);

    const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
    const [registrar, bump] = web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

    const vault = getAssociatedTokenAddressSync(mint, registrar, true);
    const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
    const [circuitBreaker, circuitBreakerBump] = anchor.web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

    const maxVoterWeightRecordSeeds = [realm.toBytes(), Buffer.from("max-voter-weight-record"), mint.toBytes()];
    const [maxVoterWeightRecord, maxVoterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(maxVoterWeightRecordSeeds, VSR_PROGRAM.programId);

    const votingConfig = defaultVotingConfig();
    const depositConfig = {
      ordinaryDepositMinLockupDuration: lockupDayily(15),
      nodeDepositLockupDuration: lockupMonthly(6),
      nodeSecurityDeposit: new anchor.BN(10000 * (1e6)),
    };

    const circuitBreakerThreshold = new anchor.BN(1e9);
    const txId = await createRegistrar(bump, registrar, vault, circuitBreaker, maxVoterWeightRecord, realm, mint, realmAuthority, realmAuthority, votingConfig, depositConfig, circuitBreakerThreshold);
    const tx = await CONNECTION.getTransaction(txId, { commitment: 'confirmed' })

    // assert vault has been initialized
    assert.isTrue(await CONNECTION.getAccountInfo(vault) != null);
    const vaultAccount = await getTokenAccount(vault);
    assert.equal(vaultAccount.owner.toBase58(), circuitBreaker.toBase58())
    assert.equal(vaultAccount.mint.toBase58(), mint.toBase58())
    // assert circuitBreaker has been initialized
    assert.isTrue(await CONNECTION.getAccountInfo(circuitBreaker) != null);

    // verify registrar data
    const registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    assert.equal(registrarData.realm.toBase58(), realm.toBase58())
    assert.equal(registrarData.realmAuthority.toBase58(), realmAuthority.publicKey.toBase58())
    assert.equal(registrarData.governanceProgramId.toBase58(), GOV_PROGRAM_ID.toBase58())
    assert.equal(registrarData.governingTokenMint.toBase58(), mint.toBase58())
    assert.equal(registrarData.bump, bump)
    assert.equal(registrarData.votingConfig.baselineVoteWeightScaledFactor.toNumber(), votingConfig.baselineVoteWeightScaledFactor.toNumber())
    assert.equal(registrarData.votingConfig.maxExtraLockupVoteWeightScaledFactor.toNumber(), votingConfig.maxExtraLockupVoteWeightScaledFactor.toNumber())
    assert.equal(registrarData.votingConfig.lockupSaturationSecs.toNumber(), votingConfig.lockupSaturationSecs.toNumber())
    assert.equal(registrarData.depositConfig.ordinaryDepositMinLockupDuration.periods.toNumber(), depositConfig.ordinaryDepositMinLockupDuration.periods.toNumber())
    assert.isTrue(registrarData.depositConfig.ordinaryDepositMinLockupDuration.unit.day != undefined)
    assert.equal(registrarData.depositConfig.nodeDepositLockupDuration.periods.toNumber(), depositConfig.nodeDepositLockupDuration.periods.toNumber())
    assert.isTrue(registrarData.depositConfig.nodeDepositLockupDuration.unit.month != undefined)
    assert.equal(registrarData.depositConfig.nodeSecurityDeposit.toNumber(), depositConfig.nodeSecurityDeposit.toNumber())

    const expectCurrentRewardAmountPerSecond = TOTAL_REWARD_AMOUNT.muln(12).divn(100).mul(EXP_SCALE).div(SECS_PER_YEAR);
    assert.equal(registrarData.currentRewardAmountPerSecond.toString(), expectCurrentRewardAmountPerSecond.toString());
    assert.equal(registrarData.lastRewardAmountPerSecondRotatedTs.toString(), tx.blockTime.toString());
    assert.equal(registrarData.rewardAccrualTs.toString(), tx.blockTime.toString());
    assert.isTrue(registrarData.rewardIndex.eqn(0));
    assert.isTrue(registrarData.issuedRewardAmount.eqn(0));
    assert.isTrue(registrarData.permanentlyLockedAmount.eqn(0));

    // verify cirruitBreaker data
    const circuitBreakerData = await CIRCUIT_BREAKER_PROGRAM.account.accountWindowedCircuitBreakerV0.fetch(circuitBreaker);
    assert.equal(circuitBreakerData.tokenAccount.toBase58(), vault.toBase58());
    assert.equal(circuitBreakerData.authority.toBase58(), realmAuthority.publicKey.toBase58());
    assert.equal(circuitBreakerData.owner.toBase58(), registrar.toBase58());
    assert.equal(circuitBreakerData.config.windowSizeSeconds.toNumber(), SECS_PER_DAY.toNumber());
    assert.isTrue(circuitBreakerData.config.thresholdType.absolute != undefined);
    assert.equal(circuitBreakerData.config.threshold.toNumber(), circuitBreakerThreshold.toNumber());

    // verify MaxVoterWeightRecord
    const maxVoterWeightRecordData = await getMaxVoterWeightRecord(anchor.getProvider().connection, maxVoterWeightRecord);
    assert.equal(maxVoterWeightRecordData.owner.toBase58(), VSR_PROGRAM.programId.toBase58());
    assert.equal(maxVoterWeightRecordData.account.realm.toBase58(), realm.toBase58());
    assert.equal(maxVoterWeightRecordData.account.governingTokenMint.toBase58(), mint.toBase58());
  });

});


