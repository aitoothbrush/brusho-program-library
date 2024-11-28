use crate::{error::RdError, state::*};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct ReportOracleArgs {
    pub index: u32,
    pub report: OracleReport
}

#[derive(Accounts)]
#[instruction(args: ReportOracleArgs)]
pub struct ReportOracle<'info> {
    #[account(
      constraint = distributor.oracles[args.index as usize] == authority.key() @ RdError::Authorization,
    )]
    pub distributor: Box<Account<'info, Distributor>>,

    #[account(
      mut,
      has_one = distributor,
    )]
    pub distribution_tree: Box<Account<'info, DistributionTree>>,

    pub authority: Signer<'info>,
}

pub fn report_oracle(ctx: Context<ReportOracle>, args: ReportOracleArgs) -> Result<()> {
    let distributor = &ctx.accounts.distributor;
    let distribution_tree = &mut ctx.accounts.distribution_tree;

    require!(
        distribution_tree.period > distributor.current_period,
        RdError::CannotReportAtPresent
    );

    distribution_tree.oracle_reports[args.index as usize] = Option::Some(args.report);

    // TODO: emit events

    Ok(())
}
