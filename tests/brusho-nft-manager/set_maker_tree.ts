import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { ACCOUNT_COMPRESSION_PROGRAM_ID, BRUSHO_NFT_MANAGER_PROGRAM, CONNECTION, MPL_BUBBLEGUM_PROGRAM_ID, newSigner, NOOP_PROGRAM_ID } from "../helper";
import { createAllocTreeIx, ValidDepthSizePair } from "@solana/spl-account-compression";
import { Keypair, PublicKey, sendAndConfirmTransaction, Transaction } from "@solana/web3.js";
import { createMaker } from "./initialize_maker";
import { assert } from "chai";

export async function setMakerTree(
    payer: Keypair,
    maker: PublicKey,
    makerUpdateAuthority: Keypair,
    depthSizePair: ValidDepthSizePair,
    canopyDepth: number
): Promise<{ merkleTree: PublicKey, treeAuthority: PublicKey }> {
    const merkleTree = await createTree(depthSizePair, canopyDepth);
    const treeAuthoritySeeds = [merkleTree.toBytes()];
    const [treeAuthority,] = anchor.web3.PublicKey.findProgramAddressSync(treeAuthoritySeeds, MPL_BUBBLEGUM_PROGRAM_ID);

    await BRUSHO_NFT_MANAGER_PROGRAM.methods
        .setMakerTree({ maxDepth: depthSizePair.maxDepth, maxBufferSize: depthSizePair.maxBufferSize })
        .accounts({
            payer: payer.publicKey,
            updateAuthority: makerUpdateAuthority.publicKey,
            maker,
            treeAuthority,
            merkleTree,
            logWrapper: NOOP_PROGRAM_ID,
            compressionProgram: ACCOUNT_COMPRESSION_PROGRAM_ID,
            bubblegumProgram: MPL_BUBBLEGUM_PROGRAM_ID
        })
        .signers([payer, makerUpdateAuthority])
        .rpc()

    return { merkleTree, treeAuthority }
}

async function createTree(sizePair: ValidDepthSizePair, canopyDepth: number): Promise<PublicKey> {
    const payer = await newSigner();
    const merkleTreeKeypair = Keypair.generate();
    const merkleTree = merkleTreeKeypair.publicKey;
    const allocAccountIx = await createAllocTreeIx(
        CONNECTION,
        merkleTree,
        payer.publicKey,
        sizePair,
        canopyDepth,
    );

    const tx = new Transaction().add(allocAccountIx);
    await sendAndConfirmTransaction(CONNECTION, tx, [payer, merkleTreeKeypair]);

    return merkleTree
}

describe("set_maker_tree!", () => {
    let payer: web3.Keypair;
    let makerAuthority: web3.Keypair;

    before(async () => {
        payer = await newSigner();
        makerAuthority = await newSigner();
    })

    it("verify_data", async () => {
        const { realm, mint, councilMint, realmAuthority, maker, makerBump, collection, collectionBump, metadata, edition, tokenAccount } =
            await createMaker(payer, makerAuthority.publicKey, makerAuthority.publicKey, "BRUSH", "https://abcd.xyz/metadata");
        const depthSizePair: ValidDepthSizePair = { maxDepth: 3, maxBufferSize: 8 };
        const { merkleTree, treeAuthority } = await setMakerTree(payer, maker, makerAuthority, depthSizePair, 0);

        const makerData = await BRUSHO_NFT_MANAGER_PROGRAM.account.maker.fetch(maker);
        assert.equal(merkleTree.toBase58(), makerData.merkleTree.toBase58())
    });

});


