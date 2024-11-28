import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";
import { bytesToHex } from '@noble/hashes/utils';

import { accountLamports, assertThrowsAnchorError, CONNECTION, isAccountInitialized, newSigner, REWARD_DISTRIBUTOR_PROGRAM, u64ToBuffer } from "../helper";
import { assert } from "chai";
import { createDistributor } from "./create_distributor";
import { createDistributionTree } from "./create_distribution_tree";
import { SYSTEM_PROGRAM_ID } from "@solana/spl-governance";

export async function createCanopy(
    payer: web3.Keypair,
    authority: web3.Keypair,
    canopyData: web3.PublicKey,
): Promise<web3.PublicKey> {
    const canopySeeds = [Buffer.from("canopy"), canopyData.toBytes()];
    const [canopy, canopyBump] = web3.PublicKey.findProgramAddressSync(canopySeeds, REWARD_DISTRIBUTOR_PROGRAM.programId);

    await REWARD_DISTRIBUTOR_PROGRAM.methods
        .createCanopy()
        .accounts({
            payer: authority.publicKey,
            canopy,
            canopyData,
            authority: authority.publicKey,
        })
        .signers([payer, authority])
        .rpc()

    return canopy;
}

export async function fastCreateCanopy(
    payer: web3.Keypair,
    authority: web3.Keypair,
    canopyDepth: number,
): Promise<{ canopy: web3.PublicKey, canopyData: web3.PublicKey }> {
    // initialize canopy data account
    const canopyDataSize = (2 ** (canopyDepth + 1) - 2) * 32;
    const canopyData = await createCanopyData(payer, canopyDataSize);

    const canopy = await createCanopy(payer, authority, canopyData);
    return { canopyData, canopy }
}

export async function createCanopyData(payer: web3.Keypair, space: number, programId = REWARD_DISTRIBUTOR_PROGRAM.programId): Promise<web3.PublicKey> {
    const canopyDataKeypair = web3.Keypair.generate();
    const canopyDataLamports = await CONNECTION.getMinimumBalanceForRentExemption(space);
    const createCanopyDataIx = web3.SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        lamports: canopyDataLamports,
        newAccountPubkey: canopyDataKeypair.publicKey,
        programId,
        space,
    });

    const tx = new web3.Transaction().add(createCanopyDataIx);
    await web3.sendAndConfirmTransaction(CONNECTION, tx, [payer, canopyDataKeypair]);

    return canopyDataKeypair.publicKey;
}

export async function setCanopyData(
    canopy: web3.PublicKey,
    canopyAuthority: web3.Keypair,
    canopyData: web3.PublicKey,
    offset: number,
    bytes: Buffer
) {
    await REWARD_DISTRIBUTOR_PROGRAM.methods
        .setCanopyData({ offset, bytes })
        .accounts({
            canopy,
            canopyData,
            authority: canopyAuthority.publicKey
        })
        .signers([canopyAuthority])
        .rpc()
}

export async function closeCanopy(
    canopy: web3.PublicKey,
    canopyAuthority: web3.Keypair,
    canopyData: web3.PublicKey,
) {
    await REWARD_DISTRIBUTOR_PROGRAM.methods
        .closeCanopy()
        .accounts({
            canopy,
            canopyData,
            authority: canopyAuthority.publicKey
        })
        .signers([canopyAuthority])
        .rpc()
}


