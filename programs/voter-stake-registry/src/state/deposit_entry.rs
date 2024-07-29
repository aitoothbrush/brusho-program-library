use crate::state::lockup::Lockup;
use crate::state::registrar::VotingConfig;
use crate::error::*;
use anchor_lang::prelude::*;
use std::cmp::min;
use std::convert::TryFrom;


/// Bookkeeping for a single deposit for a given mint and lockup schedule.
#[derive(AnchorSerialize, AnchorDeserialize, Default, Clone, Copy, PartialEq)]
pub struct DepositEntry {
    // Locked state.
    lockup: Lockup,

    /// Amount in deposited, in native currency. Withdraws of vested tokens
    /// directly reduce this amount.
    ///
    /// This directly tracks the total amount added by the user. They may
    /// never withdraw more than this amount.
    amount_deposited_native: u64,

    /// Amount in locked when the lockup began, in native currency.
    ///
    /// Note that this is not adjusted for withdraws. It is possible for this
    /// value to be bigger than amount_deposited_native after some vesting
    /// and withdrawals.
    ///
    /// This value is needed to compute the amount that vests each peroid,
    /// which should not change due to withdraws.
    amount_initially_locked_native: u64,

    // True if the deposit entry is being active.
    is_active: bool,

    reserved: [u8; 23],
}
const_assert!(std::mem::size_of::<DepositEntry>() == 32 + 2 * 8 + 1 + 23);
const_assert!(std::mem::size_of::<DepositEntry>() % 8 == 0);

/// impl: factory function and getters
impl DepositEntry {
    pub fn new_from_lockup(lockup: Lockup) -> Result<DepositEntry> {
        Ok(DepositEntry {
            lockup,
            amount_deposited_native: 0,
            amount_initially_locked_native: 0,
            is_active: true,
            reserved: [0; 23],
        })
    }

    #[inline(always)]
    pub fn get_lockup(&self) -> Lockup {
        self.lockup
    }

    #[inline(always)]
    pub fn get_amount_deposited_native(&self) -> u64 {
        self.amount_deposited_native
    }

    #[inline(always)]
    pub fn get_amount_initially_locked_native(&self) -> u64 {
        self.amount_initially_locked_native
    }

}

impl DepositEntry {
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Caution: this is a dangerous operation
    pub fn deactivate(&mut self) -> Result<()> {
        require!(self.is_active, VsrError::InternalProgramError);

        self.lockup = Lockup::default();
        self.amount_deposited_native = 0;
        self.amount_initially_locked_native = 0;
        self.is_active = false;
        Ok(())
    }

    pub fn deposit(&mut self, curr_ts: i64, amount: u64) -> Result<()> {
        require!(self.is_active, VsrError::InternalProgramError);

        let vested_amount = self.vested(curr_ts)?;
        // Deduct vested amount from amount_initially_locked_native
        self.amount_initially_locked_native = self
            .amount_initially_locked_native
            .checked_sub(vested_amount)
            .unwrap();

        // Add new deposited to amount_initially_locked_native
        self.amount_initially_locked_native = self
            .amount_initially_locked_native
            .checked_add(amount)
            .unwrap();

        // Add new deposited to amount_deposited_native
        self.amount_deposited_native = self
            .amount_deposited_native
            .checked_add(amount)
            .unwrap();

        // Reset lockup
        if self.lockup.start_ts() < curr_ts {
            self.lockup = Lockup::new_from_kind(self.lockup.kind(), curr_ts, curr_ts)?;
        }
        Ok(())
    }

    pub fn withdraw(&mut self, curr_ts: i64, amount: u64) -> Result<()> {
        require!(self.is_active, VsrError::InternalProgramError);

        let amount_unlocked = self.amount_unlocked(curr_ts)?;
        require_gte!(
            amount_unlocked,
            amount,
            VsrError::InsufficientUnlockedTokens
        );

        self.amount_deposited_native = self
            .amount_deposited_native
            .checked_sub(amount)
            .unwrap();
        Ok(())
    }

