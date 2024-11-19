use crate::{error::RdError, state::*};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct SetCanopyDataArgs {
    pub offset: u32,
    pub bytes: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(args: SetCanopyDataArgs)]
pub struct SetCanopyData<'info> {
    pub distributor: Box<Account<'info, Distributor>>,

    #[account(
      has_one = distributor,
      has_one = canopy,
      has_one = authority,
    )]
    pub distribution_tree: Box<Account<'info, DistributionTree>>,

    pub authority: Signer<'info>,

    /// CHECK: Verified by has one
    #[account(mut)]
    pub canopy: UncheckedAccount<'info>,
}

pub fn set_canopy_data(ctx: Context<SetCanopyData>, args: SetCanopyDataArgs) -> Result<()> {
    let distributor = &ctx.accounts.distributor;
    let distribution_tree = &ctx.accounts.distribution_tree;
    require_gt!(
        distribution_tree.period,
        distributor.current_period,
        RdError::CannotSetCanopyForActiveDistributionTree
    );

    let mut data = ctx.accounts.canopy.try_borrow_mut_data()?;
    data[(args.offset + 1) as usize..(args.offset + 1) as usize + args.bytes.len()]
        .copy_from_slice(&args.bytes);

    Ok(())
}
