import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { BRUSHO_NFT_MANAGER_PROGRAM, createRealm, getMint, getTokenAccount, GOV_PROGRAM_ID, newSigner, TOKEN_METADATA_PROGRAM_ID } from "../helper";
import { ASSOCIATED_TOKEN_PROGRAM_ID, getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

export async function createMaker(
    payer: web3.Keypair,
    issuingAuthority: web3.PublicKey,
    updateAuthority: web3.PublicKey,
    name: string,
    metadataUrl: string
): Promise<{
    realm: web3.PublicKey,
    mint: web3.PublicKey,
    councilMint: web3.PublicKey,
    realmAuthority: web3.Keypair,
    maker: web3.PublicKey,
    makerBump: number,
    collection: web3.PublicKey,
    collectionBump: number,
    metadata: web3.PublicKey,
    edition: web3.PublicKey,
    tokenAccount: web3.PublicKey
}> {
    const realmAuthority = await newSigner();
    const [mint, councilMint, realm] = await createRealm(realmAuthority);
    const makerSeeds = [Buffer.from("maker"), realm.toBytes(), Buffer.from(name)];
    const [maker, makerBump] = anchor.web3.PublicKey.findProgramAddressSync(makerSeeds, BRUSHO_NFT_MANAGER_PROGRAM.programId);
    const collectionSeeds = [Buffer.from("collection"), maker.toBytes()];
    const [collection, collectionBump] = anchor.web3.PublicKey.findProgramAddressSync(collectionSeeds, BRUSHO_NFT_MANAGER_PROGRAM.programId);
    const metadataSeeds = [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBytes(), collection.toBytes()];
    const [metadata,] = anchor.web3.PublicKey.findProgramAddressSync(metadataSeeds, TOKEN_METADATA_PROGRAM_ID);
    const editionSeeds = [Buffer.from("metadata"), TOKEN_METADATA_PROGRAM_ID.toBytes(), collection.toBytes(), Buffer.from("edition")];
    const [edition,] = anchor.web3.PublicKey.findProgramAddressSync(editionSeeds, TOKEN_METADATA_PROGRAM_ID);
    const tokenAccount = await getAssociatedTokenAddress(collection, maker, true);

    // console.log(`maker ${maker.toBase58()}`)
    // console.log(`collection ${collection.toBase58()}`)
    // console.log(`metadata ${metadata.toBase58()}`)
    // console.log(`edition ${edition.toBase58()}`)

    await BRUSHO_NFT_MANAGER_PROGRAM.methods
        .initializeMaker({ issuingAuthority, updateAuthority, name, metadataUrl })
        .accounts({
            payer: payer.publicKey,
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
        .signers([payer, realmAuthority])
        .rpc()

    return { realm, mint, councilMint, realmAuthority, maker, makerBump, collection, collectionBump, metadata, edition, tokenAccount }
}

describe("initialize_maker!", () => {
    it("verify_data", async () => {
        const authority = await newSigner();

        const name = "BrushO"
        const metadataUrl = "https://abcd.xyz/metadata";
        const { realm, mint, councilMint, realmAuthority, maker, makerBump, collection, collectionBump, metadata, edition, tokenAccount } =
            await createMaker(authority, authority.publicKey, authority.publicKey, name, metadataUrl);


        // Verify maker data
        const makerData = await BRUSHO_NFT_MANAGER_PROGRAM.account.maker.fetch(maker);
        assert.equal(realm.toBase58(), makerData.realm.toBase58())
        assert.equal(realmAuthority.publicKey.toBase58(), makerData.realmAuthority.toBase58())
        assert.equal(collection.toBase58(), makerData.collection.toBase58())
        assert.equal(anchor.web3.PublicKey.default.toBase58(), makerData.merkleTree.toBase58())
        assert.equal(authority.publicKey.toBase58(), makerData.updateAuthority.toBase58())
        assert.equal(authority.publicKey.toBase58(), makerData.issuingAuthority.toBase58())
        assert.equal(name, makerData.name)
        assert.equal(true, makerData.isActive)
        assert.equal(makerBump, makerData.bump)
        assert.equal(collectionBump, makerData.collectionBump)

        // Verify collection
        const collectionMint = await getMint(collection)
        assert.equal(0, collectionMint.decimals)
        assert.equal(edition.toBase58(), collectionMint.freezeAuthority.toBase58())
        assert.equal(edition.toBase58(), collectionMint.mintAuthority.toBase58())
        assert.equal("1", collectionMint.supply.toString())

        // Verify token account of collection
        const tokenAccountData = await getTokenAccount(tokenAccount);
        assert.equal(collection.toBase58(), tokenAccountData.mint.toBase58());
        assert.equal(maker.toBase58(), tokenAccountData.owner.toBase58());
        assert.equal("1", tokenAccountData.amount.toString());
    });

});