    /// # Voting Power Caclulation
    ///
    /// Returns the voting power for the deposit, giving locked tokens boosted
    /// voting power that scales linearly with the lockup time.
    ///
    /// For each cliff-locked token, the vote weight is:
    ///
    ///    voting_power = baseline_vote_weight
    ///                   + lockup_duration_factor * max_extra_lockup_vote_weight
    ///
    /// with
    ///   - lockup_duration_factor = min(lockup_time_remaining / lockup_saturation_secs, 1)
    ///   - the VotingMintConfig providing the values for
    ///     baseline_vote_weight, max_extra_lockup_vote_weight, lockup_saturation_secs
    ///
    /// Linear vesting schedules can be thought of as a sequence of cliff-
    /// locked tokens and have the matching voting weight.
    ///
    /// ## Cliff Lockup
    ///
    /// The cliff lockup allows one to lockup their tokens for a set period
    /// of time, unlocking all at once on a given date.
    ///
    /// The calculation for this is straightforward and is detailed above.
    ///
    /// ### Decay
    ///
    /// As time passes, the voting power decays until it's back to just
    /// fixed_factor when the cliff has passed. This is important because at
    /// each point in time the lockup should be equivalent to a new lockup
    /// made for the remaining time period.
    ///
    /// ## Linear Vesting Lockup
    ///
    /// Daily/monthly linear vesting can be calculated with series sum, see
    /// voting_power_linear_vesting() below.
    ///
    pub fn voting_power(&self, voting_config: &VotingConfig, curr_ts: i64) -> Result<u64> {
        require!(self.is_active, VsrError::InternalProgramError);

        let baseline_vote_weight =
            voting_config.baseline_vote_weight(self.amount_deposited_native)?;
        let max_locked_vote_weight =
            voting_config.max_extra_lockup_vote_weight(self.amount_initially_locked_native)?;
        let locked_vote_weight = self.voting_power_locked(
            curr_ts,
            max_locked_vote_weight,
            voting_config.lockup_saturation_secs,
        )?;
        require_gte!(
            max_locked_vote_weight,
            locked_vote_weight,
            VsrError::InternalErrorBadLockupVoteWeight
        );
        baseline_vote_weight
            .checked_add(locked_vote_weight)
            .ok_or_else(|| error!(VsrError::VoterWeightOverflow))
    }

    /// Vote power contribution from locked funds only.
    pub fn voting_power_locked(
        &self,
        curr_ts: i64,
        max_locked_vote_weight: u64,
        lockup_saturation_secs: u64,
    ) -> Result<u64> {
        require!(self.is_active, VsrError::InternalProgramError);

        if self.lockup.expired(curr_ts) || max_locked_vote_weight == 0 {
            return Ok(0);
        }
        if self.lockup.kind().is_vesting() {
            self.voting_power_linear_vesting(
                curr_ts,
                max_locked_vote_weight,
                lockup_saturation_secs,
            )
        } else {
            self.voting_power_cliff(curr_ts, max_locked_vote_weight, lockup_saturation_secs)
        }
    }

    /// Vote power contribution from funds with linear vesting.
    fn voting_power_cliff(
        &self,
        curr_ts: i64,
        max_locked_vote_weight: u64,
        lockup_saturation_secs: u64,
    ) -> Result<u64> {
        let remaining = min(self.lockup.seconds_left(curr_ts), lockup_saturation_secs);
        Ok(u64::try_from(
            (max_locked_vote_weight as u128)
                .checked_mul(remaining as u128)
                .unwrap()
                .checked_div(lockup_saturation_secs as u128)
                .unwrap(),
        )
        .unwrap())
    }

