import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, CONNECTION, newSigner, REWARD_DISTRIBUTOR_PROGRAM } from "../helper";
import { assert } from "chai";
import { createDistributor } from "./create_distributor";

export async function createDistributionTree(
    payer: web3.Keypair,
    distributor: web3.PublicKey,
    distributorAuthority: web3.Keypair,
): Promise<{
    distributionTree: web3.PublicKey,
}> {
    const distributorData = await REWARD_DISTRIBUTOR_PROGRAM.account.distributor.fetch(distributor);
    const nextPeriod = distributorData.currentPeriod + 1;
    const periodBuffer = Buffer.alloc(4);
    periodBuffer.writeInt32BE(nextPeriod);
    const distributionTreeSeeds = [Buffer.from("distribution_tree"), distributor.toBytes(), periodBuffer];
    const [distributionTree, distributionTreeBump] = web3.PublicKey.findProgramAddressSync(distributionTreeSeeds, REWARD_DISTRIBUTOR_PROGRAM.programId);

    await REWARD_DISTRIBUTOR_PROGRAM.methods
        .createDistributionTree()
        .accounts({
            payer: payer.publicKey,
            distributionTree,
            distributor,
            authority: distributorAuthority.publicKey,
        })
        .signers([payer, distributorAuthority])
        .rpc();

    return { distributionTree }
}

describe("create_distribution_tree!", () => {
    it("with_illegal_authority_should_fail", async () => {
        const realmAuthority = await newSigner();

        const circuitBreakerThreshold = new anchor.BN("1000000000000"); // 1,000,000

        const oracles = [web3.Keypair.generate().publicKey, web3.Keypair.generate().publicKey, web3.Keypair.generate().publicKey,];
        const { realm, rewardsMint, realmAndMintAuthority, distributor, vault, circuitBreaker } =
            await createDistributor(
                realmAuthority,
                "BrushO",
                realmAuthority.publicKey,
                oracles,
                circuitBreakerThreshold
            );

        const fakeRealmAuthority = web3.Keypair.generate();
        await assertThrowsAnchorError('Authorization', async () => {
            await createDistributionTree(realmAuthority, distributor, fakeRealmAuthority);
        },
            undefined,
            false
        );
    });

    it("verify_data", async () => {
        const payerAndDistributorAuthority = await newSigner();

        const circuitBreakerThreshold = new anchor.BN("1000000000000"); // 1,000,000

        const oracles = [web3.Keypair.generate().publicKey, web3.Keypair.generate().publicKey, web3.Keypair.generate().publicKey,];
        const { realm, rewardsMint, realmAndMintAuthority, distributor, vault, circuitBreaker } =
            await createDistributor(
                payerAndDistributorAuthority,
                "BrushO",
                payerAndDistributorAuthority.publicKey,
                oracles,
                circuitBreakerThreshold
            );

        const { distributionTree } = await createDistributionTree(payerAndDistributorAuthority, distributor, payerAndDistributorAuthority);

        const distributionTreeData = await REWARD_DISTRIBUTOR_PROGRAM.account.distributionTree.fetch(distributionTree);
        assert.equal(distributor.toString(), distributionTreeData.distributor.toString())
        assert.equal(1, distributionTreeData.period)
        assert.equal(oracles.length, distributionTreeData.oracleReports.length)
    });

});


