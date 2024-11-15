import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { ACCOUNT_COMPRESSION_PROGRAM_ID, BRUSHO_NFT_MANAGER_PROGRAM, CONNECTION, createRealm, GOV_PROGRAM_ID, MPL_BUBBLEGUM_PROGRAM_ID, newSigner, NOOP_PROGRAM_ID, TOKEN_METADATA_PROGRAM_ID } from "../helper";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { createAllocTreeIx, ValidDepthSizePair } from "@solana/spl-account-compression";
import { Keypair, PublicKey, sendAndConfirmTransaction, Transaction } from "@solana/web3.js";

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
    let realm: web3.PublicKey;
    let realmAuthority: web3.Keypair;
    let mint: web3.PublicKey;
    let councilMint: web3.PublicKey;
    let makerAuthority: web3.Keypair;
    let name: string;
    let maker: web3.PublicKey;
    let makerBump: number;
    let collection: web3.PublicKey;
    let collectionBump: number;
    let metadata: web3.PublicKey;
    let edition: web3.PublicKey;

    before(async () => {
        payer = await newSigner();
        realmAuthority = await newSigner();
        [mint, councilMint, realm] = await createRealm(realmAuthority);

        makerAuthority = await newSigner();
        name = "BrushO"
        const makerSeeds = [Buffer.from("maker"), realm.toBytes(), Buffer.from(name)];
        [maker, makerBump] = anchor.web3.PublicKey.findProgramAddressSync(makerSeeds, BRUSHO_NFT_MANAGER_PROGRAM.programId);
        const collectionSeeds = [Buffer.from("collection"), maker.toBytes()];
        [collection, collectionBump] = anchor.web3.PublicKey.findProgramAddressSync(collectionSeeds, BRUSHO_NFT_MANAGER_PROGRAM.programId);
        const metadataSeeds = [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBytes(), collection.toBytes()];
        [metadata,] = anchor.web3.PublicKey.findProgramAddressSync(metadataSeeds, TOKEN_METADATA_PROGRAM_ID);
        const editionSeeds = [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBytes(), collection.toBytes(), Buffer.from("edition")];
        [edition,] = anchor.web3.PublicKey.findProgramAddressSync(editionSeeds, TOKEN_METADATA_PROGRAM_ID);
        const tokenAccount = await getAssociatedTokenAddress(collection, maker, true);

        await BRUSHO_NFT_MANAGER_PROGRAM.methods
            .initializeMaker({ issuingAuthority: makerAuthority.publicKey, updateAuthority: makerAuthority.publicKey, name, metadataUrl: "http://bb.io/metadata" })
            .accounts({
                payer: makerAuthority.publicKey,
                maker,
                realm,
                governanceProgramId: GOV_PROGRAM_ID,
                realmAuthority: realmAuthority.publicKey,
                collection,
                metadata,
                masterEdition: edition,
                tokenAccount,
                tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
                associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
                tokenProgram: TOKEN_PROGRAM_ID
            })
            .signers([realmAuthority, makerAuthority])
            .rpc()
    })

    it("verify_data", async () => {
        const depthSizePair: ValidDepthSizePair = {maxDepth: 3, maxBufferSize: 8};
        const merkleTree = await createTree(depthSizePair, 0);
        const treeAuthoritySeeds = [merkleTree.toBytes()];
        const [treeAuthority,] = anchor.web3.PublicKey.findProgramAddressSync(treeAuthoritySeeds, MPL_BUBBLEGUM_PROGRAM_ID);

        await BRUSHO_NFT_MANAGER_PROGRAM.methods
            .setMakerTree({ maxDepth: depthSizePair.maxDepth, maxBufferSize: depthSizePair.maxBufferSize })
            .accounts({
                payer: makerAuthority.publicKey,
                updateAuthority: makerAuthority.publicKey,
                maker,
                treeAuthority,
                merkleTree,
                logWrapper: NOOP_PROGRAM_ID,
                compressionProgram: ACCOUNT_COMPRESSION_PROGRAM_ID,
                bubblegumProgram: MPL_BUBBLEGUM_PROGRAM_ID
            })
            .signers([makerAuthority])
            .rpc()
    });

});


