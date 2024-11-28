import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, CIRCUIT_BREAKER_PROGRAM, createRealm, GOV_PROGRAM_ID, newSigner, REWARD_DISTRIBUTOR_PROGRAM, SECS_PER_DAY } from "../helper";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

export async function createDistributor(
    payer: web3.Keypair,
    name: string,
    authority: web3.PublicKey,
    oracles: web3.PublicKey[],
    circuitBreakerThreshold: anchor.BN,
): Promise<{
    realm: web3.PublicKey,
    rewardsMint: web3.PublicKey,
    realmAndMintAuthority: web3.Keypair,
    distributor: web3.PublicKey,
    vault: web3.PublicKey,
    circuitBreaker: web3.PublicKey
}> {
    const realmAuthority = await newSigner();
    const [rewardsMint, councilMint, realm] = await createRealm(realmAuthority);

    const distributorSeeds = [Buffer.from("distributor"), realm.toBytes(), rewardsMint.toBytes(), Buffer.from(name)];
    const [distributor, distributorBump] = web3.PublicKey.findProgramAddressSync(distributorSeeds, REWARD_DISTRIBUTOR_PROGRAM.programId);
    const vault = getAssociatedTokenAddressSync(rewardsMint, distributor, true);
    const circuitBreakerSeeds = [Buffer.from("account_windowed_breaker"), vault.toBytes()];
    const [circuitBreaker, circuitBreakerBump] = web3.PublicKey.findProgramAddressSync(circuitBreakerSeeds, CIRCUIT_BREAKER_PROGRAM.programId);

    const circuitBreakerConfig = {
        windowSizeSeconds: SECS_PER_DAY,
        thresholdType: { absolute: {} },
        threshold: circuitBreakerThreshold,
    };

    await REWARD_DISTRIBUTOR_PROGRAM.methods
        .createDistributor({ name, authority, oracles, circuitBreakerConfig })
        .accounts({
            payer: payer.publicKey,
            distributor,
            vault,
            rewardsMint,
            circuitBreaker,
            realm,
            realmAuthority: realmAuthority.publicKey,
            governanceProgramId: GOV_PROGRAM_ID,
            circuitBreakerProgram: CIRCUIT_BREAKER_PROGRAM.programId,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            tokenProgram: TOKEN_PROGRAM_ID
        })
        .signers([payer, realmAuthority])
        .rpc()

    return { realm, rewardsMint, realmAndMintAuthority: realmAuthority, distributor, vault, circuitBreaker }
}

describe("create_distributor!", () => {
    it("with_too_many_oracles_should_fail", async () => {
        const name = "BrushO"
        // give more than 5 oracles
        const oracles = [
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
        ]
        const authority = await newSigner();
        const circuitBreakerThreshold = new anchor.BN("1000000000000"); // 1,000,000

        await assertThrowsAnchorError('OraclesCountExceeds', async () => {
            await createDistributor(authority, name, authority.publicKey, oracles, circuitBreakerThreshold);
        },
            undefined,
            false
        );
    });

    it("verify_data", async () => {
        const name = "BrushO"
        const oracles = [
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey
        ]
        const authority = await newSigner();
        const circuitBreakerThreshold = new anchor.BN("1000000000000"); // 1,000,000

        const { realm, rewardsMint, realmAndMintAuthority, distributor, vault, circuitBreaker } =
            await createDistributor(authority, name, authority.publicKey, oracles, circuitBreakerThreshold);

        // Verify distributor data
        const distributorData = await REWARD_DISTRIBUTOR_PROGRAM.account.distributor.fetch(distributor);
        assert.equal(realm.toBase58(), distributorData.realm.toBase58())
        assert.equal(realmAndMintAuthority.publicKey.toBase58(), distributorData.realmAuthority.toBase58())
        assert.equal(rewardsMint.toBase58(), distributorData.rewardsMint.toBase58())
        assert.equal(authority.publicKey.toBase58(), distributorData.authority.toBase58())
        assert.equal(name, distributorData.name)
        assert.equal(5, distributorData.oracles.length)
        assert.equal(oracles[0].toString(), distributorData.oracles[0].toString())
        assert.equal(oracles[1].toString(), distributorData.oracles[1].toString())
        assert.equal(oracles[2].toString(), distributorData.oracles[2].toString())
        assert.equal(oracles[3].toString(), distributorData.oracles[3].toString())
        assert.equal(oracles[4].toString(), distributorData.oracles[4].toString())
        assert.equal(0, distributorData.currentPeriod)

        // verify cirruitBreaker data
        const circuitBreakerData = await CIRCUIT_BREAKER_PROGRAM.account.accountWindowedCircuitBreakerV0.fetch(circuitBreaker);
        assert.equal(circuitBreakerData.tokenAccount.toBase58(), vault.toBase58());
        assert.equal(circuitBreakerData.authority.toBase58(), realmAndMintAuthority.publicKey.toBase58());
        assert.equal(circuitBreakerData.owner.toBase58(), distributor.toBase58());
        assert.equal(circuitBreakerData.config.windowSizeSeconds.toNumber(), SECS_PER_DAY.toNumber());
        assert.isTrue(circuitBreakerData.config.thresholdType.absolute != undefined);
        assert.equal(circuitBreakerData.config.threshold.toNumber(), circuitBreakerThreshold.toNumber());
    });

});


