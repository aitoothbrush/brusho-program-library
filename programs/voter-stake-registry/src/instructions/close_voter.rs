use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, CloseAccount, Token, TokenAccount};

// Remaining accounts must be all the token token accounts owned by voter, he wants to close,
// they should be writable so that they can be closed and sol required for rent
// can then be sent back to the sol_destination
#[derive(Accounts)]
pub struct CloseVoter<'info> {
    pub registrar: Box<Account<'info, Registrar>>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [voter.get_registrar().key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter.get_voter_bump(),
        constraint = voter_authority.key() == voter.get_voter_authority(),
        close = sol_destination
    )]
    pub voter: Box<Account<'info, Voter>>,

    pub voter_authority: Signer<'info>,

    #[account(mut)]
    /// CHECK:
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

/// Closes the voter account (Optionally, also token vaults, as part of remaining_accounts),
/// allowing one to retrieve rent exemption SOL.
/// Only accounts with no remaining deposits can be closed.
pub fn close_voter<'info>(ctx: Context<'_, '_, 'info, 'info, CloseVoter<'info>>) -> Result<()> {
    {
        let voter = &ctx.accounts.voter;
        let amount = voter.amount_deposited_native();
        require_eq!(amount, 0, VsrError::GoverningTokenNonZero);
        require_eq!(voter.get_reward_claimable_amount(), 0, VsrError::GoverningTokenNonZero);

        // let voter_seeds = voter_seeds!(voter);
        // let voter_seeds =
        for account in ctx.remaining_accounts.iter() {
            let token = Account::<TokenAccount>::try_from(account).unwrap();
            require_keys_eq!(
                token.owner,
                ctx.accounts.voter.key(),
                VsrError::InvalidAuthority
            );
            require_eq!(token.amount, 0, VsrError::VaultTokenNonZero);

            let cpi_accounts = CloseAccount {
                account: account.to_account_info(),
                destination: ctx.accounts.sol_destination.to_account_info(),
                authority: ctx.accounts.voter.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            token::close_account(CpiContext::new_with_signer(
                cpi_program,
                cpi_accounts,
                &[voter_seeds!(voter)],
            ))?;

            account.exit(ctx.program_id)?;
        }
    }

    Ok(())
}
