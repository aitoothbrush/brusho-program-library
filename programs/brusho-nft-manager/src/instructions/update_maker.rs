use crate::state::*;
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateMakerArgs {
    pub update_authority: Option<Pubkey>,
    pub is_active: bool,
}

#[derive(Accounts)]
pub struct UpdateMaker<'info> {
    #[account(
        mut,
        has_one = realm_authority,
    )]
    pub maker: Box<Account<'info, Maker>>,

    pub realm_authority: Signer<'info>,
}

pub fn update_maker(ctx: Context<UpdateMaker>, args: UpdateMakerArgs) -> Result<()> {
    let maker = &mut ctx.accounts.maker;

    if let Some(update_authority) = args.update_authority {
        maker.update_authority = update_authority;
    }

    maker.is_active = args.is_active;

    Ok(())
}
