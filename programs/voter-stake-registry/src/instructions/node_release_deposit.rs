use crate::error::*;
use crate::state::*;
use crate::NODE_DEPOSIT_ENTRY_INDEX;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct NodeReleaseDeposit<'info> {
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

pub fn node_release_deposit(ctx: Context<NodeReleaseDeposit>, target_deposit_entry_index: u8) -> Result<()> {
    let registrar = &ctx.accounts.registrar;
    let voter = &mut ctx.accounts.voter;

    let d_entry = voter.deposit_entry_at(NODE_DEPOSIT_ENTRY_INDEX)?;
    require!(d_entry.is_active(), VsrError::InactiveDepositEntry);

    require!(!voter.is_active(target_deposit_entry_index)?, VsrError::ActiveDepositEntryIndex);

    let node_security_deposit  = d_entry.get_amount_deposited_native();
    let lockup = d_entry.get_lockup();
    if let LockupKind::Constant(duration) = lockup.kind() {
        let curr_ts = registrar.clock_unix_timestamp();
        if curr_ts < lockup.end_ts() {
            return Err(error!(VsrError::NodeDepositUnreleasableAtPresent));
        }

        voter.deactivate(NODE_DEPOSIT_ENTRY_INDEX)?;
        
        let target_lockup = Lockup::new_from_duration(duration, curr_ts, curr_ts)?;

        voter.activate(target_deposit_entry_index, target_lockup)?;
        voter.deposit(target_deposit_entry_index, node_security_deposit, registrar)?;

        Ok(())
    } else {
        Err(error!(VsrError::InternalProgramError))
    }

}
