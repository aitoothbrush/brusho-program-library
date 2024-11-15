use anchor_lang::prelude::*;
use mpl_token_metadata::{
  instructions::{
    CreateMasterEditionV3Cpi, CreateMasterEditionV3CpiAccounts,
    CreateMasterEditionV3InstructionArgs, CreateMetadataAccountV3Cpi,
    CreateMetadataAccountV3CpiAccounts, CreateMetadataAccountV3InstructionArgs,
  },
  types::{CollectionDetails, DataV2},
  ID,
};

#[derive(Accounts)]
pub struct CreateMetadataAccountsV3<'info> {
  /// CHECK: via cpi
  pub metadata: AccountInfo<'info>,
  /// CHECK: via cpi
  pub mint: AccountInfo<'info>,
  /// CHECK: via cpi
  pub mint_authority: AccountInfo<'info>,
  /// CHECK: via cpi
  pub payer: AccountInfo<'info>,
  /// CHECK: via cpi
  pub update_authority: AccountInfo<'info>,
  /// CHECK: via cpi
  pub system_program: AccountInfo<'info>,
  pub token_metadata_program: Program<'info, Metadata>,
}

pub fn create_metadata_accounts_v3<'info>(
  ctx: CpiContext<'_, '_, '_, 'info, CreateMetadataAccountsV3<'info>>,
  data: DataV2,
  is_mutable: bool,
  details: Option<CollectionDetails>,
) -> Result<()> {
  let cpi = CreateMetadataAccountV3Cpi::new(
    &ctx.accounts.token_metadata_program,
    CreateMetadataAccountV3CpiAccounts {
      metadata: &ctx.accounts.metadata,
      mint: &ctx.accounts.mint,
      mint_authority: &ctx.accounts.mint_authority,
      payer: &ctx.accounts.payer,
      update_authority: (
        &ctx.accounts.update_authority,
        ctx.accounts.update_authority.is_signer,
      ),
      system_program: &ctx.accounts.system_program,
      rent: None,
    },
    CreateMetadataAccountV3InstructionArgs {
      data,
      is_mutable,
      collection_details: details,
    },
  );
  if ctx.accounts.update_authority.is_signer {
    cpi
      .invoke_signed_with_remaining_accounts(
        ctx.signer_seeds,
        &[(&ctx.accounts.update_authority, true, false)],
      )
      .map_err(Into::into)
  } else {
    cpi.invoke_signed(ctx.signer_seeds).map_err(Into::into)
  }
}

#[derive(Clone)]
pub struct Metadata;

impl anchor_lang::Id for Metadata {
  fn id() -> Pubkey {
    ID
  }
}

#[derive(Accounts)]
pub struct CreateMasterEditionV3<'info> {
  /// CHECK: via cpi
  pub edition: AccountInfo<'info>,
  /// CHECK: via cpi
  pub mint: AccountInfo<'info>,
  /// CHECK: via cpi
  pub update_authority: AccountInfo<'info>,
  /// CHECK: via cpi
  pub mint_authority: AccountInfo<'info>,
  /// CHECK: via cpi
  pub payer: AccountInfo<'info>,
  /// CHECK: via cpi
  pub metadata: AccountInfo<'info>,
  /// CHECK: via cpi
  pub token_program: AccountInfo<'info>,
  /// CHECK: via cpi
  pub system_program: AccountInfo<'info>,
  pub token_metadata_program: Program<'info, Metadata>,
}

pub fn create_master_edition_v3<'info>(
  ctx: CpiContext<'_, '_, '_, 'info, CreateMasterEditionV3<'info>>,
  max_supply: Option<u64>,
) -> Result<()> {
  CreateMasterEditionV3Cpi::new(
    &ctx.accounts.token_metadata_program,
    CreateMasterEditionV3CpiAccounts {
      edition: &ctx.accounts.edition,
      mint: &ctx.accounts.mint,
      update_authority: &ctx.accounts.update_authority,
      mint_authority: &ctx.accounts.mint_authority,
      payer: &ctx.accounts.payer,
      metadata: &ctx.accounts.metadata,
      token_program: &ctx.accounts.token_program,
      system_program: &ctx.accounts.system_program,
      rent: None,
    },
    CreateMasterEditionV3InstructionArgs { max_supply },
  )
  .invoke_signed(ctx.signer_seeds)
  .map_err(Into::into)
}