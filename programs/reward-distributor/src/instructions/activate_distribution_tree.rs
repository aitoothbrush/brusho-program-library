use anchor_lang::prelude::*;

use crate::{
    error::RdError,
    state::{DistributionTree, Distributor},
};

#[derive(Accounts)]
pub struct ActivateDistributionTree<'info> {
    #[account(
        mut,
        has_one = authority @ RdError::Authorization,
    )]
    pub distributor: Box<Account<'info, Distributor>>,

    pub authority: Signer<'info>,

    #[account(
        has_one = distributor,
    )]
    pub distribution_tree: Box<Account<'info, DistributionTree>>,
}

pub fn activate_distribution_tree(ctx: Context<ActivateDistributionTree>) -> Result<()> {
    let distributor = &mut ctx.accounts.distributor;
    let distributin_tree = &ctx.accounts.distribution_tree;

    require_eq!(
        distributin_tree.period,
        distributor.current_period + 1,
        RdError::IllegalPeriod
    );

    require!(distributin_tree.oracle_choice().is_some(), RdError::InvalidOracleReports);

    distributor.current_period = distributin_tree.period;
    Ok(())
}
