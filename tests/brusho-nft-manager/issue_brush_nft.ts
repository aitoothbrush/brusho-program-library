import * as anchor from "@coral-xyz/anchor";
import { web3 } from "@coral-xyz/anchor";

import { ACCOUNT_COMPRESSION_PROGRAM_ID, BRUSHO_NFT_MANAGER_PROGRAM, MPL_BUBBLEGUM_PROGRAM_ID, newSigner, NOOP_PROGRAM_ID, TOKEN_METADATA_PROGRAM_ID } from "../helper";
import { ValidDepthSizePair } from "@solana/spl-account-compression";
import { Keypair, Transaction } from "@solana/web3.js";
import { createMaker } from "./initialize_maker";
import { assert } from "chai";
import { setMakerTree } from "./set_maker_tree";

describe("issue_brush_nft!", () => {
    let payer: web3.Keypair;
    let makerAuthority: web3.Keypair;

    before(async () => {
        payer = await newSigner();
        makerAuthority = await newSigner();
    })

    it("verify_data", async () => {
        const issueNFTArgs = {
            brushNo: "0001-00222222-00001",
            metadataUrl: "http://abcd.xyz/nft/metadata/0001-00222222-00001",
        };
        const recipient = Keypair.generate().publicKey;

        const { realm, mint, councilMint, realmAuthority, maker, makerBump, collection, collectionBump, metadata, edition, tokenAccount } =
            await createMaker(payer, makerAuthority.publicKey, makerAuthority.publicKey, "BRUSH", "https://abcd.xyz/metadata");
        const depthSizePair: ValidDepthSizePair = { maxDepth: 3, maxBufferSize: 8 };
        const { merkleTree, treeAuthority } = await setMakerTree(payer, maker, makerAuthority, depthSizePair, 0);

        const topCreatorSeeds = [Buffer.from("top_creator"), realm.toBytes()];
        const [topCreator, ] = anchor.web3.PublicKey.findProgramAddressSync(topCreatorSeeds, BRUSHO_NFT_MANAGER_PROGRAM.programId);

        const brushNoToAssetSeeds = [Buffer.from("brush_no_to_asset"), realm.toBytes(), Buffer.from(issueNFTArgs.brushNo)];
        const [brushNoToAsset, brushNoToAssetBump ] = anchor.web3.PublicKey.findProgramAddressSync(brushNoToAssetSeeds, BRUSHO_NFT_MANAGER_PROGRAM.programId);

        const bubblegumSignertSeeds = [Buffer.from("collection_cpi")];
        const [bubblegumSigner, ] = anchor.web3.PublicKey.findProgramAddressSync(bubblegumSignertSeeds, MPL_BUBBLEGUM_PROGRAM_ID);

        // console.log(`topCreator: ${topCreator.toString()}`)
        // console.log(`brushNoToAsset: ${brushNoToAsset.toString()}`)
        // console.log(`bubblegumSigner: ${bubblegumSigner.toString()}`)

        await BRUSHO_NFT_MANAGER_PROGRAM.methods
            .issueBrushNft(issueNFTArgs)
            .accounts({
                payer: payer.publicKey,
                issuingAuthority: makerAuthority.publicKey,
                collection,
                collectionMetadata: metadata,
                collectionMasterEdition: edition,
                maker,
                realm,
                topCreator,
                brushNoToAsset,
                treeAuthority,
                recipient,
                merkleTree,
                bubblegumSigner,
                tokenMetadataProgram: TOKEN_METADATA_PROGRAM_ID,
                logWrapper: NOOP_PROGRAM_ID,
                bubblegumProgram: MPL_BUBBLEGUM_PROGRAM_ID,
                compressionProgram: ACCOUNT_COMPRESSION_PROGRAM_ID,
            })
            .signers([payer, makerAuthority])
            .rpc()

        const brushNoToAssetData = await BRUSHO_NFT_MANAGER_PROGRAM.account.brushNoToAsset.fetch(brushNoToAsset);
        // console.log(JSON.stringify(brushNoToAssetData, undefined, 2))

        assert.equal(realm.toString(), brushNoToAssetData.realm.toString());
        assert.equal(issueNFTArgs.brushNo, brushNoToAssetData.brushNo);
        assert.equal(brushNoToAssetBump, brushNoToAssetData.bump);
    });

});


