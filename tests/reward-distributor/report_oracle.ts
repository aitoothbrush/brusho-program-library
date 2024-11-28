import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, assertThrowsSendTransactionError, newSigner, REWARD_DISTRIBUTOR_PROGRAM } from "../helper";
import { assert } from "chai";
import { createDistributor } from "./create_distributor";
import { createDistributionTree } from "./create_distribution_tree";

export async function reportOracle(
    distributor: web3.PublicKey,
    distributionTree: web3.PublicKey,
    oracleAuthority: web3.Keypair,
    index: number,
    report: { root: number[], maxDepth: number } 
) {
    await REWARD_DISTRIBUTOR_PROGRAM.methods
        .reportOracle({ index, report })
        .accounts({
            distributor,
            distributionTree,
            authority: oracleAuthority.publicKey,
        })
        .signers([oracleAuthority])
        .rpc()
}

describe("report_oracle!", () => {
    let distributor: web3.PublicKey;
    let distributionTree: web3.PublicKey;
    let oracles: web3.Keypair[];

    before(async () => {
        oracles = [
            web3.Keypair.generate(),
            web3.Keypair.generate(),
            web3.Keypair.generate(),
            web3.Keypair.generate(),
            web3.Keypair.generate()
        ]
        const distributorAuthority = await newSigner();
        const circuitBreakerThreshold = new anchor.BN("1000000000000"); // 1,000,000

        let result1 = await createDistributor(distributorAuthority, "BrushO", distributorAuthority.publicKey, oracles.map(keypair => keypair.publicKey), circuitBreakerThreshold);
        distributor = result1.distributor;

        let result2 = await createDistributionTree(distributorAuthority, distributor, distributorAuthority);
        distributionTree = result2.distributionTree;
    })

    it("with_invalid_oracle_index_should_fail", async () => {
        const fakeAuthority = web3.Keypair.generate();
        const root = Array(32).fill(1);
        const maxDepth = 10;
        await assertThrowsSendTransactionError('Program failed to complete', async () => {
            await REWARD_DISTRIBUTOR_PROGRAM.methods
                .reportOracle({ index: oracles.length, report: { root, maxDepth } }) // oracles.length is an invalid index
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

    it("with_illegal_authority_should_fail", async () => {
        const fakeAuthority = web3.Keypair.generate();
        const root = Array(32).fill(1);
        const maxDepth = 10;
        await assertThrowsAnchorError('Authorization', async () => {
            await REWARD_DISTRIBUTOR_PROGRAM.methods
                .reportOracle({ index: 0, report: { root, maxDepth } })
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

    it("verify_data", async () => {
        const root = Array(32).fill(1);
        const maxDepth = 10;
        const report = { root, maxDepth };
        
        // report at index 0
        await reportOracle(distributor, distributionTree, oracles[0], 0, report);

        let distributionTreeData = await REWARD_DISTRIBUTOR_PROGRAM.account.distributionTree.fetch(distributionTree);
        assert.equal(String(report.root), String(distributionTreeData.oracleReports[0].root))

        // report at index 4
        await reportOracle(distributor, distributionTree, oracles[4], 4, report);
        distributionTreeData = await REWARD_DISTRIBUTOR_PROGRAM.account.distributionTree.fetch(distributionTree);
        assert.equal(String(report.root), String(distributionTreeData.oracleReports[0].root))
        assert.equal(String(report.root), String(distributionTreeData.oracleReports[4].root))
    });

});


