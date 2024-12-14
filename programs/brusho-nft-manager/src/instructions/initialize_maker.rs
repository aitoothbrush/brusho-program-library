use crate::error::BnmError;
use crate::CreateMetadataAccountsV3;
use crate::{
    create_master_edition_v3, create_metadata_accounts_v3, state::*, CreateMasterEditionV3,
    Metadata,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount},
};
use mpl_token_metadata::types::{CollectionDetails, DataV2};
use spl_governance::state::realm::get_realm_data;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct InitializeMakerArgs {
    pub update_authority: Pubkey,
    pub issuing_authority: Pubkey,
    pub name: String,
    pub metadata_url: String,
}

#[derive(Accounts)]
#[instruction(args: InitializeMakerArgs)]
pub struct InitializeMaker<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<Maker>() + 1 + 17,
        seeds = ["maker".as_bytes(), realm.key().as_ref(), args.name.as_bytes()],
        bump,
    )]
    pub maker: Box<Account<'info, Maker>>,
    /// An spl-governance realm
    ///
    /// realm is validated in the instruction:
    /// - realm is owned by the governance_program_id
    /// - realm_authority is realm.authority
    /// CHECK:
    pub realm: UncheckedAccount<'info>,
    /// The program id of the spl-governance program the realm belongs to.
    /// CHECK:
    pub governance_program_id: UncheckedAccount<'info>,
    pub realm_authority: Signer<'info>,
    #[account(
        init,
        payer = payer,
        mint::decimals = 0,
        mint::authority = maker,
        mint::freeze_authority = maker,
        seeds = ["collection".as_bytes(), maker.key().as_ref()],
        bump
    )]
    pub collection: Box<Account<'info, Mint>>,
    /// CHECK: Handled by cpi
    #[account(
        mut,
        seeds = ["metadata".as_bytes(), token_metadata_program.key().as_ref(), collection.key().as_ref()],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: Handled by cpi
    #[account(
        mut,
        seeds = ["metadata".as_bytes(), token_metadata_program.key().as_ref(), collection.key().as_ref(), "edition".as_bytes()],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub master_edition: UncheckedAccount<'info>,
    #[account(
        init,
        payer = payer,
        associated_token::mint = collection,
        associated_token::authority = maker,
    )]
    pub token_account: Box<Account<'info, TokenAccount>>,

    pub token_metadata_program: Program<'info, Metadata>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> InitializeMaker<'info> {
    fn mint_ctx(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        let cpi_accounts = MintTo {
            mint: self.collection.to_account_info(),
            to: self.token_account.to_account_info(),
            authority: self.maker.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

pub fn initialize_maker(ctx: Context<InitializeMaker>, args: InitializeMakerArgs) -> Result<()> {
    require!(args.name.len() <= 17, BnmError::InvalidMakerNameLength);
    require!(
        args.metadata_url.len() <= 200,
        BnmError::InvaliMetadataUrlLength
    );

    // Verify that "realm_authority" is the expected authority on "realm"
    let realm = get_realm_data(
        &ctx.accounts.governance_program_id.key(),
        &ctx.accounts.realm,
    )?;
    require_keys_eq!(
        realm.authority.unwrap(),
        ctx.accounts.realm_authority.key(),
        BnmError::InvalidRealmAuthority
    );

    let maker = Maker {
        realm: ctx.accounts.realm.key(),
        realm_authority: ctx.accounts.realm_authority.key(),
        collection: ctx.accounts.collection.key(),
        merkle_tree: Pubkey::default(),
        issuing_authority: args.issuing_authority,
        update_authority: args.update_authority,
        name: args.name.clone(),
        is_active: true,
        bump: ctx.bumps.maker,
        collection_bump: ctx.bumps.collection,
        reserved1: [0; 5],
        reserved2: [0; 8],
    };
    let signer_seeds: &[&[&[u8]]] = &[maker_seeds!(&maker)];

    token::mint_to(ctx.accounts.mint_ctx().with_signer(signer_seeds), 1)?;

    create_metadata_accounts_v3(
        CpiContext::new_with_signer(
            ctx.accounts
                .token_metadata_program
                .to_account_info()
                .clone(),
            CreateMetadataAccountsV3 {
                metadata: ctx.accounts.metadata.to_account_info().clone(),
                mint: ctx.accounts.collection.to_account_info().clone(),
                mint_authority: ctx.accounts.maker.to_account_info().clone(),
                payer: ctx.accounts.payer.to_account_info().clone(),
                update_authority: ctx.accounts.maker.to_account_info().clone(),
                system_program: ctx.accounts.system_program.to_account_info().clone(),
                token_metadata_program: ctx.accounts.token_metadata_program.clone(),
            },
            signer_seeds,
        ),
        DataV2 {
            name: args.name.clone(),
            symbol: "MAKER".to_string(),
            uri: args.metadata_url,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        true,
        Some(CollectionDetails::V1 { size: 0 }),
    )?;

    create_master_edition_v3(
        CpiContext::new_with_signer(
            ctx.accounts
                .token_metadata_program
                .to_account_info()
                .clone(),
            CreateMasterEditionV3 {
                edition: ctx.accounts.master_edition.to_account_info().clone(),
                mint: ctx.accounts.collection.to_account_info().clone(),
                update_authority: ctx.accounts.maker.to_account_info().clone(),
                mint_authority: ctx.accounts.maker.to_account_info().clone(),
                metadata: ctx.accounts.metadata.to_account_info().clone(),
                payer: ctx.accounts.payer.to_account_info().clone(),
                token_program: ctx.accounts.token_program.to_account_info().clone(),
                system_program: ctx.accounts.system_program.to_account_info().clone(),
                token_metadata_program: ctx.accounts.token_metadata_program.clone(),
            },
            signer_seeds,
        ),
        Some(0),
    )?;

    ctx.accounts.maker.set_inner(maker);

    Ok(())
}
