use crate::error::*;
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use std::convert::TryFrom;

/// Seconds in one day.
pub const SECS_PER_DAY: u64 = 86_400;

/// Seconds in one month.
pub const SECS_PER_MONTH: u64 = 365 * SECS_PER_DAY / 12;

/// Maximum acceptable number of lockup periods.
///
/// In the linear vesting voting power computation, a factor like
/// `periods^2 * period_secs` is used. With the current setting
/// that would be 36500^2 * SECS_PER_MONTH << 2^64.
///
/// This setting limits the maximum lockup duration for lockup methods
/// with daily periods to 200 years.
pub const MAX_LOCKUP_PERIODS: u64 = 365 * 200;

pub const MAX_LOCKUP_IN_FUTURE_SECS: i64 = 100 * 365 * 24 * 60 * 60;

#[derive(AnchorSerialize, AnchorDeserialize)]
#[zero_copy]
pub struct Lockup {
    /// Type of lockup.
    pub kind: LockupKind,

    /// Start of the lockup.
    ///
    /// Note, that if start_ts is in the future, the funds are nevertheless
    /// locked up!
    ///
    /// Similarly vote power computations don't care about start_ts and always
    /// assume the full interval from now to end_ts.
    pub start_ts: i64,
}
const_assert!(std::mem::size_of::<Lockup>() == 24 + 8);
const_assert!(std::mem::size_of::<Lockup>() % 8 == 0);

/// impl: factory function and getters
impl Lockup {
    /// Create lockup for a given lockup kind
    pub fn new_from_kind(kind: LockupKind, curr_ts: i64, start_ts: i64) -> Result<Self> {
        require_gt!(
            curr_ts + MAX_LOCKUP_IN_FUTURE_SECS,
            start_ts,
            VsrError::DepositStartTooFarInFuture
        );
        require_gte!(
            MAX_LOCKUP_PERIODS,
            kind.periods(),
            VsrError::InvalidLockupPeriod
        );
        Ok(Self {
            kind,
            start_ts,
        })
    }

    /// Create lockup for a given lockup duration
    pub fn new_from_duration(duration: LockupTimeDuration, curr_ts: i64, start_ts: i64) -> Result<Self> {
        match duration.unit {
            LockupTimeUnit::Day => {
                Lockup::new_from_kind(LockupKind::daily(duration.periods), curr_ts, start_ts)
            }
            LockupTimeUnit::Month => {
                Lockup::new_from_kind(LockupKind::monthly(duration.periods), curr_ts, start_ts)
            }
        }
    }

    /// Return the end timestamp of this lockup
    #[inline(always)]
    pub fn end_ts(&self) -> i64 {
        self.start_ts
            .checked_add(i64::try_from(self.kind.duration.seconds()).unwrap())
            .unwrap()
    }

    #[inline(always)]
    pub fn is_vesting(&self) -> bool {
        self.kind.is_vesting()
    }

}

impl Lockup {
    /// True when the lockup is finished.
    #[inline(always)]
    pub fn expired(&self, curr_ts: i64) -> bool {
        self.seconds_left(curr_ts) == 0
    }

    /// Number of seconds left in the lockup.
    /// May be more than end_ts-start_ts if curr_ts < start_ts.
    pub fn seconds_left(&self, mut curr_ts: i64) -> u64 {
        curr_ts = match self.kind.kind {
            LockupKindKind::Constant => self.start_ts,
            _ => curr_ts,
        };

        let end_ts = self.end_ts();
        if curr_ts >= end_ts {
            0
        } else {
            (end_ts - curr_ts) as u64
        }
    }

    /// Returns the number of periods left on the lockup.
    /// Returns 0 after lockup has expired and periods_total before start_ts.
    pub fn periods_left(&self, curr_ts: i64) -> Result<u64> {
        let period_secs = self.kind.period_secs();
        if period_secs == 0 {
            return Ok(0);
        }
        if curr_ts < self.start_ts {
            return Ok(self.periods_total());
        }
        Ok(self
            .seconds_left(curr_ts)
            .checked_add(period_secs.saturating_sub(1))
            .unwrap()
            .checked_div(period_secs)
            .unwrap())
    }

    /// Returns the current period in the vesting schedule.
    /// Will report periods_total() after lockup has expired and 0 before start_ts.
    #[inline]
    pub fn period_current(&self, curr_ts: i64) -> Result<u64> {
        Ok(self
            .periods_total()
            .saturating_sub(self.periods_left(curr_ts)?))
    }

