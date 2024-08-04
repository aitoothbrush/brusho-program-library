use anchor_lang::prelude::*;

use crate::Lockup;

#[event]
#[derive(Debug)]
pub struct NodeDepositEvent {
    pub voter: Pubkey,
    pub amount: u64,
    pub lockup: Lockup, 
}

#[event]
#[derive(Debug)]
pub struct NodeReleaseDepositEvent {
    pub voter: Pubkey,
    pub target_deposit_entry_index: u8,
}

#[event]
#[derive(Debug)]
pub struct OrdinaryDepositEvent {
    pub voter: Pubkey,
    pub deposit_entry_index: u8,
    pub amount: u64,
    pub lockup: Lockup, 
}

#[event]
#[derive(Debug)]
pub struct OrdinaryReleaseDepositEvent {
    pub voter: Pubkey,
    pub deposit_entry_index: u8,
    pub target_deposit_entry_index: u8,
    pub amount: u64,
}