    /// Vote power contribution from cliff-locked funds.
    fn voting_power_linear_vesting(
        &self,
        curr_ts: i64,
        max_locked_vote_weight: u64,
        lockup_saturation_secs: u64,
    ) -> Result<u64> {
        let periods_left = self.lockup.periods_left(curr_ts)?;
        let periods_total = self.lockup.periods_total();
        let period_secs = self.lockup.kind().period_secs() as u64;

        if periods_left == 0 {
            return Ok(0);
        }

        // This computes the voting power by considering the linear vesting as a
        // sequence of vesting cliffs.
        //
        // For example, if there were 5 vesting periods, with 3 of them left
        // (i.e. two have already vested and their tokens are no longer locked)
        // we'd have (max_locked_vote_weight / 5) weight in each of them, and the
        // voting power would be:
        //    (max_locked_vote_weight/5) * secs_left_for_cliff_1 / lockup_saturation_secs
        //  + (max_locked_vote_weight/5) * secs_left_for_cliff_2 / lockup_saturation_secs
        //  + (max_locked_vote_weight/5) * secs_left_for_cliff_3 / lockup_saturation_secs
        //
        // Or more simply:
        //    max_locked_vote_weight * (\sum_p secs_left_for_cliff_p) / (5 * lockup_saturation_secs)
        //  = max_locked_vote_weight * lockup_secs                    / denominator
        //
        // The value secs_left_for_cliff_p splits up as
        //    secs_left_for_cliff_p = min(
        //        secs_to_closest_cliff + (p-1) * period_secs,
        //        lockup_saturation_secs)
        //
        // If secs_to_closest_cliff < lockup_saturation_secs, we can split the sum
        //    \sum_p secs_left_for_cliff_p
        // into the part before saturation and the part after:
        // Let q be the largest integer 1 <= q <= periods_left where
        //        secs_to_closest_cliff + (q-1) * period_secs < lockup_saturation_secs
        //    =>  q = (lockup_saturation_secs - secs_to_closest_cliff + period_secs) / period_secs
        // and r be the integer where q + r = periods_left, then:
        //    lockup_secs := \sum_p secs_left_for_cliff_p
        //                 = \sum_{p<=q} secs_left_for_cliff_p
        //                   + r * lockup_saturation_secs
        //                 = q * secs_to_closest_cliff
        //                   + period_secs * \sum_0^q (p-1)
        //                   + r * lockup_saturation_secs
        //
        // Where the sum can be expanded to:
        //
        //    sum_full_periods := \sum_0^q (p-1)
        //                      = q * (q - 1) / 2
        //

        let secs_to_closest_cliff = self
            .lockup
            .seconds_left(curr_ts)
            .checked_sub(
                period_secs
                    .checked_mul(periods_left.saturating_sub(1))
                    .unwrap(),
            )
            .unwrap();

        if secs_to_closest_cliff >= lockup_saturation_secs {
            return Ok(max_locked_vote_weight);
        }

        // In the example above, periods_total was 5.
        let denominator = periods_total.checked_mul(lockup_saturation_secs).unwrap();

        let lockup_saturation_periods = (lockup_saturation_secs
            .saturating_sub(secs_to_closest_cliff)
            .checked_add(period_secs)
            .unwrap())
        .checked_div(period_secs)
        .unwrap();
        let q = min(lockup_saturation_periods, periods_left);
        let r = periods_left.saturating_sub(q);

        // Sum of the full periods left for all remaining vesting cliffs.
        //
        // Examples:
        // - if there are 3 periods left, meaning three vesting cliffs in the future:
        //   one has only a fractional period left and contributes 0
        //   the next has one full period left
        //   and the next has two full periods left
        //   so sums to 3 = 3 * 2 / 2
        // - if there's only one period left, the sum is 0
        let sum_full_periods = q.checked_mul(q.saturating_sub(1)).unwrap() / 2;

        // Total number of seconds left over all periods_left remaining vesting cliffs
        let lockup_secs_fractional = q.checked_mul(secs_to_closest_cliff).unwrap();
        let lockup_secs_full = sum_full_periods.checked_mul(period_secs).unwrap();
        let lockup_secs_saturated = r.checked_mul(lockup_saturation_secs).unwrap();
        let lockup_secs = lockup_secs_fractional as u128
            + lockup_secs_full as u128
            + lockup_secs_saturated as u128;

        Ok(u64::try_from(
            (max_locked_vote_weight as u128)
                .checked_mul(lockup_secs)
                .unwrap()
                .checked_div(denominator as u128)
                .unwrap(),
        )
        .unwrap())
    }

    /// Returns the amount of unlocked tokens for this deposit--in native units
    /// of the original token amount (not scaled by the exchange rate).
    pub fn vested(&self, curr_ts: i64) -> Result<u64> {
        require!(self.is_active, VsrError::InternalProgramError);

        if self.lockup.expired(curr_ts) {
            return Ok(self.amount_initially_locked_native);
        }
        if self.lockup.kind().is_vesting() {
            self.vested_linearly(curr_ts)
        } else {
            Ok(0)
        }
    }

    fn vested_linearly(&self, curr_ts: i64) -> Result<u64> {
        let period_current = self.lockup.period_current(curr_ts)?;
        let periods_total = self.lockup.periods_total();
        if period_current == 0 {
            return Ok(0);
        }
        if period_current >= periods_total {
            return Ok(self.amount_initially_locked_native);
        }
        let vested = self
            .amount_initially_locked_native
            .checked_mul(period_current)
            .unwrap()
            .checked_div(periods_total)
            .unwrap();
        Ok(vested)
    }

