use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

// Remaining accounts should all the token mints that have registered
// exchange rates.
#[derive(Accounts)]
pub struct UpdateMaxVoteWeight<'info> {
    pub registrar: Box<Account<'info, Registrar>>,
    /// Registrar.realm_governing_token_mint
    pub governing_token_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = [registrar.realm.key().as_ref(), b"max-voter-weight-record".as_ref(), registrar.governing_token_mint.key().as_ref()],
        bump = registrar.max_voter_weight_record_bump,
        constraint = max_voter_weight_record.realm == registrar.realm,
        constraint = max_voter_weight_record.governing_token_mint == registrar.governing_token_mint,
    )]
    pub max_voter_weight_record: Account<'info, MaxVoterWeightRecord>,
}

/// Calculates the max vote weight for the registry. This is a function
/// of the total supply of all exchange rate mints, converted into a
/// common currency with a common number of decimals.
///
/// Note that this method is only safe to use if the cumulative supply for
/// all tokens fits into a u64 *after* converting into common decimals, as
/// defined by the registrar's `rate_decimal` field.
pub fn update_max_vote_weight(ctx: Context<UpdateMaxVoteWeight>) -> Result<()> {
    let registrar = &ctx.accounts.registrar;

    let record = &mut ctx.accounts.max_voter_weight_record;
    record.max_voter_weight = registrar.max_vote_weight(&ctx.accounts.governing_token_mint)?;
    record.max_voter_weight_expiry = Some(Clock::get()?.slot);

    Ok(())
}
