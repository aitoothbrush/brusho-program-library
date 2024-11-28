use anchor_lang::prelude::*;

use crate::{error::RdError, state::{DistributionTree, Distributor, OracleReport}};

use super::MAX_ORACLES_COUNT;

#[derive(Accounts)]
pub struct CreateDistributionTree<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + 4 + std::mem::size_of::<DistributionTree>() + std::mem::size_of::<OracleReport>() * MAX_ORACLES_COUNT,
        seeds = ["distribution_tree".as_bytes(), &distributor.key().to_bytes(), &(distributor.current_period + 1).to_be_bytes()],
        bump,
    )]
    pub distribution_tree: Box<Account<'info, DistributionTree>>,

    #[account(
        has_one = authority @ RdError::Authorization,
    )]
    pub distributor: Box<Account<'info, Distributor>>,

    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_distribution_tree(ctx: Context<CreateDistributionTree>) -> Result<()> {
    ctx.accounts.distribution_tree.set_inner(DistributionTree {
        distributor: ctx.accounts.distributor.key(),
        period: ctx.accounts.distributor.current_period + 1,
        oracle_reports: vec![None; ctx.accounts.distributor.oracles.len()],
        bump: ctx.bumps.distribution_tree,
    });

    Ok(())
}
