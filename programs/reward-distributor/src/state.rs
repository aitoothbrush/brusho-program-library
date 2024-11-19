use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct Distributor {
    pub realm: Pubkey,
    pub realm_authority: Pubkey,
    pub rewards_mint: Pubkey,
    pub authority: Pubkey,
    pub current_period: u32,
    pub security_rewards_limit: u64,
    pub bump: u8,
    pub reserved1: [u8; 3],
    pub reserved2: [u64; 12],
}

#[account]
#[derive(Default)]
pub struct DistributionTree {
    pub distributor: Pubkey,
    pub period: u32,
    pub max_depth: u32,
    pub root: [u8; 32],
    pub authority: Pubkey,
    pub canopy: Pubkey,
    pub bump: u8,
    pub reserved1: [u8; 7],
    pub reserved2: [u64; 8],
}

#[account]
#[derive(Default)]
pub struct Recipient {
    pub distributor: Pubkey,
    pub asset: Pubkey, // Asset id of compressed nft. Always pay to the owner of the NFT
    pub claimed_rewards: u64, // Amount that has been claimed by the recipient
    pub last_claim_period: u32, // The period at when last claim happens.
    pub bump: u8,
}
