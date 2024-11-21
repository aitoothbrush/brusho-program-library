import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, createRealm, createRegistrar, defaultDepositConfig, defaultVotingConfig, newSigner, VSR_PROGRAM } from "../helper";
import { assert } from "chai";

describe("update_voting_config!", () => {
  let realmAuthority: web3.Keypair;
  let mint: web3.PublicKey;
  let councilMint: web3.PublicKey;
  let realm: web3.PublicKey;
  let registrar: web3.PublicKey;

  before(async () => {
    realmAuthority = await newSigner();
    [mint, councilMint, realm] = await createRealm(realmAuthority);

    [registrar] = await createRegistrar(realm, realmAuthority, mint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e10), realmAuthority);
  })

  it("with_incorrect_governing_mint_should_fail", async () => {
    const newVotingConfig = {
      baselineVoteWeightScaledFactor: new anchor.BN(5e8),
      maxExtraLockupVoteWeightScaledFactor: new anchor.BN(1e8),
      lockupSaturationSecs: new anchor.BN(86400 * 30),
    }

    await assertThrowsAnchorError('ConstraintHasOne', async () => {
      await VSR_PROGRAM.methods.updateVotingConfig(
        newVotingConfig,
      ).accounts({
        registrar,
        governingTokenMint: councilMint,
        realmAuthority: realmAuthority.publicKey,
      }).signers([realmAuthority])
        .rpc()
    })
  });

  it("with_incorrect_realm_authority_should_fail", async () => {
    const newVotingConfig = {
      baselineVoteWeightScaledFactor: new anchor.BN(5e8),
      maxExtraLockupVoteWeightScaledFactor: new anchor.BN(1e8),
      lockupSaturationSecs: new anchor.BN(86400 * 30),
    }

    const invalidRealmAuthority = await newSigner();
    await assertThrowsAnchorError('ConstraintHasOne', async () => {
      await VSR_PROGRAM.methods.updateVotingConfig(
        newVotingConfig,
      ).accounts({
        registrar,
        governingTokenMint: councilMint,
        realmAuthority: invalidRealmAuthority.publicKey,
      }).signers([invalidRealmAuthority])
        .rpc()
    })
  });

  it("verify_update_voting_config", async () => {
    const newVotingConfig = {
      baselineVoteWeightScaledFactor: new anchor.BN(5e8),
      maxExtraLockupVoteWeightScaledFactor: new anchor.BN(1e8),
      lockupSaturationSecs: new anchor.BN(86400 * 30),
    }

    await VSR_PROGRAM.methods.updateVotingConfig(
      newVotingConfig,
    ).accounts({
      registrar,
      governingTokenMint: mint,
      realmAuthority: realmAuthority.publicKey,
    }).signers([realmAuthority])
      .rpc()

    const registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    assert.equal(registrarData.votingConfig.baselineVoteWeightScaledFactor.toNumber(), newVotingConfig.baselineVoteWeightScaledFactor.toNumber())
    assert.equal(registrarData.votingConfig.maxExtraLockupVoteWeightScaledFactor.toNumber(), newVotingConfig.maxExtraLockupVoteWeightScaledFactor.toNumber())
    assert.equal(registrarData.votingConfig.lockupSaturationSecs.toNumber(), newVotingConfig.lockupSaturationSecs.toNumber())
  });
});


