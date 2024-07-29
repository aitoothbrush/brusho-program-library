use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;
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

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Creates a new voting registrar.
pub fn create_registrar(
    ctx: Context<CreateRegistrar>,
    registrar_bump: u8,
    voting_config: VotingConfig,
    deposit_config: DepositConfig,
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

    registrar.bump = registrar_bump;
    registrar.governance_program_id = ctx.accounts.governance_program_id.key();
    registrar.realm = ctx.accounts.realm.key();
    registrar.governing_token_mint = ctx.accounts.realm_governing_token_mint.key();
    registrar.realm_authority = ctx.accounts.realm_authority.key();
    registrar.time_offset = 0;
    registrar.voting_config = voting_config;
    registrar.deposit_config = deposit_config;

    // Check for overflow in vote weight
    registrar.max_vote_weight(&ctx.accounts.realm_governing_token_mint)?;

    Ok(())
}
