use crate::{error::BnmError, state::*};
use account_compression_cpi::{program::SplAccountCompression, Noop};
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
use bubblegum_cpi::{
    cpi::{accounts::MintToCollectionV1, mint_to_collection_v1},
    get_asset_id,
    program::Bubblegum,
    Collection, Creator, MetadataArgs, TokenProgramVersion, TokenStandard, TreeConfig,
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct IssueBrushNftArgs {
    pub brush_no: String,
    pub metadata_url: String,
}

#[derive(Accounts)]
#[instruction(args: IssueBrushNftArgs)]
pub struct IssueBrushNft<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub issuing_authority: Signer<'info>,
    pub collection: Box<Account<'info, Mint>>,

    /// CHECK: Handled by cpi
    #[account(
        mut,
        seeds = ["metadata".as_bytes(), token_metadata_program.key().as_ref(), collection.key().as_ref()],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub collection_metadata: UncheckedAccount<'info>,

    /// CHECK: Handled By cpi account
    #[account(
        seeds = ["metadata".as_bytes(), token_metadata_program.key().as_ref(), collection.key().as_ref(), "edition".as_bytes()],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub collection_master_edition: UncheckedAccount<'info>,

    #[account(
        has_one = issuing_authority,
        has_one = collection,
        has_one = merkle_tree,
        has_one = realm,
        constraint = maker.is_active == true @ BnmError::InactiveMaker,
    )]
    pub maker: Box<Account<'info, Maker>>,
    /// CHECK: via maker
    pub realm: UncheckedAccount<'info>,
    /// CHECK: Signs as a verified creator to make searching easier
    #[account(
        seeds = [b"top_creator", realm.key().as_ref()],
        bump,
    )]
    pub top_creator: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<BrushNoToAsset>() + 1 + 14,
        seeds = [
            "brush_no_to_asset".as_bytes(),
            realm.key().as_ref(),
            args.brush_no.as_bytes(),
        ],
        bump
    )]
    pub brush_no_to_asset: Box<Account<'info, BrushNoToAsset>>,
    #[account(
        mut,
        seeds = [merkle_tree.key().as_ref()],
        seeds::program = bubblegum_program.key(),
        bump,
    )]
    pub tree_authority: Box<Account<'info, TreeConfig>>,
    /// CHECK: Used in cpi
    pub recipient: AccountInfo<'info>,
    /// CHECK: Used in cpi
    #[account(mut)]
    pub merkle_tree: AccountInfo<'info>,
    #[account(
        seeds = ["collection_cpi".as_bytes()],
        seeds::program = bubblegum_program.key(),
        bump,
    )]
    /// CHECK: Used in cpi
    pub bubblegum_signer: UncheckedAccount<'info>,

    /// CHECK: Verified by constraint  
    #[account(address = mpl_token_metadata::ID)]
    pub token_metadata_program: AccountInfo<'info>,
    pub log_wrapper: Program<'info, Noop>,
    pub bubblegum_program: Program<'info, Bubblegum>,
    pub compression_program: Program<'info, SplAccountCompression>,
    pub system_program: Program<'info, System>,
}

impl<'info> IssueBrushNft<'info> {
    fn mint_to_collection_ctx(&self) -> CpiContext<'_, '_, '_, 'info, MintToCollectionV1<'info>> {
        let cpi_accounts = MintToCollectionV1 {
            tree_authority: self.tree_authority.to_account_info(),
            leaf_delegate: self.recipient.to_account_info(),
            leaf_owner: self.recipient.to_account_info(),
            merkle_tree: self.merkle_tree.to_account_info(),
            payer: self.payer.to_account_info(),
            tree_delegate: self.maker.to_account_info(),
            log_wrapper: self.log_wrapper.to_account_info(),
            compression_program: self.compression_program.to_account_info(),
            system_program: self.system_program.to_account_info(),
            collection_authority: self.maker.to_account_info(),
            collection_authority_record_pda: self.bubblegum_program.to_account_info(),
            collection_mint: self.collection.to_account_info(),
            collection_metadata: self.collection_metadata.to_account_info(),
            edition_account: self.collection_master_edition.to_account_info(),
            bubblegum_signer: self.bubblegum_signer.to_account_info(),
            token_metadata_program: self.token_metadata_program.to_account_info(),
        };
        CpiContext::new(self.bubblegum_program.to_account_info(), cpi_accounts)
    }
}

pub fn issue_brush_nft(ctx: Context<IssueBrushNft>, args: IssueBrushNftArgs) -> Result<()> {
    require!(args.brush_no.len() <= 14, BnmError::InvalidBrushNoLength);

    let asset_id = get_asset_id(
        &ctx.accounts.merkle_tree.key(),
        ctx.accounts.tree_authority.num_minted,
    );

    ctx.accounts.brush_no_to_asset.set_inner(BrushNoToAsset {
        asset: asset_id,
        realm: ctx.accounts.realm.key(),
        brush_no: args.brush_no.clone(),
        bump: ctx.bumps.brush_no_to_asset,
    });

    let maker = &ctx.accounts.maker;
    let name = format!("BrushO#{}", args.brush_no.clone());

    let metadata = MetadataArgs {
        name,
        symbol: String::from("BRUSH"),
        uri: args.metadata_url,
        collection: Some(Collection {
            key: ctx.accounts.collection.key(),
            verified: false, // Verified in cpi
        }),
        primary_sale_happened: true,
        is_mutable: true,
        edition_nonce: None,
        token_standard: Some(TokenStandard::NonFungible),
        uses: None,
        token_program_version: TokenProgramVersion::Original,
        creators: vec![
            Creator {
                address: ctx.accounts.top_creator.key(),
                verified: true,
                share: 100,
            },
            Creator {
                address: ctx.accounts.brush_no_to_asset.key(),
                verified: true,
                share: 0,
            },
        ],
        seller_fee_basis_points: 0,
    };

    let top_creator_signer_seeds: &[&[u8]] = &[
        b"top_creator",
        ctx.accounts.realm.to_account_info().key.as_ref(),
        &[ctx.bumps.top_creator],
    ];

    let maker_signer_seeds: &[&[u8]] = maker_seeds!(maker);
    let mut creator = ctx.accounts.top_creator.to_account_info();
    creator.is_signer = true;
    let mut brush_no_to_asset_creator = ctx.accounts.brush_no_to_asset.to_account_info();
    brush_no_to_asset_creator.is_signer = true;
    let brush_no_to_asset_signer_seeds: &[&[u8]] =
        brush_no_to_asset_seeds!(ctx.accounts.brush_no_to_asset);
    mint_to_collection_v1(
        ctx.accounts
            .mint_to_collection_ctx()
            .with_remaining_accounts(vec![creator, brush_no_to_asset_creator])
            .with_signer(&[
                maker_signer_seeds,
                top_creator_signer_seeds,
                brush_no_to_asset_signer_seeds,
            ]),
        metadata,
    )?;

    Ok(())
}