describe("canopy!", () => {
    let distributor: web3.PublicKey;
    let distributionTree: web3.PublicKey;
    let oracles: web3.Keypair[];

    before(async () => {
        oracles = [
            web3.Keypair.generate(),
        ]
        const distributorAuthority = await newSigner();
        const circuitBreakerThreshold = new anchor.BN("1000000000000"); // 1,000,000

        let result1 = await createDistributor(distributorAuthority, "BrushO", distributorAuthority.publicKey, oracles.map(keypair => keypair.publicKey), circuitBreakerThreshold);
        distributor = result1.distributor;

        let result2 = await createDistributionTree(distributorAuthority, distributor, distributorAuthority);
        distributionTree = result2.distributionTree;
    })

    describe("create_canopy!", () => {
        it("with_invalid_canopy_data_space_should_fail", async () => {
            const authority = await newSigner();
            const canopyData = await createCanopyData(authority, 100, SYSTEM_PROGRAM_ID); // Canopy byte length must be a multiple of 32

            await assertThrowsAnchorError('InvalidCanopyLength', async () => {
                await createCanopy(authority, authority, canopyData);
            },
                undefined,
                false
            );
        });

        it("with_not_owned_canopy_data_should_fail", async () => {
            const authority = await newSigner();
            const canopyData = await createCanopyData(authority, 32, SYSTEM_PROGRAM_ID);

            await assertThrowsAnchorError('ConstraintOwner', async () => {
                await createCanopy(authority, authority, canopyData);
            },
                undefined,
                false
            );
        });

        it("verify_data", async () => {
            const authority = await newSigner();
            const canopyDepth = 10;

            const { canopy, canopyData } = await fastCreateCanopy(authority, authority, canopyDepth);
            const canopyAccount = await REWARD_DISTRIBUTOR_PROGRAM.account.canopy.fetch(canopy);

            assert.equal(canopyData.toString(), canopyAccount.canopyData.toString())
            assert.equal(authority.publicKey.toString(), canopyAccount.authority.toString())
        });
    })

    describe("set_canopy_data!", () => {
        let canopy: web3.PublicKey;
        let canopyAuthority: web3.Keypair;
        let canopyData: web3.PublicKey;
        const canopyDepth = 5;

        before(async () => {
            canopyAuthority = await newSigner();
            const result = await fastCreateCanopy(canopyAuthority, canopyAuthority, canopyDepth);
            canopy = result.canopy;
            canopyData = result.canopyData;
        })

        it("with_invalid_authority_should_fail", async () => {
            const fakeAuthority = await newSigner();

            await assertThrowsAnchorError('Authorization', async () => {
                await setCanopyData(canopy, fakeAuthority, canopyData, 0, Buffer.from("abc"));
            },
                undefined,
                false
            );
        });

        it("with_invalid_canopy_data_should_fail", async () => {
            const fakeCanopyData = web3.Keypair.generate().publicKey;

            await assertThrowsAnchorError('ConstraintHasOne', async () => {
                await setCanopyData(canopy, canopyAuthority, fakeCanopyData, 0, Buffer.from("abc"));
            },
                undefined,
                false
            );
        });

        it("verify_data", async () => {
            // data[0..8]
            let offset = 0;
            let bytes = Buffer.from("abcdefgh");
            await setCanopyData(canopy, canopyAuthority, canopyData, offset, bytes);

            let canopyDataAccount = await CONNECTION.getAccountInfo(canopyData);
            assert.equal(bytes.toString(), canopyDataAccount.data.subarray(offset, offset + bytes.length).toString())

            // data[8..16]
            offset = bytes.length;
            bytes = Buffer.from("ijklmnop")
            await setCanopyData(canopy, canopyAuthority, canopyData, offset, bytes);

            canopyDataAccount = await CONNECTION.getAccountInfo(canopyData);
            assert.equal(bytes.toString(), canopyDataAccount.data.subarray(offset, offset + bytes.length).toString())
        });
    })

    describe("close_canopy!", () => {
        let canopy: web3.PublicKey;
        let canopyAuthority: web3.Keypair;
        let canopyData: web3.PublicKey;
        const canopyDepth = 5;

        before(async () => {
            canopyAuthority = await newSigner();
            const result = await fastCreateCanopy(canopyAuthority, canopyAuthority, canopyDepth);
            canopy = result.canopy;
            canopyData = result.canopyData;
        })

        it("with_invalid_authority_should_fail", async () => {
            const fakeAuthority = await newSigner();

            await assertThrowsAnchorError('Authorization', async () => {
                await closeCanopy(canopy, fakeAuthority, canopyData);
            },
                undefined,
                false
            );
        });

        it("close_canopy", async () => {
            const prevCanopyAuthorityLamports = await accountLamports(canopyAuthority.publicKey);

            assert.isTrue((await isAccountInitialized(canopy)))
            assert.isTrue((await isAccountInitialized(canopyData)))

            await closeCanopy(canopy, canopyAuthority, canopyData);

            assert.isFalse((await isAccountInitialized(canopy)))
            assert.isFalse((await isAccountInitialized(canopyData)))

            const postCanopyAuthorityLamports = await accountLamports(canopyAuthority.publicKey);
            assert.isTrue(postCanopyAuthorityLamports > prevCanopyAuthorityLamports);
        });
    })

});
