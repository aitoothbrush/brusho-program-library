use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateDepositConfig<'info> {
    #[account(
        mut, 
        has_one = realm_authority,
    )]
    pub registrar: AccountLoader<'info, Registrar>,

    pub realm_authority: Signer<'info>,
}

/// Update deposit configurations
pub fn update_deposit_config(
    ctx: Context<UpdateDepositConfig>,
    deposit_config: DepositConfig,
) -> Result<()> {
    require!(
        deposit_config.node_security_deposit > 0,
        VsrError::NodeSecurityDepositMustBePositive
    );

    let registrar = &mut ctx.accounts.registrar.load_mut()?;
    registrar.deposit_config = deposit_config;

    Ok(())
}
