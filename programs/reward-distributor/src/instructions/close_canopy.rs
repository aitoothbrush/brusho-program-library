use crate::{error::RdError, state::*};
use anchor_lang::{prelude::*, system_program};

pub fn close<'info>(info: AccountInfo<'info>, sol_destination: AccountInfo<'info>) -> Result<()> {
    // Transfer tokens from the account to the sol_destination.
    let dest_starting_lamports = sol_destination.lamports();
    **sol_destination.lamports.borrow_mut() =
        dest_starting_lamports.checked_add(info.lamports()).unwrap();
    **info.lamports.borrow_mut() = 0;

    info.assign(&system_program::ID);
    info.realloc(0, false).map_err(Into::into)
}

#[derive(Accounts)]
pub struct CloseCanopy<'info> {
    #[account(
        mut,
        has_one = authority @ RdError::Authorization,
        has_one = canopy_data,
    )]
    pub canopy: Box<Account<'info, Canopy>>,

    #[account(mut)]
    /// CHECK: see canopy constraints 
    pub canopy_data: UncheckedAccount<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn close_canopy(ctx: Context<CloseCanopy>) -> Result<()> {
    close(
        ctx.accounts.canopy_data.to_account_info(),
        ctx.accounts.authority.to_account_info(),
    )?;

    close(
        ctx.accounts.canopy.to_account_info(),
        ctx.accounts.authority.to_account_info(),
    )?;

    Ok(())
}
