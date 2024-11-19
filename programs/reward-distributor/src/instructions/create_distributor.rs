use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use circuit_breaker::{
    cpi::{accounts::InitializeAccountWindowedBreakerV0, initialize_account_windowed_breaker_v0},
    CircuitBreaker, InitializeAccountWindowedBreakerArgsV0, WindowedCircuitBreakerConfigV0,
};
use spl_governance::state::realm::get_realm_data;

use crate::{error::RdError, state::Distributor};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct CreateDistributorArgs {
    pub security_rewards_limit: u64,
    pub authority: Pubkey,
    pub circuit_breaker_config: WindowedCircuitBreakerConfigV0,
}

#[derive(Accounts)]
#[instruction(args: CreateDistributorArgs)]
pub struct CreateDistributor<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<Distributor>(),
        seeds = ["distributor".as_bytes(), realm.key().as_ref(), rewards_mint.key().as_ref()],
        bump,
    )]
    pub distributor: Box<Account<'info, Distributor>>,

    #[account(
        init_if_needed,
        associated_token::authority = distributor,
        associated_token::mint = rewards_mint,
        payer = payer
    )]
    pub vault: Box<Account<'info, TokenAccount>>,
    pub rewards_mint: Account<'info, Mint>,

    #[account(
        mut,
        seeds = ["account_windowed_breaker".as_bytes(), vault.key().as_ref()],
        seeds::program = circuit_breaker_program.key(),
        bump,
    )]
    /// CHECK: Verified by cpi
    pub circuit_breaker: AccountInfo<'info>,
    /// An spl-governance realm
    ///
    /// realm is validated in the instruction:
    /// - realm is owned by the governance_program_id
    /// - realm_authority is realm.authority
    /// CHECK:
    pub realm: UncheckedAccount<'info>,
    pub realm_authority: Signer<'info>,
    /// The program id of the spl-governance program the realm belongs to.
    /// CHECK:
    pub governance_program_id: UncheckedAccount<'info>,

    pub circuit_breaker_program: Program<'info, CircuitBreaker>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_distributor(
    ctx: Context<CreateDistributor>,
    args: CreateDistributorArgs,
) -> Result<()> {
    require!(args.security_rewards_limit > 0, RdError::IllegalPeriodRewardsLimit);

    // Verify that "realm_authority" is the expected authority on "realm"
    let realm = get_realm_data(
        &ctx.accounts.governance_program_id.key(),
        &ctx.accounts.realm,
    )?;
    require_keys_eq!(
        realm.authority.unwrap(),
        ctx.accounts.realm_authority.key(),
        RdError::InvalidRealmAuthority
    );

    // Initialize circuit breaker
    initialize_account_windowed_breaker_v0(
        CpiContext::new_with_signer(
            ctx.accounts.circuit_breaker_program.to_account_info(),
            InitializeAccountWindowedBreakerV0 {
                payer: ctx.accounts.payer.to_account_info(),
                circuit_breaker: ctx.accounts.circuit_breaker.to_account_info(),
                token_account: ctx.accounts.vault.to_account_info(),
                owner: ctx.accounts.distributor.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            &[&[
                ctx.accounts.realm.key().as_ref(),
                "distributor".as_ref(),
                ctx.accounts.rewards_mint.key().as_ref(),
                &[ctx.bumps.distributor],
            ]],
        ),
        InitializeAccountWindowedBreakerArgsV0 {
            authority: ctx.accounts.realm_authority.key(),
            config: args.circuit_breaker_config,
            owner: ctx.accounts.distributor.key(),
        },
    )?;

    ctx.accounts.distributor.set_inner(Distributor {
        realm: ctx.accounts.realm.key(),
        realm_authority: ctx.accounts.realm_authority.key(),
        rewards_mint: ctx.accounts.rewards_mint.key(),
        authority: args.authority,
        current_period: 0,
        security_rewards_limit: args.security_rewards_limit,
        bump: ctx.bumps.distributor,
        reserved1: [0; 3],
        reserved2: [0; 12],
    });

    Ok(())
}