    /// Returns the total amount of periods in the lockup.
    #[inline]
    pub fn periods_total(&self) -> u64 {
        self.kind.periods() as u64
    }
}

impl Default for Lockup {
    fn default() -> Self {
        Lockup {
            kind: LockupKind::constant(LockupTimeDuration {
                periods: 0,
                unit: LockupTimeUnit::Day,
                filler: [0; 7]
            }),
            start_ts: 0,
        }
    }
}

#[zero_copy]
#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct LockupKind {
    pub duration: LockupTimeDuration,
    pub kind: LockupKindKind,
    pub filler: [u8; 7]
}
const_assert!(std::mem::size_of::<LockupKind>() == 16 + 1 + 7);
const_assert!(std::mem::size_of::<LockupKind>() % 8 == 0);

impl LockupKind {
    pub fn daily(days: u64) -> LockupKind {
        LockupKind {
            duration: LockupTimeDuration { periods: days, unit: LockupTimeUnit::Day, filler: [0; 7] },
            kind: LockupKindKind::Daily,
            filler: [0; 7]
        }
    }

    pub fn monthly(months: u64) -> LockupKind {
        LockupKind {
            duration: LockupTimeDuration { periods: months, unit: LockupTimeUnit::Month, filler: [0;7] },
            kind: LockupKindKind::Monthly,
            filler: [0; 7]
        }
    }

    pub fn constant(duration: LockupTimeDuration) -> LockupKind {
        LockupKind {
            duration,
            kind: LockupKindKind::Constant,
            filler: [0; 7]
        }
    }

    #[inline(always)]
    pub fn periods(&self) -> u64 {
        self.duration.periods
    }

    /// The lockup length is specified by passing the number of lockup periods
    /// to create_deposit_entry. This describes a period's length.
    ///
    /// For vesting lockups, the period length is also the vesting period.
    #[inline(always)]
    pub fn period_secs(&self) -> u64 {
        self.duration.unit.seconds()
    }

    #[inline(always)]
    pub fn is_vesting(&self) -> bool {
        match self.kind {
            LockupKindKind::Daily => true,
            LockupKindKind::Monthly => true,
            LockupKindKind::Constant => false,
        }
    }
}

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Zeroable)]
pub enum LockupKindKind {
    /// Lock up for a number of days.
    Daily,

    /// Lock up for a number of months.
    Monthly,

    /// Lock up permanently. 
    Constant,
}

unsafe impl Pod for LockupKindKind { }

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Zeroable, Pod)]
pub struct LockupTimeDuration {
    pub periods: u64,
    pub unit: LockupTimeUnit,
    pub filler: [u8; 7]
}
const_assert!(std::mem::size_of::<LockupTimeDuration>() == 8 + 1 + 7);
const_assert!(std::mem::size_of::<LockupTimeDuration>() % 8 == 0);

impl LockupTimeDuration {
    pub fn seconds(&self) -> u64 {
        self.unit
            .seconds()
            .checked_mul(self.periods.into())
            .unwrap()
    }
}

#[repr(u8)]
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Zeroable)]
pub enum LockupTimeUnit {
    Day,
    Month,
}

unsafe impl Pod for LockupTimeUnit { }

