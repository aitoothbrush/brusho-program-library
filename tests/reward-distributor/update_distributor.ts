import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { assertThrowsAnchorError, newSigner, REWARD_DISTRIBUTOR_PROGRAM } from "../helper";
import { assert } from "chai";
import { createDistributor } from "./create_distributor";

describe("update_distributor!", () => {
    let realmAuthority: web3.Keypair;
    let distributor: web3.PublicKey;

    before(async () => {
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

        let result = await createDistributor(authority, name, authority.publicKey, oracles, circuitBreakerThreshold);
        realmAuthority = result.realmAndMintAuthority;
        distributor = result.distributor;
    })

    it("with_illegal_realm_authority_should_fail", async () => {
        const newAuthority = web3.Keypair.generate();
        const newOracles = [
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey
        ]

        const fakeRealmAuthority = web3.Keypair.generate();
        await assertThrowsAnchorError('ConstraintHasOne', async () => {
            await REWARD_DISTRIBUTOR_PROGRAM.methods
                .updateDistributor({ authority: newAuthority.publicKey, oracles: newOracles })
                .accounts({
                    distributor,
                    realmAuthority: fakeRealmAuthority.publicKey,
                })
                .signers([fakeRealmAuthority])
                .rpc()
        },
            undefined,
            false
        );
    });

    it("with_too_many_oracles_should_fail", async () => {
        const newAuthority = web3.Keypair.generate();
        const newOracles = [
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey
        ]

        await assertThrowsAnchorError('OraclesCountExceeds', async () => {
            await REWARD_DISTRIBUTOR_PROGRAM.methods
                .updateDistributor({ authority: newAuthority.publicKey, oracles: newOracles })
                .accounts({
                    distributor,
                    realmAuthority: realmAuthority.publicKey,
                })
                .signers([realmAuthority])
                .rpc()
        },
            undefined,
            false
        );
    });

    it("verify_data", async () => {
        const newAuthority = web3.Keypair.generate();
        const newOracles = [
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey,
            web3.Keypair.generate().publicKey
        ]
        await REWARD_DISTRIBUTOR_PROGRAM.methods
            .updateDistributor({ authority: newAuthority.publicKey, oracles: newOracles })
            .accounts({
                distributor,
                realmAuthority: realmAuthority.publicKey
            })
            .signers([realmAuthority])
            .rpc()

        // Verify distributor data
        const distributorData = await REWARD_DISTRIBUTOR_PROGRAM.account.distributor.fetch(distributor);
        assert.equal(newAuthority.publicKey.toBase58(), distributorData.authority.toBase58())
        assert.equal(5, distributorData.oracles.length)
        assert.equal(newOracles[0].toString(), distributorData.oracles[0].toString())
        assert.equal(newOracles[1].toString(), distributorData.oracles[1].toString())
        assert.equal(newOracles[2].toString(), distributorData.oracles[2].toString())
        assert.equal(newOracles[3].toString(), distributorData.oracles[3].toString())
        assert.equal(newOracles[4].toString(), distributorData.oracles[4].toString())
    });

});


