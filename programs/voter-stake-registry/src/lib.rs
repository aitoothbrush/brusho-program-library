use anchor_lang::prelude::*;
use instructions::*;
use state::*;

mod error;
pub mod events;
mod governance;
mod instructions;
pub mod state;

#[macro_use]
extern crate static_assertions;

// The program address.
declare_id!("Bvsr2wYoKA1btgu3DGUFZ4KKtdwWMAQA5vvd4FKRPi8T");

/// # Introduction
///
/// The governance registry is an "addin" to the SPL governance program that
/// allows one to both vote with many different ypes of tokens for voting and to
/// scale voting power as a linear function of time locked--subject to some
/// maximum upper bound.
///
/// The flow for voting with this program is as follows:
///
/// - Create a SPL governance realm.
/// - Create a governance registry account.
/// - Add exchange rates for any tokens one wants to deposit. For example,
///   if one wants to vote with tokens A and B, where token B has twice the
///   voting power of token A, then the exchange rate of B would be 2 and the
///   exchange rate of A would be 1.
/// - Create a voter account.
/// - Deposit tokens into this program, with an optional lockup period.
/// - Vote.
///
/// Upon voting with SPL governance, a client is expected to call
/// `decay_voting_power` to get an up to date measurement of a given `Voter`'s
/// voting power for the given slot. If this is not done, then the transaction
/// will fail (since the SPL governance program will require the measurement
/// to be active for the current slot).
///
/// # Interacting with SPL Governance
///
/// This program does not directly interact with SPL governance via CPI.
/// Instead, it simply writes a `VoterWeightRecord` account with a well defined
/// format, which is then used by SPL governance as the voting power measurement
/// for a given user.
///
/// # Max Vote Weight
///
/// Given that one can use multiple tokens to vote, the max vote weight needs
/// to be a function of the total supply of all tokens, converted into a common
/// currency. For example, if you have Token A and Token B, where 1 Token B =
/// 10 Token A, then the `max_vote_weight` should be `supply(A) + supply(B)*10`
/// where both are converted into common decimals. Then, when calculating the
/// weight of an individual voter, one can convert B into A via the given
/// exchange rate, which must be fixed.
///
/// Note that the above also implies that the `max_vote_weight` must fit into
/// a u64.
#[program]
pub mod voter_stake_registry {
    use super::*;

    pub fn create_registrar(
        ctx: Context<CreateRegistrar>,
        registrar_bump: u8,
        voting_config: VotingConfig,
        deposit_config: DepositConfig,
        circuit_breaker_threshold: u64,
    ) -> Result<()> {
        instructions::create_registrar(ctx, registrar_bump, voting_config, deposit_config, circuit_breaker_threshold)
    }

    pub fn create_voter(
        ctx: Context<CreateVoter>,
        voter_bump: u8,
        voter_weight_record_bump: u8,
    ) -> Result<()> {
        instructions::create_voter(ctx, voter_bump, voter_weight_record_bump)
    }

    pub fn node_deposit(ctx: Context<NodeDeposit>) -> Result<()> {
        instructions::node_deposit(ctx)
    }

    pub fn node_release_deposit(
        ctx: Context<NodeReleaseDeposit>,
        target_deposit_entry_index: u8,
    ) -> Result<()> {
        instructions::node_release_deposit(ctx, target_deposit_entry_index)
    }

    pub fn ordinary_deposit(
        ctx: Context<OrdinaryDeposit>,
        deposit_entry_index: u8,
        amount: u64,
        duration: LockupTimeDuration
    ) -> Result<()> {
        instructions::ordinary_deposit(ctx, deposit_entry_index, amount, duration)
    }

    pub fn ordinary_release_deposit(
        ctx: Context<OrdinaryReleaseDeposit>,
        deposit_entry_index: u8,
        target_deposit_entry_index: u8,
        amount: u64,
    ) -> Result<()> {
        instructions::ordinary_release_deposit(
            ctx,
            deposit_entry_index,
            target_deposit_entry_index,
            amount,
        )
    }

    pub fn update_voter_weight_record(ctx: Context<UpdateVoterWeightRecord>) -> Result<()> {
        instructions::update_voter_weight_record(ctx)
    }

    pub fn update_max_vote_weight(ctx: Context<UpdateMaxVoteWeight>) -> Result<()> {
        instructions::update_max_vote_weight(ctx)
    }

    pub fn close_voter<'info>(ctx: Context<'_, '_, 'info, 'info, CloseVoter<'info>>) -> Result<()> {
        instructions::close_voter(ctx)
    }

    pub fn set_time_offset(ctx: Context<SetTimeOffset>, time_offset: i64) -> Result<()> {
        instructions::set_time_offset(ctx, time_offset)
    }

    pub fn update_deposit_config(ctx: Context<UpdateDepositConfig>, deposit_config: DepositConfig) -> Result<()> {
        instructions::update_deposit_config(ctx, deposit_config)
    }

    pub fn update_voting_config(ctx: Context<UpdateVotingConfig>, voting_config: VotingConfig) -> Result<()> {
        instructions::update_voting_config(ctx, voting_config)
    }

    pub fn withdraw(ctx: Context<Withdraw>, deposit_entry_index: u8, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, deposit_entry_index, amount)
    }

    pub fn claim_reward(ctx: Context<ClaimReward>, amount: Option<u64>) -> Result<()> {
        instructions::claim_reward(ctx, amount)
    }

    pub fn log_voter_info(ctx: Context<LogVoterInfo>) -> Result<()> {
        instructions::log_voter_info(ctx)
    }
}
