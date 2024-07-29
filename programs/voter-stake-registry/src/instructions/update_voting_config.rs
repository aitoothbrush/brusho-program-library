use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

#[derive(Accounts)]
pub struct UpdateVotingConfig<'info> {
    #[account(
        mut, 
        has_one = governing_token_mint,
        has_one = realm_authority,
    )]
    pub registrar: Box<Account<'info, Registrar>>,
    pub governing_token_mint: Box<Account<'info, Mint>>,

    pub realm_authority: Signer<'info>,
}

/// Update voting configurations
pub fn update_voting_config(
    ctx: Context<UpdateVotingConfig>,
    voting_config: VotingConfig,
) -> Result<()> {
    require!(
        voting_config.lockup_saturation_secs > 0,
        VsrError::LockupSaturationMustBePositive
    );

    let registrar = &mut ctx.accounts.registrar;
    registrar.voting_config = voting_config;

    // Check for overflow in vote weight
    registrar.max_vote_weight(&ctx.accounts.governing_token_mint)?;

    Ok(())
}
