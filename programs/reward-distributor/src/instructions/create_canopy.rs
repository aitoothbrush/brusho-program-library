use crate::{canopy::check_canopy_bytes, error::RdError, id, state::*};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct CreateCanopy<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<Canopy>(),
        seeds = ["canopy".as_bytes(), &canopy_data.key().to_bytes()],
        bump,
    )]
    pub canopy: Box<Account<'info, Canopy>>,

    #[account(
      owner = id(),
      constraint = check_canopy_bytes(&canopy_data.data.borrow()).is_ok() @ RdError::InvalidCanopyLength,
    )]
    /// CHECK: Account to store the canopy data
    pub canopy_data: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn create_canopy(ctx: Context<CreateCanopy>) -> Result<()> {
    ctx.accounts.canopy.set_inner(Canopy {
        canopy_data: ctx.accounts.canopy_data.key(),
        authority: ctx.accounts.authority.key()
    });

    Ok(())
}
