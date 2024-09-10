use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::Mint;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use std::mem::size_of;
use std::ops::DerefMut;

#[derive(Accounts)]
pub struct CreateVoter<'info> {
    #[account(mut)]
    pub registrar: AccountLoader<'info, Registrar>,
    pub governing_token_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + size_of::<Voter>(),
    )]
    pub voter: AccountLoader<'info, Voter>,

    /// The authority controling the voter. Must be the same as the
    /// `governing_token_owner` in the token owner record used with
    /// spl-governance.
    pub voter_authority: Signer<'info>,

    #[account(
        init_if_needed,
        associated_token::authority = voter,
        associated_token::mint = governing_token_mint,
        payer = payer
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    /// The voter weight record is the account that will be shown to spl-governance
    /// to prove how much vote weight the voter has. See update_voter_weight_record.
    #[account(
        init,
        seeds = [registrar.key().as_ref(), b"voter-weight-record".as_ref(), voter_authority.key().as_ref()],
        bump,
        payer = payer,
        space = size_of::<VoterWeightRecord>(),
    )]
    pub voter_weight_record: Box<Account<'info, VoterWeightRecord>>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
}

/// Creates a new voter account. There can only be a single voter per
/// voter_authority.
///
/// The user must register with spl-governance using the same voter_authority.
/// Their token owner record will be required for withdrawing funds later.
pub fn create_voter(
    ctx: Context<CreateVoter>,
    voter_bump: u8,
    voter_weight_record_bump: u8,
) -> Result<()> {
    require_eq!(voter_bump, ctx.bumps.voter);
    require_eq!(voter_weight_record_bump, ctx.bumps.voter_weight_record);

    let registrar = &mut ctx.accounts.registrar.load_mut()?;
    let voter_authority = ctx.accounts.voter_authority.key();

    // accrue rewards
    let curr_ts = registrar.clock_unix_timestamp();
    registrar.accrue_rewards(curr_ts);

    let voter = &mut ctx.accounts.voter.load_init()?;
    (*voter.deref_mut()) = Voter::new(
        voter_authority,
        ctx.accounts.registrar.key(),
        registrar.reward_index,
        voter_bump,
        voter_weight_record_bump,
    );

    let voter_weight_record = &mut ctx.accounts.voter_weight_record;
    voter_weight_record.account_discriminator =
        spl_governance_addin_api::voter_weight::VoterWeightRecord::ACCOUNT_DISCRIMINATOR;
    voter_weight_record.realm = registrar.realm;
    voter_weight_record.governing_token_mint = registrar.governing_token_mint;
    voter_weight_record.governing_token_owner = voter_authority;

    Ok(())
}
