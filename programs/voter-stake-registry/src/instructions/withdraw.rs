use crate::error::*;
use crate::events::WithdrawEvent;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub registrar: AccountLoader<'info, Registrar>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter_authority.key().as_ref()],
        bump = voter.load()?.get_voter_bump(),
        constraint = voter.load()?.get_registrar() == registrar.key(),
        constraint = voter.load()?.get_voter_authority() == voter_authority.key(),
    )]
    pub voter: AccountLoader<'info, Voter>,
    pub voter_authority: Signer<'info>,

    /// The token_owner_record for the voter_authority. This is needed
    /// to be able to forbid withdraws while the voter is engaged with
    /// a vote or has an open proposal.
    ///
    /// token_owner_record is validated in the instruction:
    /// - owned by registrar.governance_program_id
    /// - for the registrar.realm
    /// - for the registrar.realm_governing_token_mint
    /// - governing_token_owner is voter_authority
    /// CHECK: verified while loading data
    pub token_owner_record: UncheckedAccount<'info>,

    /// Withdraws must update the voter weight record, to prevent a stale
    /// record being used to vote after the withdraw.
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter-weight-record".as_ref(), voter_authority.key().as_ref()],
        bump = voter.load()?.get_voter_weight_record_bump(),
        constraint = voter_weight_record.realm == registrar.load()?.realm,
        constraint = voter_weight_record.governing_token_owner == voter.load()?.get_voter_authority(),
        constraint = voter_weight_record.governing_token_mint == registrar.load()?.governing_token_mint,
    )]
    pub voter_weight_record: Account<'info, VoterWeightRecord>,

    #[account(
        mut,
        associated_token::authority = voter,
        associated_token::mint = registrar.load()?.governing_token_mint,
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        token::mint = vault.mint,
    )]
    pub destination: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Withdraw<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.vault.to_account_info(),
            to: self.destination.to_account_info(),
            authority: self.voter.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

/// Withdraws tokens from a deposit entry, if they are unlocked according
/// to the deposit's vesting schedule.
///
/// `deposit_entry_index`: The deposit entry to withdraw from.
/// `amount` is in units of the native currency being withdrawn.
pub fn withdraw(ctx: Context<Withdraw>, deposit_entry_index: u8, amount: u64) -> Result<()> {
    {
        // Transfer the tokens to withdraw.
        let voter = &ctx.accounts.voter.load()?;
        require!(
            voter.is_active(deposit_entry_index)?,
            VsrError::InactiveDepositEntry
        );

        token::transfer(
            ctx.accounts
                .transfer_ctx()
                .with_signer(&[voter_seeds!(voter)]),
            amount,
        )?;
    }

    // Load the accounts.
    let registrar = &mut ctx.accounts.registrar.load_mut()?;
    let voter = &mut ctx.accounts.voter.load_mut()?;

    // Governance may forbid withdraws, for example when engaged in a vote.
    let token_owner_record = load_token_owner_record(
        &ctx.accounts.token_owner_record.to_account_info(),
        voter,
        registrar,
    )?;
    token_owner_record.assert_can_withdraw_governing_tokens()?;

    // accrue rewards
    let curr_ts = registrar.clock_unix_timestamp();
    registrar.accrue_rewards(curr_ts);

    let entry_amount_deposited_native =
        voter.withdraw(deposit_entry_index, curr_ts, amount, registrar)?;

    // Deactivate deposit entry if no funds remains.
    if entry_amount_deposited_native == 0 {
        voter.deactivate(deposit_entry_index, curr_ts, registrar)?;
    }

    // Update the voter weight record
    let record = &mut ctx.accounts.voter_weight_record;
    record.voter_weight = voter.weight(curr_ts, registrar)?;
    record.voter_weight_expiry = Some(Clock::get()?.slot);

    emit!(WithdrawEvent {
        voter: voter.get_voter_authority(),
        deposit_entry_index,
        amount,
    });

    Ok(())
}

pub fn load_token_owner_record(
    account_info: &AccountInfo,
    voter: &Voter,
    registrar: &Registrar,
) -> Result<spl_governance::state::token_owner_record::TokenOwnerRecordV2> {
    let record = spl_governance::state::token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint(
        &registrar.governance_program_id,
        account_info,
        &registrar.realm,
        &registrar.governing_token_mint,
    )?;
    require_keys_eq!(
        record.governing_token_owner,
        voter.get_voter_authority(),
        VsrError::InvalidTokenOwnerRecord
    );
    Ok(record)
}
