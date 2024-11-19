use crate::{canopy::check_canopy_bytes, id, state::*};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateDistributionTreeArgs {
    pub max_depth: Option<u32>,
    pub root: Option<[u8; 32]>,
    pub authority: Option<Pubkey>,
}

#[derive(Accounts)]
#[instruction(args: UpdateDistributionTreeArgs)]
pub struct UpdateDistributionTree<'info> {
    #[account(
        mut,
        has_one = distributor,
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
      constraint = canopy.key() == distribution_tree.canopy || canopy.data.borrow()[0] == 0,
      constraint = check_canopy_bytes(&canopy.data.borrow()[1..]).is_ok(),
    )]
    pub canopy: AccountInfo<'info>,
}

pub fn update_distribution_tree(
    ctx: Context<UpdateDistributionTree>,
    args: UpdateDistributionTreeArgs,
) -> Result<()> {
    let mut data = ctx.accounts.canopy.try_borrow_mut_data()?;
    data[0] = 1;

    let distribution_tree = &mut ctx.accounts.distribution_tree;
    distribution_tree.canopy = ctx.accounts.canopy.key();

    if let Some(max_depth) = args.max_depth {
        distribution_tree.max_depth = max_depth;
    }

    if let Some(root) = args.root {
        distribution_tree.root = root;
    }

    if let Some(authority) = args.authority {
        distribution_tree.authority = authority;
    }

    Ok(())
}
