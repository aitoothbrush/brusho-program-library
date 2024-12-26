use anchor_lang::prelude::*;

#[event]
pub struct ClaimRewardsEvent {
    pub distributor: Pubkey,
    pub asset: Pubkey,
    pub owner: Pubkey,
    pub period: u32,
    pub amount: u64,
}
