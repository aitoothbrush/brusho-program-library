import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, newSigner, REWARD_DISTRIBUTOR_PROGRAM } from "../helper";
import { assert } from "chai";
import { createDistributor } from "./create_distributor";
import { createDistributionTree } from "./create_distribution_tree";
import { reportOracle } from "./report_oracle";

export async function activateDistributionTree(
    distributor: web3.PublicKey,
    distributorAuthority: web3.Keypair,
    distributionTree: web3.PublicKey,
) {
    await REWARD_DISTRIBUTOR_PROGRAM.methods
        .activateDistributionTree()
        .accounts({
            distributor,
            distributionTree,
            authority: distributorAuthority.publicKey,
        })
        .signers([distributorAuthority])
        .rpc()
}


describe("activate_distribution_tree!", () => {
    let distributor: web3.PublicKey;
    let distributorAuthority: web3.Keypair;
    let distributionTree: web3.PublicKey;
    let oracles: web3.Keypair[];

    before(async () => {
        oracles = [
            web3.Keypair.generate(),
            web3.Keypair.generate(),
            web3.Keypair.generate(),
        ]
        distributorAuthority = await newSigner();
        const circuitBreakerThreshold = new anchor.BN("1000000000000"); // 1,000,000

        let result1 = await createDistributor(distributorAuthority, "BrushO", distributorAuthority.publicKey, oracles.map(keypair => keypair.publicKey), circuitBreakerThreshold);
        distributor = result1.distributor;

        let result2 = await createDistributionTree(distributorAuthority, distributor, distributorAuthority);
        distributionTree = result2.distributionTree;
    })

    it("with_illegal_authority_should_fail", async () => {
        const fakeAuthority = web3.Keypair.generate();
        await assertThrowsAnchorError('Authorization', async () => {
            await REWARD_DISTRIBUTOR_PROGRAM.methods
                .activateDistributionTree()
                .accounts({
                    distributor,
                    distributionTree,
                    authority: fakeAuthority.publicKey,
                })
                .signers([fakeAuthority])
                .rpc()
        },
            undefined,
            false
        );
    });

    it("with_illegal_oracles_should_fail", async () => {
        await assertThrowsAnchorError('IllegalOracleReports', async () => {
            await activateDistributionTree(distributor, distributorAuthority, distributionTree);
        },
            undefined,
            false
        );
    });

    it("verify_data", async () => {
        const root = Array(32).fill(1);
        const maxDepth = 10;
        const report = { root, maxDepth };

        // Set reports that meet conditions
        await reportOracle(distributor, distributionTree, oracles[0], 0, report);
        await reportOracle(distributor, distributionTree, oracles[1], 1, report);
        // now we can call activate 
        await activateDistributionTree(distributor, distributorAuthority, distributionTree);

        let distributionTreeData = await REWARD_DISTRIBUTOR_PROGRAM.account.distributionTree.fetch(distributionTree);
        let distributorData = await REWARD_DISTRIBUTOR_PROGRAM.account.distributor.fetch(distributor);

        assert.equal(distributionTreeData.period, distributorData.currentPeriod)

        // report again should fail
        await assertThrowsAnchorError('CannotReportAtPresent', async () => {
            await reportOracle(distributor, distributionTree, oracles[2], 2, report);
        },
            undefined,
            false
        );

        // activate again should fail
        await assertThrowsAnchorError('IllegalPeriod', async () => {
            await activateDistributionTree(distributor, distributorAuthority, distributionTree);
        },
            undefined,
            false
        );
    });

});
