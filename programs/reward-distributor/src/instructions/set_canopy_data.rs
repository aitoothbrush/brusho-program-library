use crate::{error::RdError, state::*};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct SetCanopyDataArgs {
    pub offset: u32,
    pub bytes: Vec<u8>,
}

#[derive(Accounts)]
pub struct SetCanopyData<'info> {
    #[account(
        has_one = authority @ RdError::Authorization,
        has_one = canopy_data,
    )]
    pub canopy: Box<Account<'info, Canopy>>,

    #[account(mut)]
    /// CHECK: see canopy constraints 
    pub canopy_data: UncheckedAccount<'info>,

    pub authority: Signer<'info>,
}

pub fn set_canopy_data(ctx: Context<SetCanopyData>, args: SetCanopyDataArgs) -> Result<()> {
    let mut data = ctx.accounts.canopy_data.try_borrow_mut_data()?;
    data[(args.offset) as usize..(args.offset) as usize + args.bytes.len()]
        .copy_from_slice(&args.bytes);

    Ok(())
}
