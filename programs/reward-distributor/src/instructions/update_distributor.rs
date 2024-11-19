use crate::state::*;
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateDistributorArgs {
    pub authority: Option<Pubkey>,
    pub security_rewards_limit: Option<u64>,
}

#[derive(Accounts)]
#[instruction(args: UpdateDistributorArgs)]
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
    if let Some(security_rewards_limit) = args.security_rewards_limit {
        distributor.security_rewards_limit = security_rewards_limit;
    }

    Ok(())
}
