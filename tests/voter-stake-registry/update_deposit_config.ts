import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, createRealm, createRegistrar, defaultDepositConfig, defaultVotingConfig, DepositConfig, GOV_PROGRAM_ID, lockupDayily, lockupMonthly, newMint, newSigner, VotingConfig, VSR_PROGRAM } from "./helper";
import { assert } from "chai";

describe("update_deposit_config!", () => {
  let realmAuthority: web3.Keypair;
  let mint: web3.PublicKey;
  let councilMint: web3.PublicKey;
  let realm: web3.PublicKey;
  let registrar: web3.PublicKey;

  before(async () => {
    realmAuthority = await newSigner();
    [mint, councilMint, realm] = await createRealm(realmAuthority);

    [registrar] = await createRegistrar(realm, realmAuthority, mint, defaultVotingConfig(), defaultDepositConfig(), realmAuthority);
  })

  it("with_incorrect_realm_authority_should_fail", async () => {
    const newDepositConfig = {
      ordinaryDepositMinLockupDuration: lockupDayily(30),
      nodeDepositLockupDuration: lockupMonthly(12),
      nodeSecurityDeposit: new anchor.BN(1000 * (1e6)),
    }

    const invalidRealmAuthority = await newSigner();
    await assertThrowsAnchorError('ConstraintHasOne', async () => {
      await VSR_PROGRAM.methods.updateDepositConfig(
        newDepositConfig,
      ).accounts({
        registrar,
        realmAuthority: invalidRealmAuthority.publicKey,
      }).signers([invalidRealmAuthority])
        .rpc()
    })
  });

  it("verify_update_deposit_config", async () => {
    const newDepositConfig = {
      ordinaryDepositMinLockupDuration: lockupDayily(30),
      nodeDepositLockupDuration: lockupMonthly(12),
      nodeSecurityDeposit: new anchor.BN(1000 * (1e6)),
    }

    await VSR_PROGRAM.methods.updateDepositConfig(
      newDepositConfig,
    ).accounts({
      registrar,
      realmAuthority: realmAuthority.publicKey,
    }).signers([realmAuthority])
      .rpc()

    const registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    assert.equal(registrarData.depositConfig.ordinaryDepositMinLockupDuration.periods, newDepositConfig.ordinaryDepositMinLockupDuration.periods)
    assert.isTrue(registrarData.depositConfig.ordinaryDepositMinLockupDuration.unit.day != undefined)
    assert.equal(registrarData.depositConfig.nodeDepositLockupDuration.periods, newDepositConfig.nodeDepositLockupDuration.periods)
    assert.isTrue(registrarData.depositConfig.nodeDepositLockupDuration.unit.month != undefined)
    assert.equal(registrarData.depositConfig.nodeSecurityDeposit.toNumber(), newDepositConfig.nodeSecurityDeposit.toNumber())
  });
});


