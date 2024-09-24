use anchor_lang::prelude::*;

use crate::{Lockup, VOTER_DEPOSIT_ENTRY_COUNT};

#[event]
pub struct NodeDepositEvent {
    // voter authority address
    pub voter: Pubkey,
    pub amount: u64,
    pub lockup: Lockup,
}

#[event]
pub struct NodeReleaseDepositEvent {
    // voter authority address
    pub voter: Pubkey,
    pub deposit_entry_index: u8,
    pub target_deposit_entry_index: u8,
    pub amount: u64,
}

#[event]
pub struct OrdinaryDepositEvent {
    // voter authority address
    pub voter: Pubkey,
    pub deposit_entry_index: u8,
    pub amount: u64,
    pub lockup: Lockup,
}

#[event]
pub struct OrdinaryReleaseDepositEvent {
    // voter authority address
    pub voter: Pubkey,
    pub deposit_entry_index: u8,
    pub target_deposit_entry_index: u8,
    pub amount: u64,
}

#[event]
pub struct WithdrawEvent {
    // voter authority address
    pub voter: Pubkey,
    pub deposit_entry_index: u8,
    pub amount: u64,
}

#[event]
pub struct ClaimRewardEvent {
    // voter authority address
    pub voter: Pubkey,
    pub amount: u64,
}

#[event]
pub struct VoterInfo {
    /// Voter's total voting power
    pub voting_power: u64,
    /// Voter's total voting power, when ignoring any effects from lockup
    pub voting_power_baseline: u64,
    /// Accumulated reward amount
    pub reward_amount: u64,
    /// DepositEntry info array
    pub deposit_entries: [Option<DepositEntryInfo>; VOTER_DEPOSIT_ENTRY_COUNT],
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DepositEntryInfo {
    /// The lockup info 
    pub lockup: Lockup,
    /// Amount that is locked
    pub amount_locked: u64,
    /// Amount that is unlocked
    pub amount_unlocked: u64,
    /// Voting power implied by this deposit entry
    pub voting_power: u64,
    /// Voting power without any adjustments for lockup
    pub voting_power_baseline: u64,
    /// Information about vesting, if any
    pub vesting: Option<VestingInfo>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct VestingInfo {
    /// Amount of tokens vested each period
    pub rate: u64,
    /// Time of the next upcoming vesting
    pub next_timestamp: u64,
}
