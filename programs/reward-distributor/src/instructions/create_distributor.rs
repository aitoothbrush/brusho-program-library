use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use circuit_breaker::{
    cpi::{accounts::InitializeAccountWindowedBreakerV0, initialize_account_windowed_breaker_v0},
    CircuitBreaker, InitializeAccountWindowedBreakerArgsV0,
};
use spl_governance::state::realm::get_realm_data;

use crate::{
    circuit_breaker::WindowedCircuitBreakerConfigV0,
    distributor_seeds,
    error::RdError,
    state::Distributor,
};

pub const MAX_ORACLES_COUNT: usize = 5;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct CreateDistributorArgs {
    pub name: String,
    pub authority: Pubkey,
    pub oracles: Vec<Pubkey>,
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
        space = 8 + 60 + std::mem::size_of::<Distributor>() + std::mem::size_of::<Pubkey>() * MAX_ORACLES_COUNT, 
        seeds = ["distributor".as_bytes(), realm.key().as_ref(), rewards_mint.key().as_ref(), args.name.as_bytes()],
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
    require!(args.name.len() <= 32, RdError::InvalidDistributorName);
    require!(args.oracles.len() <= MAX_ORACLES_COUNT, RdError::OraclesCountExceeds);

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

    let distributor = Distributor {
        realm: ctx.accounts.realm.key(),
        realm_authority: ctx.accounts.realm_authority.key(),
        rewards_mint: ctx.accounts.rewards_mint.key(),
        vault: ctx.accounts.vault.key(),
        name: args.name,
        authority: args.authority,
        oracles: args.oracles,
        current_period: 0,
        bump: ctx.bumps.distributor,
    };

    let distributor_signer_seeds: &[&[u8]] = distributor_seeds!(distributor);
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
            &[distributor_signer_seeds],
        ),
        InitializeAccountWindowedBreakerArgsV0 {
            authority: ctx.accounts.realm_authority.key(),
            config: args.circuit_breaker_config.into(),
            owner: ctx.accounts.distributor.key(),
        },
    )?;

    ctx.accounts.distributor.set_inner(distributor);

    Ok(())
}
