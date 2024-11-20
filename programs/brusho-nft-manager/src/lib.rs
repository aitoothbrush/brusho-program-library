use anchor_lang::prelude::*;

declare_id!("BrMwmWYxuAiCMJXf1yPAsp5WZ46e6ZgYC3c8gmkRq3nE");

// pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod token_metadata;

use instructions::*;
use token_metadata::*;

#[program]
pub mod brusho_nft_manager {
    use super::*;

    pub fn initialize_maker(
        ctx: Context<InitializeMaker>,
        args: InitializeMakerArgs,
    ) -> Result<()> {
        instructions::initialize_maker(ctx, args)
    }

    pub fn set_maker_tree(ctx: Context<SetMakerTree>, args: SetMakerTreeArgs) -> Result<()> {
        instructions::set_maker_tree(ctx, args)
    }

    pub fn issue_brush_nft(ctx: Context<IssueBrushNft>, args: IssueBrushNftArgs) -> Result<()> {
        instructions::issue_brush_nft(ctx, args)
    }

    pub fn update_issuing_authority(ctx: Context<UpdateIssuingAuthority>, args: UpdateIssuingAuthorityArgs) -> Result<()> {
        instructions::update_issuing_authority(ctx, args)
    }

    pub fn update_maker(ctx: Context<UpdateMaker>, args: UpdateMakerArgs) -> Result<()> {
        instructions::update_maker(ctx, args)
    }
}