    /// Returns native tokens still locked.
    #[inline(always)]
    pub fn amount_locked(&self, curr_ts: i64) -> Result<u64> {
        require!(self.is_active, VsrError::InternalProgramError);

        Ok(self.amount_initially_locked_native
            .checked_sub(self.vested(curr_ts).unwrap())
            .unwrap())
    }

    /// Returns native tokens that are unlocked given current vesting
    /// and previous withdraws.
    #[inline(always)]
    pub fn amount_unlocked(&self, curr_ts: i64) -> Result<u64> {
        require!(self.is_active, VsrError::InternalProgramError);

        Ok(self.amount_deposited_native
            .checked_sub(self.amount_locked(curr_ts)?)
            .unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LockupKind::*, *};

    #[test]
    pub fn deactivate_test() -> Result<()> {
        let lockup = Lockup::new_from_kind(Daily(2), 0, 1)?;
        let mut entry = DepositEntry::new_from_lockup(lockup)?;

        assert!(entry.deactivate().is_ok());
        // deactivate again should fail
        assert_eq!(entry.deactivate(), Err(error!(VsrError::InternalProgramError)) as Result<()>);

        Ok(())
    }

    #[test]
    pub fn deposit_test() -> Result<()> {
        let day: i64 = i64::try_from(LockupTimeUnit::Day.seconds()).unwrap();
        let lockup_start = 1; // arbitrary point
        let lockup_kind = Daily(4);
        let lockup = Lockup::new_from_kind(lockup_kind, 0, lockup_start)?;
        let mut entry = DepositEntry::new_from_lockup(lockup)?;

        // deposit at time 0
        entry.deposit(0, 10_000)?;

        assert_eq!(entry.amount_deposited_native, 10_000);
        assert_eq!(entry.amount_initially_locked_native, 10_000);
        assert_eq!(entry.lockup, lockup);

        // deposit at time 1
        entry.deposit(1, 10_000)?;
        assert_eq!(entry.amount_deposited_native, 20_000);
        assert_eq!(entry.amount_initially_locked_native, 20_000);
        assert_eq!(entry.lockup, lockup);

        // deposit at lock_start + day
        entry.deposit(lockup_start + day, 10_000)?;
        assert_eq!(entry.amount_deposited_native, 30_000);
        assert_eq!(entry.amount_initially_locked_native, 25_000);
        assert_ne!(entry.lockup, lockup);
        assert_eq!(entry.lockup.kind(), lockup_kind);
        assert_eq!(entry.lockup.start_ts(), lockup_start + day);

        // deactivate, then deposit again
        entry.deactivate()?;
        assert_eq!(entry.deposit(lockup_start + day, 10_000), Err(error!(VsrError::InternalProgramError)) as Result<()>);

        Ok(())
    }

    #[test]
    pub fn far_future_lockup_start_test() -> Result<()> {
        // Check that voting power stays correct even if the lockup is very far in the
        // future, or at least more than lockup_saturation_secs in the future.
        let day: i64 = i64::try_from(LockupTimeUnit::Day.seconds()).unwrap();
        let saturation: i64 = 5 * day;
        let lockup_start = 10_000_000_000; // arbitrary point
        let mut entry = DepositEntry::new_from_lockup(Lockup::new_from_kind(Daily(2), lockup_start, lockup_start)?)?;
        entry.deposit(lockup_start, 10_000)?;
        let voting_mint_config = VotingConfig {
            baseline_vote_weight_scaled_factor: 1_000_000_000, // 1x
            max_extra_lockup_vote_weight_scaled_factor: 1_000_000_000, // 1x
            lockup_saturation_secs: saturation as u64,
        };

        let baseline_vote_weight =
            voting_mint_config.baseline_vote_weight(entry.amount_deposited_native)?;
        assert_eq!(baseline_vote_weight, 10_000);
        let max_locked_vote_weight = voting_mint_config
            .max_extra_lockup_vote_weight(entry.amount_initially_locked_native)?;
        assert_eq!(max_locked_vote_weight, 10_000);

        // The timestamp 100_000 is very far before the lockup_start timestamp
        let withdrawable = entry.amount_unlocked(100_000)?;
        assert_eq!(withdrawable, 0);
        let voting_power = entry.voting_power(&voting_mint_config, 100_000).unwrap();
        assert_eq!(voting_power, 20_000);

        let voting_power = entry
            .voting_power(&voting_mint_config, lockup_start - saturation)
            .unwrap();
        assert_eq!(voting_power, 20_000);

        let voting_power = entry
            .voting_power(&voting_mint_config, lockup_start - saturation + day)
            .unwrap();
        assert_eq!(voting_power, 20_000);

        let voting_power = entry
            .voting_power(&voting_mint_config, lockup_start - saturation + day + 1)
            .unwrap();
        assert_eq!(voting_power, 19_999);

        let voting_power = entry
            .voting_power(&voting_mint_config, lockup_start - saturation + 2 * day)
            .unwrap();
        assert_eq!(voting_power, 19_000); // the second cliff has only 4/5th of lockup period left

        let voting_power = entry
            .voting_power(&voting_mint_config, lockup_start - saturation + 2 * day + 1)
            .unwrap();
        assert_eq!(voting_power, 18_999);

        Ok(())
    }

    #[test]
    pub fn daily_vested_test() -> Result<()> {
        let day: i64 = i64::try_from(LockupTimeUnit::Day.seconds()).unwrap();
        let lockup_start = 1; // arbitrary point
        let mut entry = DepositEntry::new_from_lockup(Lockup::new_from_kind(Daily(2), 0, lockup_start)?)?;
        entry.deposit(lockup_start, 10_000)?;

        let mut vested = entry.vested(lockup_start - 1)?;
        let mut amount_locked = entry.amount_locked(lockup_start - 1)?;
        let mut amount_unlocked = entry.amount_unlocked(lockup_start - 1)?;
        assert_eq!(vested, 0);
        assert_eq!(amount_locked, 10_000);
        assert_eq!(amount_unlocked, 0);

        vested = entry.vested(lockup_start + day - 1)?;
        amount_locked = entry.amount_locked(lockup_start + day - 1)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + day - 1)?;
        assert_eq!(vested, 0);
        assert_eq!(amount_locked, 10_000);
        assert_eq!(amount_unlocked, 0);

        vested = entry.vested(lockup_start + day)?;
        amount_locked = entry.amount_locked(lockup_start + day)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + day)?;
        assert_eq!(vested, 5_000);
        assert_eq!(amount_locked, 5_000);
        assert_eq!(amount_unlocked, 5_000);

