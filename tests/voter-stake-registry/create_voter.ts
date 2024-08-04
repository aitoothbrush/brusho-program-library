import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, CONNECTION, createRealm, createRegistrar, defaultDepositConfig, defaultVotingConfig, newSigner, VSR_PROGRAM } from "./helper";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAccount, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";
import { getVoterWeightRecord } from "@solana/spl-governance";

async function createVoter(
  voterBump,
  voterWeightRecordBump,
  registrar,
  governingTokenMint,
  voter,
  voterAuthority,
  vault,
  voterWeightRecord,
  payer,
): Promise<string> {
  return await VSR_PROGRAM.methods.createVoter(
    voterBump,
    voterWeightRecordBump,
  ).accounts({
    registrar,
    governingTokenMint,
    voter,
    voterAuthority: voterAuthority.publicKey,
    vault,
    voterWeightRecord,
    payer: payer.publicKey,
    tokenProgram: TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  }).signers([voterAuthority, payer])
    .rpc()
}

describe("create_voter!", () => {
  let realmAuthority: web3.Keypair;
  let mint: web3.PublicKey;
  let councilMint: web3.PublicKey;
  let realm: web3.PublicKey;
  let registrar: web3.PublicKey;

  before(async () => {
    realmAuthority = await newSigner();
    [mint, councilMint, realm] = await createRealm(realmAuthority);

    [registrar] = await createRegistrar(realm, realmAuthority, mint, defaultVotingConfig(), defaultDepositConfig(), new anchor.BN(1e9), realmAuthority);
  })

  it("with_incorrect_voter_authority_should_fail", async () => {
    const voterAuthority = await newSigner();

    const voterSeeds = [registrar.toBytes(), Buffer.from("voter"), voterAuthority.publicKey.toBytes()];
    let [voter, voterBump] = anchor.web3.PublicKey.findProgramAddressSync(voterSeeds, VSR_PROGRAM.programId);

    const voterWeightRecordSeeds = [registrar.toBytes(), Buffer.from("voter-weight-record"), voterAuthority.publicKey.toBytes()];
    let [voterWeightRecord, voterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(voterWeightRecordSeeds, VSR_PROGRAM.programId);

    const vaultSeeds = [voter.toBytes(), TOKEN_PROGRAM_ID.toBytes(), mint.toBytes()];
    let [vault, vaultBump] = anchor.web3.PublicKey.findProgramAddressSync(vaultSeeds, ASSOCIATED_TOKEN_PROGRAM_ID);

    const incorrectVoterAuthority = await newSigner();
    await assertThrowsAnchorError('ConstraintSeeds', async () => {
      await createVoter(
        voterBump,
        voterWeightRecordBump,
        registrar,
        mint,
        voter,
        incorrectVoterAuthority, // incorrect voterAuthority
        vault,
        voterWeightRecord,
        voterAuthority,
      );
    })
  });

  it("with_incorrect_voter_bump_should_fail", async () => {
    const voterAuthority = await newSigner();

    const voterSeeds = [registrar.toBytes(), Buffer.from("voter"), voterAuthority.publicKey.toBytes()];
    let [voter, voterBump] = anchor.web3.PublicKey.findProgramAddressSync(voterSeeds, VSR_PROGRAM.programId);

    const voterWeightRecordSeeds = [registrar.toBytes(), Buffer.from("voter-weight-record"), voterAuthority.publicKey.toBytes()];
    let [voterWeightRecord, voterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(voterWeightRecordSeeds, VSR_PROGRAM.programId);

    const vaultSeeds = [voter.toBytes(), TOKEN_PROGRAM_ID.toBytes(), mint.toBytes()];
    let [vault, vaultBump] = anchor.web3.PublicKey.findProgramAddressSync(vaultSeeds, ASSOCIATED_TOKEN_PROGRAM_ID);

    await assertThrowsAnchorError('RequireEqViolated', async () => {
      await createVoter(
        voterBump - 1, // incorrect voterBump
        voterWeightRecordBump,
        registrar,
        mint,
        voter,
        voterAuthority,
        vault,
        voterWeightRecord,
        voterAuthority,
      );
    })
  });

  it("with_incorrect_voter_weight_record_bump_should_fail", async () => {
    const voterAuthority = await newSigner();

    const voterSeeds = [registrar.toBytes(), Buffer.from("voter"), voterAuthority.publicKey.toBytes()];
    let [voter, voterBump] = anchor.web3.PublicKey.findProgramAddressSync(voterSeeds, VSR_PROGRAM.programId);

    const voterWeightRecordSeeds = [registrar.toBytes(), Buffer.from("voter-weight-record"), voterAuthority.publicKey.toBytes()];
    let [voterWeightRecord, voterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(voterWeightRecordSeeds, VSR_PROGRAM.programId);

    const vaultSeeds = [voter.toBytes(), TOKEN_PROGRAM_ID.toBytes(), mint.toBytes()];
    let [vault, vaultBump] = anchor.web3.PublicKey.findProgramAddressSync(vaultSeeds, ASSOCIATED_TOKEN_PROGRAM_ID);

    await assertThrowsAnchorError('RequireEqViolated', async () => {
      await createVoter(
        voterBump,
        voterWeightRecordBump - 1, // incorrect voterBump
        registrar,
        mint,
        voter,
        voterAuthority,
        vault,
        voterWeightRecord,
        voterAuthority,
      );
    })
  });

  it("verify_voter_data", async () => {
    const voterAuthority = await newSigner();

    const voterSeeds = [registrar.toBytes(), Buffer.from("voter"), voterAuthority.publicKey.toBytes()];
    let [voter, voterBump] = anchor.web3.PublicKey.findProgramAddressSync(voterSeeds, VSR_PROGRAM.programId);

    const voterWeightRecordSeeds = [registrar.toBytes(), Buffer.from("voter-weight-record"), voterAuthority.publicKey.toBytes()];
    let [voterWeightRecord, voterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(voterWeightRecordSeeds, VSR_PROGRAM.programId);

    const vaultSeeds = [voter.toBytes(), TOKEN_PROGRAM_ID.toBytes(), mint.toBytes()];
    let [vault, vaultBump] = anchor.web3.PublicKey.findProgramAddressSync(vaultSeeds, ASSOCIATED_TOKEN_PROGRAM_ID);

    const txId = await createVoter(
      voterBump,
      voterWeightRecordBump,
      registrar,
      mint,
      voter,
      voterAuthority,
      vault,
      voterWeightRecord,
      voterAuthority,
    );

    // verify registrar data
    let registrarData = await VSR_PROGRAM.account.registrar.fetch(registrar);
    const tx = await CONNECTION.getTransaction(txId, {commitment: 'confirmed'});
    const txTime = registrarData.timeOffset.add(new anchor.BN(tx.blockTime.toString()));
    assert.equal(registrarData.rewardAccrualTs.toString(), txTime.toString());

    // verify voter account
    const voterData = await VSR_PROGRAM.account.voter.fetch(voter);
    assert.equal(voterData.voterAuthority.toBase58(), voterAuthority.publicKey.toBase58())
    assert.equal(voterData.registrar.toBase58(), registrar.toBase58())
    assert.equal(voterData.voterBump, voterBump)
    assert.equal(voterData.voterWeightRecordBump, voterWeightRecordBump);
    assert.equal(voterData.rewardIndex.v.toString(), registrarData.rewardIndex.v.toString())
    assert.isTrue(voterData.rewardClaimableAmount.eqn(0));
    for (let e of voterData.deposits) {
      assert.isFalse(e.isActive)
      assert.equal(e.amountDepositedNative.toString(), "0")
      assert.equal(e.amountInitiallyLockedNative.toString(), "0")
    }

    // verify vault account
    const vaultData = await getAccount(CONNECTION, vault);
    assert.equal(vaultData.mint.toBase58(), mint.toBase58());
    assert.equal(vaultData.owner.toBase58(), voter.toBase58());
    assert.equal(vaultData.amount.toString(), "0");

    // verify VoteWeightRecord account
    const voteWeightRecordData = await getVoterWeightRecord(CONNECTION, voterWeightRecord);
    assert.equal(voteWeightRecordData.owner.toBase58(), VSR_PROGRAM.programId.toBase58());
    assert.equal(voteWeightRecordData.account.governingTokenMint.toBase58(), mint.toBase58());
    assert.equal(voteWeightRecordData.account.governingTokenOwner.toBase58(), voterAuthority.publicKey.toBase58());
    assert.equal(voteWeightRecordData.account.realm.toBase58(), realm.toBase58());
    // assert.equal(voteWeightRecordData.account.accountDiscriminator.toString(), Uint8Array.from([46, 249, 155, 75, 153, 248, 116, 9]).toString());
    // console.log(voteWeightRecordData.account.accountDiscriminator)
  });
});


