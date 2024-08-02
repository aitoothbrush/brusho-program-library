use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateVoterWeightRecord<'info> {
    pub registrar: Box<Account<'info, Registrar>>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter.get_voter_authority().key().as_ref()],
        bump = voter.get_voter_bump(),
        constraint = registrar.key() == voter.get_registrar(),
    )]
    pub voter: Box<Account<'info, Voter>>,

    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter-weight-record".as_ref(), voter.get_voter_authority().key().as_ref()],
        bump = voter.get_voter_weight_record_bump(),
        constraint = voter_weight_record.realm == registrar.realm,
        constraint = voter_weight_record.governing_token_owner == voter.get_voter_authority(),
        constraint = voter_weight_record.governing_token_mint == registrar.governing_token_mint,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    pub system_program: Program<'info, System>,
}

/// Calculates the lockup-scaled, time-decayed voting power for the given
/// voter and writes it into a `VoteWeightRecord` account to be used by
/// the SPL governance program.
///
/// This "revise" instruction must be called immediately before voting, in
/// the same transaction.
pub fn update_voter_weight_record(ctx: Context<UpdateVoterWeightRecord>) -> Result<()> {
    let registrar = &ctx.accounts.registrar;
    let voter = &ctx.accounts.voter;
    let record = &mut ctx.accounts.voter_weight_record;
    let curr_ts = registrar.clock_unix_timestamp();
    record.voter_weight = voter.weight(curr_ts, registrar)?;
    record.voter_weight_expiry = Some(Clock::get()?.slot);

    Ok(())
}
