use crate::state::*;
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct UpdateIssuingAuthorityArgs {
    pub issuing_authority: Pubkey,
}

#[derive(Accounts)]
pub struct UpdateIssuingAuthority<'info> {
    pub update_authority: Signer<'info>,
    #[account(
        mut,
        has_one = update_authority,
    )]
    pub maker: Box<Account<'info, Maker>>,
}

pub fn update_issuing_authority(ctx: Context<UpdateIssuingAuthority>, args: UpdateIssuingAuthorityArgs) -> Result<()> {
    let maker = &mut ctx.accounts.maker;

    maker.issuing_authority = args.issuing_authority;

    Ok(())
}
