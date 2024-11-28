use crate::{error::RdError, state::*, MAX_ORACLES_COUNT};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateDistributorArgs {
    pub authority: Option<Pubkey>,
    pub oracles: Option<Vec<Pubkey>>,
}

#[derive(Accounts)]
pub struct UpdateDistributor<'info> {
    #[account(
        mut,
        has_one = realm_authority,
    )]
    pub distributor: Box<Account<'info, Distributor>>,

    pub realm_authority: Signer<'info>,
}

pub fn update_distributor(
    ctx: Context<UpdateDistributor>,
    args: UpdateDistributorArgs,
) -> Result<()> {
    let distributor = &mut ctx.accounts.distributor;

    if let Some(authority) = args.authority {
        distributor.authority = authority;
    }

    if let Some(oracles) = args.oracles {
        require!(oracles.len() <= MAX_ORACLES_COUNT, RdError::OraclesCountExceeds);

        distributor.oracles = oracles;
    }

    Ok(())
}
