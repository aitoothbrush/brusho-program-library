import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, createRealm, defaultDepositConfig, defaultVotingConfig, DepositConfig, GOV_PROGRAM_ID, lockupDayily, lockupMonthly, newMint, newSigner, VotingConfig, VSR_PROGRAM } from "./helper";
import { assert } from "chai";

async function createRegistrar(
  bump: number,
  registrar: web3.PublicKey,
  realm: web3.PublicKey,
  mint: web3.PublicKey,
  realmAuthority: web3.Keypair,
  payer: web3.Keypair,
  votingConfig?: VotingConfig,
  depositConfig?: DepositConfig,
) {
  if (votingConfig == undefined) {
    votingConfig = defaultVotingConfig();
  }

  if (depositConfig == undefined) {
    depositConfig = defaultDepositConfig();
  }

  await VSR_PROGRAM.methods.createRegistrar(
    bump,
    votingConfig,
    depositConfig
  ).accounts({
    registrar,
    realm: realm,
    governanceProgramId: GOV_PROGRAM_ID,
    realmGoverningTokenMint: mint,
    realmAuthority: realmAuthority.publicKey,
    payer: payer.publicKey,
  }).signers([payer, realmAuthority])
    .rpc()
}

describe("create_registrar!", () => {
  describe("PDA Verification", () => {
    it("with_incorrect_governing_token_mint_should_fail", async () => {
      const realmAuthority = await newSigner();
      let [mint, councilMint, realm] = await createRealm(realmAuthority);

      const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
      const [registrar, bump] = anchor.web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

      await assertThrowsAnchorError('ConstraintSeeds', async () => {
        // use councilMint 
        await createRegistrar(bump, registrar, realm, councilMint, realmAuthority, realmAuthority);
      })
    });

    it("with_incorrect_bump_should_fail", async () => {
      const realmAuthority = await newSigner();
      let [mint, councilMint, realm] = await createRealm(realmAuthority);

      const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
      const [registrar, bump] = anchor.web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

      await assertThrowsAnchorError('RequireEqViolated', async () => {
        await createRegistrar(bump - 1, registrar, realm, mint, realmAuthority, realmAuthority);
      });
    });
  });

  describe("Args verification", () => {
    it("with_zero_lockup_saturation_secs_should_fail", async () => {
      const realmAuthority = await newSigner();
      let [mint, councilMint, realm] = await createRealm(realmAuthority);

      const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
      const [registrar, bump] = anchor.web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

      const votingConfig = {
        baselineVoteWeightScaledFactor: new anchor.BN(1e9),
        maxExtraLockupVoteWeightScaledFactor: new anchor.BN(0),
        lockupSaturationSecs: new anchor.BN(0), // zero value
      };

      await assertThrowsAnchorError('LockupSaturationMustBePositive', async () => {
        await createRegistrar(bump, registrar, realm, mint, realmAuthority, realmAuthority, votingConfig);
      });
    });

    it("with_zero_node_security_deposit_should_fail", async () => {
      const realmAuthority = await newSigner();
      let [mint, councilMint, realm] = await createRealm(realmAuthority);

      const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
      const [registrar, bump] = anchor.web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

      const depositConfig = {
        ordinaryDepositMinLockupDuration: lockupDayily(15),
        nodeDepositLockupDuration: lockupMonthly(6),
        nodeSecurityDeposit: new anchor.BN(0), // zero value
      };

      await assertThrowsAnchorError('NodeSecurityDepositMustBePositive', async () => {
        await createRegistrar(bump, registrar, realm, mint, realmAuthority, realmAuthority, undefined, depositConfig);
      });
    });
  });

  describe("Realm verification", () => {
    it("with_incorrect_governing_mint_should_fail", async () => {
      const realmAuthority = await newSigner();
      let [mint, councilMint, realm] = await createRealm(realmAuthority);

      let invalidMint = await newMint(realmAuthority);

      const seeds = [realm.toBytes(), Buffer.from("registrar"), invalidMint.toBytes()];
      const [registrar, bump] = web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

      try {
        await createRegistrar(bump, registrar, realm, invalidMint, realmAuthority, realmAuthority);
      } catch (e) {
        assert.isTrue(e instanceof web3.SendTransactionError);
        assert.strictEqual((e as web3.SendTransactionError).transactionError.message, 'Transaction simulation failed: Error processing Instruction 0: custom program error: 0x1f7')
      }
    });

    it("with_incorrect_realm_authority_should_fail", async () => {
      const realmAuthority = await newSigner();
      let [mint, councilMint, realm] = await createRealm(realmAuthority);

      const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
      const [registrar, bump] = web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

      const invalidRealmAuthority = await newSigner();
      await assertThrowsAnchorError('InvalidRealmAuthority', async () => {
        await createRegistrar(bump, registrar, realm, mint, invalidRealmAuthority, realmAuthority);
      })
    });

  });

  it("verify_registrar_data", async () => {
    const realmAuthority = await newSigner();
    let [mint, councilMint, realm] = await createRealm(realmAuthority);

    const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
    const [registrar, bump] = web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

    const votingConfig = defaultVotingConfig();
    const depositConfig = {
      ordinaryDepositMinLockupDuration: lockupDayily(15),
      nodeDepositLockupDuration: lockupMonthly(6),
      nodeSecurityDeposit: new anchor.BN(10000 * (1e6)),
    };

    await createRegistrar(bump, registrar, realm, mint, realmAuthority, realmAuthority, votingConfig, depositConfig);

    const registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);

    assert.equal(registrarData.realm.toBase58(), realm.toBase58())
    assert.equal(registrarData.realmAuthority.toBase58(), realmAuthority.publicKey.toBase58())
    assert.equal(registrarData.governanceProgramId.toBase58(), GOV_PROGRAM_ID.toBase58())
    assert.equal(registrarData.governingTokenMint.toBase58(), mint.toBase58())
    assert.equal(registrarData.bump, bump)
    assert.equal(registrarData.votingConfig.baselineVoteWeightScaledFactor.toNumber(), votingConfig.baselineVoteWeightScaledFactor.toNumber())
    assert.equal(registrarData.votingConfig.maxExtraLockupVoteWeightScaledFactor.toNumber(), votingConfig.maxExtraLockupVoteWeightScaledFactor.toNumber())
    assert.equal(registrarData.votingConfig.lockupSaturationSecs.toNumber(), votingConfig.lockupSaturationSecs.toNumber())
    assert.equal(registrarData.depositConfig.ordinaryDepositMinLockupDuration.periods, depositConfig.ordinaryDepositMinLockupDuration.periods)
    assert.isTrue(registrarData.depositConfig.ordinaryDepositMinLockupDuration.unit.day != undefined)
    assert.equal(registrarData.depositConfig.nodeDepositLockupDuration.periods, depositConfig.nodeDepositLockupDuration.periods)
    assert.isTrue(registrarData.depositConfig.nodeDepositLockupDuration.unit.month != undefined)
    assert.equal(registrarData.depositConfig.nodeSecurityDeposit.toNumber(), depositConfig.nodeSecurityDeposit.toNumber())
  });

});


