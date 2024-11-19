use crate::state::*;
use anchor_lang::{prelude::*, solana_program::system_program};

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
    #[account(mut)]
    /// CHECK: Just receiving funds
    pub refund: UncheckedAccount<'info>,

    #[account(
      has_one = authority,
      has_one = canopy
    )]
    pub distribution_tree: Box<Account<'info, DistributionTree>>,

    pub authority: Signer<'info>,

    /// CHECK: Verified by has one
    #[account(mut)]
    pub canopy: UncheckedAccount<'info>,
}

pub fn close_canopy(ctx: Context<CloseCanopy>) -> Result<()> {
    close(
        ctx.accounts.canopy.to_account_info(),
        ctx.accounts.refund.to_account_info(),
    )?;
    Ok(())
}
