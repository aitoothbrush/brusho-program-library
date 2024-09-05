use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::Mint;
use anchor_spl::token::Token;
use anchor_spl::token::TokenAccount;
use circuit_breaker::cpi::accounts::InitializeAccountWindowedBreakerV0;
use circuit_breaker::cpi::initialize_account_windowed_breaker_v0;
use circuit_breaker::CircuitBreaker;
use circuit_breaker::InitializeAccountWindowedBreakerArgsV0;
use circuit_breaker::WindowedCircuitBreakerConfigV0;
use spl_governance::state::realm;
use std::mem::size_of;

#[derive(Accounts)]
pub struct CreateRegistrar<'info> {
    /// The voting registrar. There can only be a single registrar
    /// per governance realm and governing mint.
    #[account(
        init,
        seeds = [realm.key().as_ref(), b"registrar".as_ref(), realm_governing_token_mint.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + size_of::<Registrar>()
    )]
    pub registrar: Box<Account<'info, Registrar>>,

    #[account(
        init_if_needed,
        associated_token::authority = registrar,
        associated_token::mint = realm_governing_token_mint,
        payer = payer
    )]
    pub vault: Box<Account<'info, TokenAccount>>,

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
    /// - realm_governing_token_mint must be the community or council mint
    /// - realm_authority is realm.authority
    /// CHECK:
    pub realm: UncheckedAccount<'info>,

    /// The program id of the spl-governance program the realm belongs to.
    /// CHECK:
    pub governance_program_id: UncheckedAccount<'info>,
    /// Either the realm community mint or the council mint.
    pub realm_governing_token_mint: Account<'info, Mint>,
    pub realm_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub circuit_breaker_program: Program<'info, CircuitBreaker>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Creates a new voting registrar.
pub fn create_registrar(
    ctx: Context<CreateRegistrar>,
    registrar_bump: u8,
    voting_config: VotingConfig,
    deposit_config: DepositConfig,
    circuit_breaker_threshold: u64,
) -> Result<()> {
    require!(
        voting_config.lockup_saturation_secs > 0,
        VsrError::LockupSaturationMustBePositive
    );

    require!(
        deposit_config.node_security_deposit > 0,
        VsrError::NodeSecurityDepositMustBePositive
    );

    // Verify that "realm_authority" is the expected authority on "realm"
    // and that the mint matches one of the realm mints too.
    let realm = realm::get_realm_data_for_governing_token_mint(
        &ctx.accounts.governance_program_id.key(),
        &ctx.accounts.realm.to_account_info(),
        &ctx.accounts.realm_governing_token_mint.key(),
    )?;

    require_keys_eq!(
        realm.authority.unwrap(),
        ctx.accounts.realm_authority.key(),
        VsrError::InvalidRealmAuthority
    );

    let registrar = &mut ctx.accounts.registrar;
    require_eq!(registrar_bump, ctx.bumps.registrar);

    initialize_account_windowed_breaker_v0(
        CpiContext::new_with_signer(
            ctx.accounts.circuit_breaker_program.to_account_info(),
            InitializeAccountWindowedBreakerV0 {
                payer: ctx.accounts.payer.to_account_info(),
                circuit_breaker: ctx.accounts.circuit_breaker.to_account_info(),
                token_account: ctx.accounts.vault.to_account_info(),
                owner: registrar.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            &[&[
                ctx.accounts.realm.key().as_ref(),
                "registrar".as_ref(),
                ctx.accounts.realm_governing_token_mint.key().as_ref(),
                &[ctx.bumps.registrar],
            ]],
        ),
        InitializeAccountWindowedBreakerArgsV0 {
            authority: ctx.accounts.realm_authority.key(),
            config: WindowedCircuitBreakerConfigV0 {
                window_size_seconds: SECS_PER_DAY,
                threshold_type: circuit_breaker::ThresholdType::Absolute,
                threshold: circuit_breaker_threshold,
            },
            owner: registrar.key(),
        },
    )?;

    registrar.bump = registrar_bump;
    registrar.governance_program_id = ctx.accounts.governance_program_id.key();
    registrar.realm = ctx.accounts.realm.key();
    registrar.governing_token_mint = ctx.accounts.realm_governing_token_mint.key();
    registrar.realm_authority = ctx.accounts.realm_authority.key();
    registrar.voting_config = voting_config;
    registrar.deposit_config = deposit_config;
    registrar.current_reward_amount_per_second = Exponential::new(0);
    registrar.last_reward_amount_per_second_rotated_ts = 0;
    registrar.reward_accrual_ts = 0;
    registrar.reward_index = Exponential::new(0);
    registrar.issued_reward_amount = 0;
    registrar.permanently_locked_amount = 0;
    registrar.time_offset = 0;


    // Initialize reward stuffs
    let curr_ts = registrar.clock_unix_timestamp();
    registrar.accrue_rewards(curr_ts);

    // Check for overflow in vote weight
    registrar.max_vote_weight(&ctx.accounts.realm_governing_token_mint)?;

    Ok(())
}