impl LockupTimeUnit {
    pub fn seconds(&self) -> u64 {
        match *self {
            LockupTimeUnit::Day => SECS_PER_DAY,
            LockupTimeUnit::Month => SECS_PER_MONTH
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::deposit_entry::DepositEntry;

    // intentionally not a multiple of a day
    const MAX_SECS_LOCKED: u64 = 365 * 24 * 60 * 60 + 7 * 60 * 60;
    const MAX_DAYS_LOCKED: f64 = MAX_SECS_LOCKED as f64 / (24.0 * 60.0 * 60.0);

    #[test]
    pub fn period_computations() -> Result<()> {
        let lockup = Lockup::new_from_kind(LockupKind::daily(3), 1000, 1000)?;
        let day = SECS_PER_DAY as i64;
        assert_eq!(lockup.periods_total(), 3);
        assert_eq!(lockup.period_current(0)?, 0);
        assert_eq!(lockup.periods_left(0)?, 3);
        assert_eq!(lockup.period_current(999)?, 0);
        assert_eq!(lockup.periods_left(999)?, 3);
        assert_eq!(lockup.period_current(1000)?, 0);
        assert_eq!(lockup.periods_left(1000)?, 3);
        assert_eq!(lockup.period_current(1000 + day - 1)?, 0);
        assert_eq!(lockup.periods_left(1000 + day - 1)?, 3);
        assert_eq!(lockup.period_current(1000 + day)?, 1);
        assert_eq!(lockup.periods_left(1000 + day)?, 2);
        assert_eq!(lockup.period_current(1000 + 3 * day - 1)?, 2);
        assert_eq!(lockup.periods_left(1000 + 3 * day - 1)?, 1);
        assert_eq!(lockup.period_current(1000 + 3 * day)?, 3);
        assert_eq!(lockup.periods_left(1000 + 3 * day)?, 0);
        assert_eq!(lockup.period_current(100 * day)?, 3);
        assert_eq!(lockup.periods_left(100 * day)?, 0);
        Ok(())
    }

    #[test]
    pub fn days_left_start() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 10,
            days_total: 10,
            curr_day: 0.0,
        })
    }

    #[test]
    pub fn days_left_one_half() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 10,
            days_total: 10,
            curr_day: 0.5,
        })
    }

    #[test]
    pub fn days_left_one() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 9,
            days_total: 10,
            curr_day: 1.0,
        })
    }

    #[test]
    pub fn days_left_one_and_one_half() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 9,
            days_total: 10,
            curr_day: 1.5,
        })
    }

    #[test]
    pub fn days_left_9() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 1,
            days_total: 10,
            curr_day: 9.0,
        })
    }

    #[test]
    pub fn days_left_9_dot_one() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 1,
            days_total: 10,
            curr_day: 9.1,
        })
    }

    #[test]
    pub fn days_left_9_dot_nine() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 1,
            days_total: 10,
            curr_day: 9.9,
        })
    }

    #[test]
    pub fn days_left_ten() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 0,
            days_total: 10,
            curr_day: 10.0,
        })
    }

    #[test]
    pub fn days_left_eleven() -> Result<()> {
        run_test_days_left(TestDaysLeft {
            expected_days_left: 0,
            days_total: 10,
            curr_day: 11.0,
        })
    }

    #[test]
    pub fn months_left_start() -> Result<()> {
        run_test_months_left(TestMonthsLeft {
            expected_months_left: 10,
            months_total: 10,
            curr_month: 0.,
        })
    }

    #[test]
    pub fn months_left_one_half() -> Result<()> {
        run_test_months_left(TestMonthsLeft {
            expected_months_left: 10,
            months_total: 10,
            curr_month: 0.5,
        })
    }

    #[test]
    pub fn months_left_one_and_a_half() -> Result<()> {
        run_test_months_left(TestMonthsLeft {
            expected_months_left: 9,
            months_total: 10,
            curr_month: 1.5,
        })
    }

    #[test]
    pub fn months_left_ten() -> Result<()> {
        run_test_months_left(TestMonthsLeft {
            expected_months_left: 9,
            months_total: 10,
            curr_month: 1.5,
        })
    }

    #[test]
    pub fn months_left_eleven() -> Result<()> {
        run_test_months_left(TestMonthsLeft {
            expected_months_left: 0,
            months_total: 10,
            curr_month: 11.,
        })
    }

    #[test]
    pub fn voting_power_daily_warmup() -> Result<()> {
        let amount_deposited = 10 * 1_000_000;
        run_test_voting_power(TestVotingPower {
            expected_voting_power: locked_daily_power(amount_deposited, -1.5, 10),
            amount_deposited: 10 * 1_000_000, // 10 tokens with 6 decimals.
            curr_day: -1.5,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_start() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 0.0, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 0.0,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_one_half() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 0.5, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 0.5,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_one() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 1.0, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 1.0,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_one_and_one_third() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 1.3, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 1.3,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_two() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 2.0, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 2.0,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_nine() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 9.0, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 9.0,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_nine_dot_nine() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 9.9, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 9.9,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_ten() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 10.0, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 10.0,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_ten_dot_one() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 10.1, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 10.1,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_eleven() -> Result<()> {
        // 10 tokens with 6 decimals.
        let amount_deposited = 10 * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 11.0, 10);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 11.0,
            kind: LockupKind::daily(10),
        })
    }

    #[test]
    pub fn voting_power_daily_saturation() -> Result<()> {
        let days = MAX_DAYS_LOCKED.floor() as u64;
        let amount_deposited = days * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 0.0, days);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 0.0,
            kind: LockupKind::daily(MAX_DAYS_LOCKED.floor() as u64),
        })
    }

    #[test]
    pub fn voting_power_daily_above_saturation1() -> Result<()> {
        let days = (MAX_DAYS_LOCKED + 10.0).floor() as u64;
        let amount_deposited = days * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 0.0, days);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 0.0,
            kind: LockupKind::daily((MAX_DAYS_LOCKED + 10.0).floor() as u64),
        })
    }

    #[test]
    pub fn voting_power_daily_above_saturation2() -> Result<()> {
        let days = (MAX_DAYS_LOCKED + 10.0).floor() as u64;
        let amount_deposited = days * 1_000_000;
        let expected_voting_power = locked_daily_power(amount_deposited, 0.5, days);
        run_test_voting_power(TestVotingPower {
            expected_voting_power,
            amount_deposited,
            curr_day: 0.5,
            kind: LockupKind::daily((MAX_DAYS_LOCKED + 10.0).floor() as u64),
        })
    }

    struct TestDaysLeft {
        expected_days_left: u64,
        days_total: u64,
        curr_day: f64,
    }

    struct TestMonthsLeft {
        expected_months_left: u64,
        months_total: u64,
        curr_month: f64,
    }

    struct TestVotingPower {
        amount_deposited: u64,
        curr_day: f64,
        expected_voting_power: u64,
        kind: LockupKind,
    }

    fn run_test_days_left(t: TestDaysLeft) -> Result<()> {
        let start_ts = 1634929833;
        let curr_ts = start_ts + days_to_secs(t.curr_day);
        let l = Lockup {
            kind: LockupKind::daily(t.days_total),
            start_ts,
        };
        let days_left = l.periods_left(curr_ts)?;
        assert_eq!(days_left, t.expected_days_left);
        Ok(())
    }

    fn run_test_months_left(t: TestMonthsLeft) -> Result<()> {
        let start_ts = 1634929833;
        let curr_ts = start_ts + months_to_secs(t.curr_month);
        let l = Lockup {
            kind: LockupKind::monthly(t.months_total),
            start_ts,
        };
        let months_left = l.periods_left(curr_ts)?;
        assert_eq!(months_left, t.expected_months_left);
        Ok(())
    }

    fn run_test_voting_power(t: TestVotingPower) -> Result<()> {
        let start_ts = 1634929833;
        let mut d = DepositEntry::new_from_lockup(Lockup::new_from_kind(t.kind, start_ts, start_ts)?)?;
        d.deposit(start_ts, t.amount_deposited)?;
        let curr_ts = start_ts + days_to_secs(t.curr_day);
        let power = d.voting_power_locked(curr_ts, t.amount_deposited, MAX_SECS_LOCKED)?;
        assert_eq!(power, t.expected_voting_power);
        Ok(())
    }

    fn days_to_secs(days: f64) -> i64 {
        let d = (SECS_PER_DAY as f64) * days;
        d.round() as i64
    }

    fn months_to_secs(months: f64) -> i64 {
        let d = (SECS_PER_MONTH as f64) * months;
        d.round() as i64
    }

    // Calculates locked voting power. Done iteratively as a sanity check on
    // the closed form calcuation.
    //
    // deposit - the amount locked up
    // day - the current day in the lockup period
    // total_days - the number of days locked up
    fn locked_daily_power(amount: u64, day: f64, total_days: u64) -> u64 {
        if day >= total_days as f64 {
            return 0;
        }
        let days_remaining = total_days - day.floor() as u64;
        let mut total = 0f64;
        for k in 0..days_remaining {
            // We have 'days_remaining' remaining cliff-locked deposits of
            // amount / total_days each.
            let remaining_days = total_days as f64 - day - k as f64;
            total += locked_cliff_power_float(amount / total_days, remaining_days);
        }
        // the test code uses floats to compute the voting power; avoid
        // getting incurrect expected results due to floating point rounding
        (total + 0.0001).floor() as u64
    }

    fn locked_cliff_power_float(amount: u64, remaining_days: f64) -> f64 {
        let relevant_days = if remaining_days < MAX_DAYS_LOCKED as f64 {
            remaining_days
        } else {
            MAX_DAYS_LOCKED as f64
        };
        (amount as f64) * relevant_days / (MAX_DAYS_LOCKED as f64)
    }
}
