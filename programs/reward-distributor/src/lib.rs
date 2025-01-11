use anchor_lang::prelude::*;
use instructions::*;

pub mod canopy;
pub mod circuit_breaker;
pub mod compressed_nfts;
pub mod error;
pub mod events;
pub mod instructions;
pub mod merkle_proof;
pub mod state;

declare_id!("bearnU7ZZAaoCyokY882WTFqHhc84rzVfTxnkiMpVCq");

#[program]
pub mod reward_distributor {
    use super::*;

    pub fn create_distributor(
        ctx: Context<CreateDistributor>,
        args: CreateDistributorArgs,
    ) -> Result<()> {
        instructions::create_distributor(ctx, args)
    }

    pub fn create_distribution_tree(ctx: Context<CreateDistributionTree>) -> Result<()> {
        instructions::create_distribution_tree(ctx)
    }

    pub fn report_oracle(ctx: Context<ReportOracle>, args: ReportOracleArgs) -> Result<()> {
        instructions::report_oracle(ctx, args)
    }

    pub fn activate_distribution_tree(ctx: Context<ActivateDistributionTree>) -> Result<()> {
        instructions::activate_distribution_tree(ctx)
    }

    pub fn claim_rewards<'info>(
        ctx: Context<'_, '_, 'info, 'info, ClaimRewards<'info>>,
        args: ClaimRewardsArgs,
    ) -> Result<()> {
        instructions::claim_rewards(ctx, args)
    }

    pub fn update_distributor(
        ctx: Context<UpdateDistributor>,
        args: UpdateDistributorArgs,
    ) -> Result<()> {
        instructions::update_distributor(ctx, args)
    }

    pub fn create_canopy(ctx: Context<CreateCanopy>) -> Result<()> {
        instructions::create_canopy(ctx)
    }

    pub fn set_canopy_data(ctx: Context<SetCanopyData>, args: SetCanopyDataArgs) -> Result<()> {
        instructions::set_canopy_data(ctx, args)
    }

    pub fn close_canopy(ctx: Context<CloseCanopy>) -> Result<()> {
        instructions::close_canopy(ctx)
    }
}
