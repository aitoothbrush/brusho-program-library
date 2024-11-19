use anchor_lang::prelude::*;

use crate::{
    canopy::check_canopy_bytes, id, state::{DistributionTree, Distributor}
};

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct CreateDistributionTreeArgs {
    pub max_depth: u32,
    pub root: [u8; 32],
    pub authority: Pubkey,
}

#[derive(Accounts)]
#[instruction(args: CreateDistributionTreeArgs)]
pub struct CreateDistributionTree<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + std::mem::size_of::<DistributionTree>(),
        seeds = ["distribution_tree".as_bytes(), &distributor.key().to_bytes(), &(distributor.current_period + 1).to_be_bytes()],
        bump,
    )]
    pub distribution_tree: Box<Account<'info, DistributionTree>>,

    #[account(
        has_one = authority,
    )]
    pub distributor: Box<Account<'info, Distributor>>,

    pub authority: Signer<'info>,

    /// CHECK: Account to store the canopy, the size will determine the size of the canopy
    #[account(
        mut,
        owner = id(),
        constraint = canopy.data.borrow()[0] == 0,
        constraint = check_canopy_bytes(&canopy.data.borrow()[1..]).is_ok(),
    )]
    pub canopy: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_distribution_tree(
    ctx: Context<CreateDistributionTree>,
    args: CreateDistributionTreeArgs,
) -> Result<()> {
    let mut data = ctx.accounts.canopy.try_borrow_mut_data()?;
    data[0] = 1;

    ctx.accounts.distribution_tree.set_inner(DistributionTree {
        distributor: ctx.accounts.distributor.key(),
        period: ctx.accounts.distributor.current_period + 1,
        max_depth: args.max_depth,
        root: args.root,
        canopy: ctx.accounts.canopy.key(),
        authority: args.authority,
        bump: ctx.bumps.distribution_tree,
        reserved1: [0; 7],
        reserved2: [0; 8],
    });

    Ok(())
}
