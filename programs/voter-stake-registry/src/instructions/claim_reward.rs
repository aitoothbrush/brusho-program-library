use crate::events::ClaimRewardEvent;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use circuit_breaker::cpi::accounts::TransferV0;
use circuit_breaker::cpi::transfer_v0;
use circuit_breaker::AccountWindowedCircuitBreakerV0;
use circuit_breaker::CircuitBreaker;
use circuit_breaker::TransferArgsV0;

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    #[account(mut)]
    pub registrar: Box<Account<'info, Registrar>>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter.get_voter_bump(),
        constraint = voter.get_registrar() == registrar.key(),
        constraint = voter.get_voter_authority() == voter_authority.key(),
    )]
    pub voter: Box<Account<'info, Voter>>,
    pub voter_authority: Signer<'info>,

    #[account(
        mut,
        token::authority = circuit_breaker,
        token::mint = registrar.governing_token_mint,
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = ["account_windowed_breaker".as_bytes(), vault.key().as_ref()],
        seeds::program = circuit_breaker_program.key(),
        bump = circuit_breaker.bump_seed,
    )]
    pub circuit_breaker: Box<Account<'info, AccountWindowedCircuitBreakerV0>>,

    #[account(
        mut,
        token::mint = vault.mint,
    )]
    pub destination: Box<Account<'info, TokenAccount>>,

    pub circuit_breaker_program: Program<'info, CircuitBreaker>,

    pub token_program: Program<'info, Token>,
}

pub fn claim_reward(ctx: Context<ClaimReward>) -> Result<()> {
    // Load the accounts.
    let registrar = &mut ctx.accounts.registrar;
    let voter = &mut ctx.accounts.voter;

    // accrue rewards
    let curr_ts = registrar.clock_unix_timestamp();
    registrar.accrue_rewards(curr_ts);

    // claim reward
    let claimed_amount = voter.claim_reward(curr_ts, registrar)?;

    transfer_v0(
        CpiContext::new_with_signer(
            ctx.accounts.circuit_breaker_program.to_account_info(),
            TransferV0 {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                owner: registrar.to_account_info(),
                circuit_breaker: ctx.accounts.circuit_breaker.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
            &[&[
                registrar.realm.key().as_ref(),
                "registrar".as_ref(),
                registrar.governing_token_mint.key().as_ref(),
                &[registrar.bump],
            ]],
        ),
        TransferArgsV0 {
            amount: claimed_amount,
        },
    )?;

    emit!(ClaimRewardEvent {
        voter: voter.get_voter_authority(),
        amount: claimed_amount
    });

    Ok(())
}
