
use anchor_lang::prelude::*;

use crate::{
    canopy::fill_in_proof_from_canopy, error::RdError, merkle_proof::verify, state::{DistributionTree, Distributor}
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct ActivateDistributionTreeArgs {
    leaf_index: u32,
    leaf_hash: [u8; 32]
}

#[derive(Accounts)]
#[instruction(args: ActivateDistributionTreeArgs)]
pub struct ActivateDistributionTree<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        has_one = authority,
    )]
    pub distributor: Box<Account<'info, Distributor>>,

    pub authority: Signer<'info>,

    #[account(
        has_one = canopy,
        has_one = distributor,
    )]
    pub distribution_tree: Box<Account<'info, DistributionTree>>,

    /// CHECK: by has_one
    pub canopy: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn activate_distribution_tree(
    ctx: Context<ActivateDistributionTree>,
    args: ActivateDistributionTreeArgs,
) -> Result<()> {
    let distributor = &mut ctx.accounts.distributor;
    let distributin_tree = &ctx.accounts.distribution_tree;
    require_eq!(
        distributin_tree.period, 
        distributor.current_period + 1,
        RdError::IllegalPeriod
    );

    let mut proof = ctx.remaining_accounts
        .iter()
        .map(|a| a.key.to_bytes())
        .collect::<Vec<_>>();

    fill_in_proof_from_canopy(
        &ctx.accounts.canopy.try_borrow_data()?[1..],
        distributin_tree.max_depth,
        args.leaf_index,
        &mut proof,
    )?;

    if !verify(proof, distributin_tree.root, args.leaf_hash, args.leaf_index) {
        return Err(error!(RdError::IllegalCanopyData));
    };

    distributor.current_period = distributin_tree.period;
    Ok(())
}
