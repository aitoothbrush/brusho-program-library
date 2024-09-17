use crate::error::*;
use crate::events::NodeReleaseDepositEvent;
use crate::state::*;
use crate::NODE_DEPOSIT_ENTRY_INDEX;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct NodeReleaseDeposit<'info> {
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
}

pub fn node_release_deposit(
    ctx: Context<NodeReleaseDeposit>,
    target_deposit_entry_index: u8,
) -> Result<()> {
    let registrar = &mut ctx.accounts.registrar.load_mut()?;
    let voter = &mut ctx.accounts.voter.load_mut()?;

    let d_entry = voter.deposit_entry_at(NODE_DEPOSIT_ENTRY_INDEX)?;
    require!(d_entry.is_active(), VsrError::InactiveDepositEntry);
    require!(
        !voter.is_active(target_deposit_entry_index)?,
        VsrError::ActiveDepositEntryIndex
    );

    // accrue rewards
    let curr_ts = registrar.clock_unix_timestamp();
    registrar.accrue_rewards(curr_ts);

    let amount_deposited = d_entry.get_amount_deposited_native();
    let lockup = d_entry.get_lockup();
    let lockup_kind = lockup.kind;
    if let LockupKindKind::Constant = lockup_kind.kind {
        if curr_ts < lockup.end_ts() {
            return Err(error!(VsrError::NodeDepositUnreleasableAtPresent));
        }

        voter.deactivate(NODE_DEPOSIT_ENTRY_INDEX, curr_ts, registrar)?;

        let target_lockup = Lockup::new_from_duration(lockup_kind.duration, curr_ts, curr_ts)?;

        voter.activate(
            target_deposit_entry_index,
            curr_ts,
            target_lockup,
            registrar,
        )?;
        voter.deposit(
            target_deposit_entry_index,
            curr_ts,
            amount_deposited,
            registrar,
        )?;

        emit!(NodeReleaseDepositEvent {
            voter: voter.get_voter_authority(),
            deposit_entry_index: NODE_DEPOSIT_ENTRY_INDEX,
            target_deposit_entry_index,
            amount: amount_deposited
        });

        Ok(())
    } else {
        Err(error!(VsrError::InternalProgramError))
    }
}
