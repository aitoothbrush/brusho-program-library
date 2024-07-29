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
    // TODO: SPL governance has not yet implemented this.
    /// CHECK:
    pub max_vote_weight_record: UncheckedAccount<'info>,
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
    let _max_vote_weight = registrar.max_vote_weight(&ctx.accounts.governing_token_mint)?;
    // TODO: SPL governance has not yet implemented this feature.
    //       When it has, probably need to write the result into an account,
    //       similar to VoterWeightRecord.
    Ok(())
}
