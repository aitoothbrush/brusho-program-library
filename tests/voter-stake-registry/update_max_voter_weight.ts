import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { CIRCUIT_BREAKER_PROGRAM, createRealm, defaultDepositConfig, defaultVotingConfig, DepositConfig, GOV_PROGRAM_ID, mintTokenToWallet, newSigner, VotingConfig, VSR_PROGRAM } from "./helper";
import { assert } from "chai";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";
import { getMaxVoterWeightRecord } from "@solana/spl-governance";

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

describe("update_max_voter_weight!", () => {
  it("verify_update_max_voter_weight", async () => {
    const authority = await newSigner();
    const [mint, councilMint, realm] = await createRealm(authority);

    const seeds = [realm.toBytes(), Buffer.from("registrar"), mint.toBytes()];
    const [registrar, bump] = web3.PublicKey.findProgramAddressSync(seeds, VSR_PROGRAM.programId);

    const vault = getAssociatedTokenAddressSync(mint, registrar, true);
    const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
    const [circuitBreaker, circuitBreakerBump] = anchor.web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

    const maxVoterWeightRecordSeeds = [realm.toBytes(), Buffer.from("max-voter-weight-record"), mint.toBytes()];
    const [maxVoterWeightRecord, maxVoterWeightRecordBump] = anchor.web3.PublicKey.findProgramAddressSync(maxVoterWeightRecordSeeds, VSR_PROGRAM.programId);

    const baselineVoteWeightScaledFactorOne = new anchor.BN(1e9);
    const votingConfig = {
      baselineVoteWeightScaledFactor: new anchor.BN(11e8), // 1.1
      maxExtraLockupVoteWeightScaledFactor: new anchor.BN(0),
      lockupSaturationSecs: new anchor.BN(86400),
    };

    await createRegistrar(bump, registrar, vault, circuitBreaker, maxVoterWeightRecord, realm, mint, authority, authority, votingConfig);

    // mint some tokens
    const tokenSupply = new anchor.BN(1e10); // 10000
    await mintTokenToWallet(mint, authority, web3.Keypair.generate().publicKey, tokenSupply);

    // call update_max_voter_weight
    const txId = await VSR_PROGRAM.methods.updateMaxVoteWeight()
      .accounts({
        registrar,
        maxVoterWeightRecord,
        governingTokenMint: mint
      })
      .rpc();
    // const tx = await CONNECTION.getTransaction(txId, {commitment: "confirmed"})

    // verify MaxVoterWeightRecord
    const maxVoterWeightRecordData = await getMaxVoterWeightRecord(anchor.getProvider().connection, maxVoterWeightRecord);
    assert.equal(maxVoterWeightRecordData.owner.toBase58(), VSR_PROGRAM.programId.toBase58());
    assert.equal(maxVoterWeightRecordData.account.realm.toBase58(), realm.toBase58());
    assert.equal(maxVoterWeightRecordData.account.governingTokenMint.toBase58(), mint.toBase58());
    assert.equal(
      maxVoterWeightRecordData.account.maxVoterWeight.toString(), 
      tokenSupply.mul(votingConfig.baselineVoteWeightScaledFactor).div(baselineVoteWeightScaledFactorOne).toString()
    );
    // assert.equal(maxVoterWeightRecordData.account.maxVoterWeightExpiry.toNumber(), tx!.slot)
  });

});


