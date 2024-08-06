use crate::error::*;
use crate::events::OrdinaryReleaseDepositEvent;
use crate::state::*;
use crate::NODE_DEPOSIT_ENTRY_INDEX;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct OrdinaryReleaseDeposit<'info> {
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
}

pub fn ordinary_release_deposit(
    ctx: Context<OrdinaryReleaseDeposit>,
    deposit_entry_index: u8,
    target_deposit_entry_index: u8,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, VsrError::ZeroAmount);
    require!(
        deposit_entry_index != NODE_DEPOSIT_ENTRY_INDEX
            && target_deposit_entry_index != NODE_DEPOSIT_ENTRY_INDEX,
        VsrError::NodeDepositReservedEntryIndex
    );

    let registrar = &mut ctx.accounts.registrar;
    let voter = &mut ctx.accounts.voter;

    let d_entry = voter.deposit_entry_at(deposit_entry_index)?;
    require!(d_entry.is_active(), VsrError::InactiveDepositEntry);

    let curr_ts = registrar.clock_unix_timestamp();
    // accure rewards
    registrar.accure_rewards(curr_ts);

    let lockup = d_entry.get_lockup();
    if let LockupKind::Constant(duration) = lockup.kind() {
        let amount_deposited_native = d_entry.get_amount_deposited_native();
        require_gte!(
            amount_deposited_native,
            amount,
            VsrError::InsufficientLockedTokens
        );

        voter.deactivate(deposit_entry_index, curr_ts, registrar)?;
        if amount_deposited_native > amount {
            voter.activate(deposit_entry_index, curr_ts, lockup, registrar)?;
            voter.deposit(
                deposit_entry_index,
                curr_ts,
                amount_deposited_native - amount,
                registrar,
            )?;
        }

        require!(
            !voter.is_active(target_deposit_entry_index)?,
            VsrError::ActiveDepositEntryIndex
        );

        let target_lockup = Lockup::new_from_duration(duration, curr_ts, curr_ts)?;

        voter.activate(target_deposit_entry_index, curr_ts, target_lockup, registrar)?;
        voter.deposit(target_deposit_entry_index, curr_ts, amount, registrar)?;

        emit!(OrdinaryReleaseDepositEvent {
            voter: voter.get_voter_authority(),
            deposit_entry_index,
            target_deposit_entry_index,
            amount,
        });

        Ok(())
    } else {
        Err(error!(VsrError::NotOrdinaryDepositEntry))
    }
}
