import { web3 } from "@coral-xyz/anchor";

import { BRUSHO_NFT_MANAGER_PROGRAM, newSigner } from "../helper";
import { Keypair } from "@solana/web3.js";
import { createMaker } from "./initialize_maker";
import { assert } from "chai";

describe("update_maker!", () => {
    let payer: web3.Keypair;
    let makerAuthority: web3.Keypair;
    let maker_: web3.PublicKey;
    let realmAuthority_: web3.Keypair;

    before(async () => {
        payer = await newSigner();
        makerAuthority = await newSigner();

        const { realm, mint, councilMint, realmAuthority, maker, makerBump, collection, collectionBump, metadata, edition, tokenAccount } =
            await createMaker(payer, makerAuthority.publicKey, makerAuthority.publicKey, "BRUSH", "https://abcd.xyz/metadata");
        maker_ = maker;
        realmAuthority_ = realmAuthority;
    })

    it("update_issuing_authority", async () => {
        const newIssuingAuthority = Keypair.generate().publicKey;

        await BRUSHO_NFT_MANAGER_PROGRAM.methods
            .updateIssuingAuthority({ issuingAuthority: newIssuingAuthority })
            .accounts({
                maker: maker_,
                updateAuthority: makerAuthority.publicKey
            })
            .signers([makerAuthority])
            .rpc()

        const makerData = await BRUSHO_NFT_MANAGER_PROGRAM.account.maker.fetch(maker_);
        assert.equal(newIssuingAuthority.toBase58(), makerData.issuingAuthority.toBase58())
    });

    it("update_maker", async () => {
        const newUpdateAuthority = Keypair.generate().publicKey;

        await BRUSHO_NFT_MANAGER_PROGRAM.methods
            .updateMaker({ updateAuthority: newUpdateAuthority, isActive: false })
            .accounts({
                maker: maker_,
                realmAuthority: realmAuthority_.publicKey,
            })
            .signers([realmAuthority_])
            .rpc()

        const makerData = await BRUSHO_NFT_MANAGER_PROGRAM.account.maker.fetch(maker_);
        assert.equal(newUpdateAuthority.toBase58(), makerData.updateAuthority.toBase58())
        assert.equal(false, makerData.isActive)
    });
});


