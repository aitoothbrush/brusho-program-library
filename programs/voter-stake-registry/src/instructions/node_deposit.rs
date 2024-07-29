use crate::{error::VsrError, state::*};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

/// Deposit entry index for node deposit
pub const NODE_DEPOSIT_ENTRY_INDEX: u8 = 0;

#[derive(Accounts)]
pub struct NodeDeposit<'info> {
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

impl<'info> NodeDeposit<'info> {
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
pub fn node_deposit(ctx: Context<NodeDeposit>) -> Result<()> {
    let registrar = &ctx.accounts.registrar;
    let node_security_deposit = registrar.deposit_config.node_security_deposit;

    // Deposit tokens into the vault
    token::transfer(ctx.accounts.transfer_ctx(), node_security_deposit)?;

    let voter = &mut ctx.accounts.voter;
    require!(
        !(voter.is_active(NODE_DEPOSIT_ENTRY_INDEX)?),
        VsrError::DuplicateNodeDeposit
    );

    let curr_ts = registrar.clock_unix_timestamp();
    voter.activate(
        NODE_DEPOSIT_ENTRY_INDEX,
        Lockup::new_from_kind(
            LockupKind::Constant(registrar.deposit_config.node_deposit_lockup_duration),
            curr_ts,
            curr_ts,
        )?,
    )?;
    voter.deposit(NODE_DEPOSIT_ENTRY_INDEX, node_security_deposit, registrar)?;

    msg!(
        "node_deposit: amount {}, lockup kind {:?}",
        node_security_deposit,
        voter
            .deposit_entry_at(NODE_DEPOSIT_ENTRY_INDEX)?
            .get_lockup()
            .kind(),
    );

    Ok(())
}

