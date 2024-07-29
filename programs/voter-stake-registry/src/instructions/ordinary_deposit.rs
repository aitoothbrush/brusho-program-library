use crate::{error::VsrError, state::*, NODE_DEPOSIT_ENTRY_INDEX};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

#[derive(Accounts)]
pub struct OrdinaryDeposit<'info> {
    pub registrar: Box<Account<'info, Registrar>>,

    // checking the PDA address it just an extra precaution,
    // the other constraints must be exhaustive
    #[account(
        mut,
        seeds = [registrar.key().as_ref(), b"voter".as_ref(), voter.get_voter_authority().key().as_ref()],
        bump = voter.get_voter_bump(),
        constraint = voter.get_registrar() == registrar.key()
    )]
    pub voter: Box<Account<'info, Voter>>,

    #[account(
        mut,
        associated_token::authority = voter,
        associated_token::mint = registrar.governing_token_mint,
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::authority = deposit_authority,
        associated_token::mint = vault.mint,
    )]
    pub deposit_token: Box<Account<'info, TokenAccount>>,
    pub deposit_authority: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> OrdinaryDeposit<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.deposit_token.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.deposit_authority.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

/// Adds tokens to a ordinary deposit entry.
///
/// Tokens will be transfered from deposit_token to vault using the deposit_authority.
///
/// `deposit_entry_index`: Index of deposit entry.
/// `amount`: Number of native tokens to transfer.
/// `duration`: New lockup duration.
pub fn ordinary_deposit(
    ctx: Context<OrdinaryDeposit>,
    deposit_entry_index: u8,
    amount: u64,
    duraiton: LockupTimeDuration,
) -> Result<()> {
    require!(amount > 0, VsrError::ZeroDepositAmount);
    require!(
        deposit_entry_index != NODE_DEPOSIT_ENTRY_INDEX,
        VsrError::NodeDepositReservedEntryIndex
    );

    // Deposit tokens into the vault
    token::transfer(ctx.accounts.transfer_ctx(), amount)?;

    let registrar = &ctx.accounts.registrar;
    let voter = &mut ctx.accounts.voter;
    require_gte!(
        duraiton.seconds(),
        registrar
            .deposit_config
            .ordinary_deposit_min_lockup_duration
            .seconds(),
        VsrError::InvalidLockupDuration
    );

    let curr_ts = registrar.clock_unix_timestamp();
    let lockup = Lockup::new_from_kind(LockupKind::Constant(duraiton), curr_ts, curr_ts)?;

    let mut amount_to_deposit: u64 = amount;
    if voter.is_active(deposit_entry_index)? {
        let d_entry = voter.deposit_entry_at(deposit_entry_index)?;
        if let LockupKind::Constant(old_duration) = d_entry.get_lockup().kind() {
            require_eq!(
                d_entry.get_amount_deposited_native(),
                d_entry.get_amount_initially_locked_native(),
                VsrError::InternalProgramError
            );

            if old_duration != duraiton {
                amount_to_deposit = d_entry
                    .get_amount_deposited_native()
                    .checked_add(amount)
                    .unwrap();

                voter.deactivate(deposit_entry_index)?;
                voter.activate(deposit_entry_index, lockup)?;
            }
        } else {
            return Err(error!(VsrError::InternalProgramError));
        }
    } else {
        voter.activate(deposit_entry_index, lockup)?;
    }

    voter.deposit(deposit_entry_index, amount_to_deposit, registrar)?;

    msg!(
        "ordinary_deposit: amount {}, lockup kind {:?}",
        amount,
        voter
            .deposit_entry_at(deposit_entry_index)?
            .get_lockup()
            .kind(),
    );

    Ok(())
}