        vested = entry.vested(lockup_start + day + day - 1)?;
        amount_locked = entry.amount_locked(lockup_start + day + day - 1)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + day + day - 1)?;
        assert_eq!(vested, 5_000);
        assert_eq!(amount_locked, 5_000);
        assert_eq!(amount_unlocked, 5_000);

        vested = entry.vested(lockup_start + day + day)?;
        amount_locked = entry.amount_locked(lockup_start + day + day)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + day + day)?;
        assert_eq!(vested, 10_000);
        assert_eq!(amount_locked, 0);
        assert_eq!(amount_unlocked, 10_000);

        vested = entry.vested(lockup_start + day + day + 1)?;
        amount_locked = entry.amount_locked(lockup_start + day + day + 1)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + day + day + 1)?;
        assert_eq!(vested, 10_000);
        assert_eq!(amount_locked, 0);
        assert_eq!(amount_unlocked, 10_000);

        Ok(())
    }

    #[test]
    pub fn monthly_vested_test() -> Result<()> {
        let month: i64 = i64::try_from(LockupTimeUnit::Month.seconds()).unwrap();
        let lockup_start = 1; // arbitrary point
        let mut entry = DepositEntry::new_from_lockup(Lockup::new_from_kind(Monthly(2), 0, lockup_start)?)?;
        entry.deposit(lockup_start, 10_000)?;

        let mut vested = entry.vested(lockup_start - 1)?;
        let mut amount_locked = entry.amount_locked(lockup_start - 1)?;
        let mut amount_unlocked = entry.amount_unlocked(lockup_start - 1)?;
        assert_eq!(vested, 0);
        assert_eq!(amount_locked, 10_000);
        assert_eq!(amount_unlocked, 0);

        vested = entry.vested(lockup_start + month - 1)?;
        amount_locked = entry.amount_locked(lockup_start + month - 1)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + month - 1)?;
        assert_eq!(vested, 0);
        assert_eq!(amount_locked, 10_000);
        assert_eq!(amount_unlocked, 0);

        vested = entry.vested(lockup_start + month)?;
        amount_locked = entry.amount_locked(lockup_start + month)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + month)?;
        assert_eq!(vested, 5_000);
        assert_eq!(amount_locked, 5_000);
        assert_eq!(amount_unlocked, 5_000);

        vested = entry.vested(lockup_start + month + month - 1)?;
        amount_locked = entry.amount_locked(lockup_start + month + month - 1)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + month + month - 1)?;
        assert_eq!(vested, 5_000);
        assert_eq!(amount_locked, 5_000);
        assert_eq!(amount_unlocked, 5_000);

        vested = entry.vested(lockup_start + month + month)?;
        amount_locked = entry.amount_locked(lockup_start + month + month)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + month + month)?;
        assert_eq!(vested, 10_000);
        assert_eq!(amount_locked, 0);
        assert_eq!(amount_unlocked, 10_000);

        vested = entry.vested(lockup_start + month + month + 1)?;
        amount_locked = entry.amount_locked(lockup_start + month + month + 1)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + month + month + 1)?;
        assert_eq!(vested, 10_000);
        assert_eq!(amount_locked, 0);
        assert_eq!(amount_unlocked, 10_000);

        Ok(())
    }

    #[test]
    pub fn constant_vested_test() -> Result<()> {
        let month: i64 = i64::try_from(LockupTimeUnit::Month.seconds()).unwrap();
        let lockup_start = 1; // arbitrary point
        let mut entry = DepositEntry::new_from_lockup(Lockup::new_from_kind(Constant(LockupTimeDuration{periods: 1, unit: LockupTimeUnit::Month}), 0, lockup_start)?)?;
        entry.deposit(lockup_start, 10_000)?;

        let mut vested = entry.vested(lockup_start - 1)?;
        let mut amount_locked = entry.amount_locked(lockup_start - 1)?;
        let mut amount_unlocked = entry.amount_unlocked(lockup_start - 1)?;
        assert_eq!(vested, 0);
        assert_eq!(amount_locked, 10_000);
        assert_eq!(amount_unlocked, 0);

        vested = entry.vested(lockup_start + month - 1)?;
        amount_locked = entry.amount_locked(lockup_start + month - 1)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + month - 1)?;
        assert_eq!(vested, 0);
        assert_eq!(amount_locked, 10_000);
        assert_eq!(amount_unlocked, 0);

        vested = entry.vested(lockup_start + month + 1)?;
        amount_locked = entry.amount_locked(lockup_start + month + 1)?;
        amount_unlocked = entry.amount_unlocked(lockup_start + month + 1)?;
        assert_eq!(vested, 0);
        assert_eq!(amount_locked, 10_000);
        assert_eq!(amount_unlocked, 0);

        Ok(())
    }

    #[test]
    pub fn withdraw_test() -> Result<()> {
        let day: i64 = i64::try_from(LockupTimeUnit::Day.seconds()).unwrap();
        let lockup_start = 0; // arbitrary point
        let lockup_kind = Daily(4);
        let lockup = Lockup::new_from_kind(lockup_kind, lockup_start, lockup_start)?;

        let mut entry = DepositEntry::new_from_lockup(lockup)?;
        entry.deposit(0, 10_000)?;

        let withdraw_at = lockup_start + day;
        assert_eq!(entry.amount_unlocked(withdraw_at)?, 2_500);

        // First withdrawal: amount 1000
        entry.withdraw(withdraw_at, 1_000)?;
        assert_eq!(entry.amount_deposited_native, 9000);
        assert_eq!(entry.amount_initially_locked_native, 10_000);
        assert_eq!(entry.amount_unlocked(withdraw_at)?, 1_500);
        assert_eq!(entry.lockup, lockup);

        // Second withdrawal: amount 1000
        entry.withdraw(withdraw_at, 1_000)?;
        assert_eq!(entry.amount_deposited_native, 8000);
        assert_eq!(entry.amount_initially_locked_native, 10_000);
        assert_eq!(entry.amount_unlocked(withdraw_at)?, 500);
        assert_eq!(entry.lockup, lockup);

        // Third withdrawal: amount 1000
        assert_eq!(entry.withdraw(withdraw_at, 1_000), Err(error!(VsrError::InsufficientUnlockedTokens)) as Result<()>);

        // Fourth withdrawal: amount 500
        entry.withdraw(withdraw_at, 500)?;
        assert_eq!(entry.amount_deposited_native, 7500);
        assert_eq!(entry.amount_initially_locked_native, 10_000);
        assert_eq!(entry.amount_unlocked(withdraw_at)?, 0);
        assert_eq!(entry.lockup, lockup);

        // Withdrawal after deactivate
        entry.deactivate()?;
        assert_eq!(entry.withdraw(withdraw_at, 1_000), Err(error!(VsrError::InternalProgramError)) as Result<()>);

        Ok(())
    }
}
