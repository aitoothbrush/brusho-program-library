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
    let distribution_tree = &ctx.accounts.distribution_tree;

    require_eq!(
        distribution_tree.period,
        distributor.current_period + 1,
        RdError::IllegalPeriod
    );

    require!(distribution_tree.oracle_choice().is_some(), RdError::OracleReportsNotAvailable);

    distributor.current_period = distribution_tree.period;
    Ok(())
}
