use crate::error::*;
use crate::state::lockup::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

pub const SCALED_FACTOR_BASE: u64 = 1_000_000_000;
/// Total amount of staking rewards
pub const TOTAL_REWARD_AMOUNT: u64 = 770_000_000_000_000; // 770M
/// Floor of permanently locked tokens to be met for full rewards
pub const FULL_REWARD_PERMANENTLY_LOCKED_FLOOR: u64 = 195_000_000_000_000; // 195M
/// Seconds in a year
pub const SECS_PER_YEAR: u64 = SECS_PER_DAY * 365;

/// Instance of a voting rights distributor.
#[account(zero_copy)]
pub struct Registrar {
    pub governance_program_id: Pubkey,
    pub realm: Pubkey,
    pub realm_authority: Pubkey,
    pub governing_token_mint: Pubkey,

    /// Storage for voting configuration: voting_config + reserved1.
    pub voting_config: VotingConfig,
    pub reserved1: [u64; 5],

    /// Storage for deposit configuration: deposit_config + reserved2.
    pub deposit_config: DepositConfig,
    pub reserved2: [u64; 5],

    // The current value of reward amount per second.
    pub current_reward_amount_per_second: u128,

    /// The last time 'current_reward_amount_per_second' was rotated.
    pub last_reward_amount_per_second_rotated_ts: i64,

    /// The timestamp that rewards was last accrued at
    pub reward_accrual_ts: i64,

    /// Accumulator of the total earned rewards rate since the opening
    pub reward_index: u128,

    /// Amount of rewards that were issued.
    pub issued_reward_amount: u64,

    /// Total permanently locked amount.
    /// Depositions with lockup kind 'Constant' are considered permanently locked
    pub permanently_locked_amount: u64,

    /// Debug only: time offset, to allow tests to move forward in time.
    pub time_offset: i64,

    pub bump: u8,
    pub max_voter_weight_record_bump: u8,
    pub reserved3: [u8; 14],
    pub reserved4: [u64; 9],
}
const_assert!(
    std::mem::size_of::<Registrar>() == 4 * 32 + 80 + 64 + 16 + 8 + 8 + 16 + 8 * 3 + 1 + 1 + 86
);
const_assert!(std::mem::size_of::<Registrar>() % 8 == 0);

impl Registrar {
    pub fn clock_unix_timestamp(&self) -> i64 {
        Clock::get()
            .unwrap()
            .unix_timestamp
            .checked_add(self.time_offset)
            .unwrap()
    }

    pub fn max_vote_weight(&self, mint: &Account<Mint>) -> Result<u64> {
        if mint.key() != self.governing_token_mint {
            return Err(error!(VsrError::InvalidGoverningMint));
        }

        let mut sum = self.voting_config.baseline_vote_weight(mint.supply)?;
        sum = sum
            .checked_add(
                self.voting_config
                    .max_extra_lockup_vote_weight(mint.supply)?,
            )
            .ok_or_else(|| error!(VsrError::VoterWeightOverflow))?;
        Ok(sum)
    }

    pub fn accrue_rewards(&mut self, curr_ts: i64) {
        let seconds_delta = curr_ts.checked_sub(self.reward_accrual_ts).unwrap() as u64;
        if seconds_delta == 0 {
            return;
        }

        let reward_index_delta = if self.permanently_locked_amount != 0 {
            self.current_reward_amount_per_second
                .mul_scalar(seconds_delta as core::primitive::u128)
                .div_scalar(u64::max(
                    self.permanently_locked_amount,
                    FULL_REWARD_PERMANENTLY_LOCKED_FLOOR,
                ) as core::primitive::u128)
        } else {
            u128::new(0)
        };

        let issued_reward_amount_delta = u64::try_from(
            reward_index_delta
                .mul_scalar(self.permanently_locked_amount as core::primitive::u128)
                .truncate(),
        )
        .unwrap();

        self.reward_accrual_ts = curr_ts;
        self.reward_index = self.reward_index.add(reward_index_delta);
        self.issued_reward_amount = self
            .issued_reward_amount
            .checked_add(issued_reward_amount_delta)
            .unwrap();

        self.rotate_reward_amount_per_second_if_needed(curr_ts);
    }

    fn rotate_reward_amount_per_second_if_needed(&mut self, curr_ts: i64) {
        if self.last_reward_amount_per_second_rotated_ts + SECS_PER_YEAR as i64 <= curr_ts {
            let current_annual_reward_amount = TOTAL_REWARD_AMOUNT
                .checked_sub(self.issued_reward_amount)
                .unwrap()
                .checked_mul(12)
                .unwrap()
                .checked_div(100)
                .unwrap() as core::primitive::u128;
            self.current_reward_amount_per_second =
                u128::new_with_denom(current_annual_reward_amount, SECS_PER_YEAR as core::primitive::u128);
            self.last_reward_amount_per_second_rotated_ts = curr_ts;
        }
    }
}

#[macro_export]
macro_rules! registrar_seeds {
    ( $registrar:expr ) => {
        &[
            $registrar.realm.as_ref(),
            b"registrar".as_ref(),
            $registrar.governing_token_mint.as_ref(),
            &[$registrar.bump],
        ]
    };
}

pub use registrar_seeds;

/// Exchange rate for an asset that can be used to mint voting rights.
///
/// See documentation of configure_voting_mint for details on how
/// native token amounts convert to vote weight.

#[derive(AnchorSerialize, AnchorDeserialize)]
#[zero_copy]
pub struct VotingConfig {
    /// Vote weight factor for all funds in the account, no matter if locked or not.
    ///
    /// In 1/SCALED_FACTOR_BASE units.
    pub baseline_vote_weight_scaled_factor: u64,

    /// Maximum extra vote weight factor for lockups.
    ///
    /// This is the extra votes gained for lockups lasting lockup_saturation_secs or
    /// longer. Shorter lockups receive only a fraction of the maximum extra vote weight,
    /// based on lockup_time divided by lockup_saturation_secs.
    ///
    /// In 1/SCALED_FACTOR_BASE units.
    pub max_extra_lockup_vote_weight_scaled_factor: u64,

    /// Number of seconds of lockup needed to reach the maximum lockup bonus.
    pub lockup_saturation_secs: u64,
}
const_assert!(std::mem::size_of::<VotingConfig>() == 3 * 8);
const_assert!(std::mem::size_of::<VotingConfig>() % 8 == 0);

impl VotingConfig {
    /// Apply a factor in SCALED_FACTOR_BASE units.
    fn apply_factor(base: u64, factor: u64) -> Result<u64> {
        let compute = || -> Option<u64> {
            u64::try_from(
                (base as core::primitive::u128)
                    .checked_mul(factor as core::primitive::u128)?
                    .checked_div(SCALED_FACTOR_BASE as core::primitive::u128)?,
            )
            .ok()
        };
        compute().ok_or_else(|| error!(VsrError::VoterWeightOverflow))
    }

    /// The vote weight a deposit of a number of native tokens should have.
    ///
    /// This vote_weight is a component for all funds in a voter account, no
    /// matter if locked up or not.
    pub fn baseline_vote_weight(&self, amount_native: u64) -> Result<u64> {
        Self::apply_factor(amount_native, self.baseline_vote_weight_scaled_factor)
    }

    /// The maximum extra vote weight a number of locked up native tokens can have.
    /// Will be multiplied with a factor between 0 and 1 for the lockup duration.
    pub fn max_extra_lockup_vote_weight(&self, amount_native: u64) -> Result<u64> {
        Self::apply_factor(
            amount_native,
            self.max_extra_lockup_vote_weight_scaled_factor,
        )
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
#[zero_copy]
pub struct DepositConfig {
    /// The minimal lock up duration for ordinary deposit.
    pub ordinary_deposit_min_lockup_duration: LockupTimeDuration,
    /// The lock up duration for node deposit.
    pub node_deposit_lockup_duration: LockupTimeDuration,
    /// Specific amount for node deposit.
    pub node_security_deposit: u64,
}
const_assert!(std::mem::size_of::<DepositConfig>() == 16 + 16 + 8);
const_assert!(std::mem::size_of::<DepositConfig>() % 8 == 0);

/// Wrapper of core::primitive::u128.
/// In order to avoid 16 bits alignment problem.
/// See: https://solana.stackexchange.com/questions/7720/using-u128-without-sacrificing-alignment-8
/// 
#[derive(Copy, Clone, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct u128([u8; 16]);
pub const EXP_SCALE: core::primitive::u128 = 1_000_000_000_000_000_000;

impl u128 {
    #[inline(always)]
    pub fn new(num: core::primitive::u128) -> u128 {
        u128(EXP_SCALE.checked_mul(num).unwrap().to_le_bytes())
    }

    #[inline(always)]
    pub fn new_with_denom(num: core::primitive::u128, denom: core::primitive::u128) -> u128 {
        u128(
            EXP_SCALE
                .checked_mul(num)
                .unwrap()
                .checked_div(denom)
                .unwrap()
                .to_le_bytes(),
        )
    }

    #[inline(always)]
    pub fn as_u128(&self) -> core::primitive::u128 {
        core::primitive::u128::from_le_bytes(self.0)
    }

    #[inline(always)]
    pub fn add(&self, exp: u128) -> u128 {
        u128(
            self.as_u128()
                .checked_add(exp.as_u128())
                .unwrap()
                .to_le_bytes(),
        )
    }

    #[inline(always)]
    pub fn sub(&self, exp: u128) -> u128 {
        u128(
            self.as_u128()
                .checked_sub(exp.as_u128())
                .unwrap()
                .to_le_bytes(),
        )
    }

    #[inline(always)]
    pub fn mul_scalar(&self, scalar: core::primitive::u128) -> u128 {
        u128 (
            self.as_u128().checked_mul(scalar).unwrap().to_le_bytes(),
        )
    }

    #[inline(always)]
    pub fn div_scalar(&self, scalar: core::primitive::u128) -> u128 {
        u128 (
            self.as_u128().checked_div(scalar).unwrap().to_le_bytes(),
        )
    }

    #[inline(always)]
    pub fn truncate(&self) -> core::primitive::u128 {
        self.as_u128().checked_div(EXP_SCALE).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        state::registrar::{FULL_REWARD_PERMANENTLY_LOCKED_FLOOR, TOTAL_REWARD_AMOUNT},
        EXP_SCALE, SECS_PER_DAY,
    };
    use anchor_lang::prelude::*;
    use solana_sdk::{clock::SECONDS_PER_DAY, pubkey::Pubkey, timing::SECONDS_PER_YEAR};

    use super::{
        DepositConfig, u128, LockupTimeUnit, Registrar, VotingConfig, SECS_PER_YEAR,
    };

    fn new_registrar_data() -> Registrar {
        Registrar {
            governance_program_id: Pubkey::new_unique(),
            realm: Pubkey::new_unique(),
            realm_authority: Pubkey::new_unique(),
            governing_token_mint: Pubkey::new_unique(),
            voting_config: VotingConfig {
                baseline_vote_weight_scaled_factor: 1,
                max_extra_lockup_vote_weight_scaled_factor: 1,
                lockup_saturation_secs: 1,
            },
            reserved1: [0; 5],
            deposit_config: DepositConfig {
                ordinary_deposit_min_lockup_duration: crate::LockupTimeDuration {
                    periods: 1,
                    unit: crate::LockupTimeUnit::Day,
                    filler: [0; 7]
                },
                node_deposit_lockup_duration: crate::LockupTimeDuration {
                    periods: 1,
                    unit: LockupTimeUnit::Month,
                    filler: [0; 7]
                },
                node_security_deposit: 1,
            },
            reserved2: [0; 5],
            current_reward_amount_per_second: u128::new(0),
            last_reward_amount_per_second_rotated_ts: 0,
            reward_index: u128::new(0),
            reward_accrual_ts: 0,
            issued_reward_amount: 0,
            permanently_locked_amount: 0,
            time_offset: 0,
            bump: 0,
            max_voter_weight_record_bump: 0,
            reserved3: [0; 14],
            reserved4: [0; 9],
        }
    }

    #[test]
    fn accrue_rewards_initialize_test() -> Result<()> {
        let mut registrar = new_registrar_data();

        let curr_ts = (SECS_PER_YEAR * 10) as i64;
        registrar.accrue_rewards(curr_ts);

        assert_eq!(
            (TOTAL_REWARD_AMOUNT as core::primitive::u128) * EXP_SCALE * 12 / 100 / (SECS_PER_YEAR as core::primitive::u128),
            registrar.current_reward_amount_per_second.as_u128()
        );
        assert_eq!(curr_ts, registrar.last_reward_amount_per_second_rotated_ts);
        assert_eq!(0, registrar.reward_index.as_u128());
        assert_eq!(curr_ts, registrar.reward_accrual_ts);
        assert_eq!(0, registrar.issued_reward_amount);
        assert_eq!(0, registrar.permanently_locked_amount);

        Ok(())
    }

    #[test]
    fn accrue_rewards_test() -> Result<()> {
        let mut registrar = new_registrar_data();

        let curr_ts = (SECS_PER_YEAR * 10) as i64;
        registrar.accrue_rewards(curr_ts);

        // case 1: curr_ts == registrar.reward_accrual_ts
        registrar.accrue_rewards(curr_ts);

        assert_eq!(0, registrar.reward_index.as_u128());
        assert_eq!(curr_ts, registrar.reward_accrual_ts);
        assert_eq!(0, registrar.issued_reward_amount);
        assert_eq!(0, registrar.permanently_locked_amount);

        // case 2: permanently_locked_amount == 0
        let curr_ts = curr_ts + SECS_PER_DAY as i64;
        registrar.accrue_rewards(curr_ts);

        assert_eq!(0, registrar.reward_index.as_u128());
        assert_eq!(curr_ts, registrar.reward_accrual_ts);
        assert_eq!(0, registrar.issued_reward_amount);
        assert_eq!(0, registrar.permanently_locked_amount);

        // case 3:  0 < permanently_locked_amount < FULL_REWARD_PERMANENTLY_LOCKED_FLOOR
        let curr_ts = curr_ts + SECS_PER_DAY as i64;
        registrar.permanently_locked_amount = FULL_REWARD_PERMANENTLY_LOCKED_FLOOR / 2;
        registrar.accrue_rewards(curr_ts);

        let reward_index_delta = registrar
            .current_reward_amount_per_second
            .mul_scalar(SECS_PER_DAY as core::primitive::u128)
            .div_scalar(FULL_REWARD_PERMANENTLY_LOCKED_FLOOR as core::primitive::u128);
        assert_eq!(reward_index_delta.as_u128(), registrar.reward_index.as_u128());
        assert_eq!(curr_ts, registrar.reward_accrual_ts);
        assert_eq!(
            reward_index_delta
                .mul_scalar(registrar.permanently_locked_amount as core::primitive::u128)
                .truncate() as u64,
            registrar.issued_reward_amount
        );

        // case 4:  permanently_locked_amount > FULL_REWARD_PERMANENTLY_LOCKED_FLOOR
        let curr_ts = curr_ts + SECS_PER_DAY as i64;
        let registrar_cloned = registrar.clone();
        registrar.permanently_locked_amount = FULL_REWARD_PERMANENTLY_LOCKED_FLOOR * 2;
        registrar.accrue_rewards(curr_ts);

        let reward_index_delta = registrar
            .current_reward_amount_per_second
            .mul_scalar(SECS_PER_DAY as core::primitive::u128)
            .div_scalar(registrar.permanently_locked_amount as core::primitive::u128);
        assert_eq!(
            registrar_cloned.reward_index.add(reward_index_delta).as_u128(),
            registrar.reward_index.as_u128()
        );
        assert_eq!(curr_ts, registrar.reward_accrual_ts);
        assert_eq!(
            registrar_cloned.issued_reward_amount
                + reward_index_delta
                    .mul_scalar(registrar.permanently_locked_amount as core::primitive::u128)
                    .truncate() as u64,
            registrar.issued_reward_amount
        );

        Ok(())
    }

    #[test]
    fn accrue_rewards_rotation_test() -> Result<()> {
        let mut registrar = new_registrar_data();

        // initialize
        let mut curr_ts = SECONDS_PER_YEAR as i64;
        registrar.accrue_rewards(curr_ts);
        assert_eq!(
            ((TOTAL_REWARD_AMOUNT) as core::primitive::u128) * EXP_SCALE * 12 / 100 / (SECS_PER_YEAR as core::primitive::u128),
            registrar.current_reward_amount_per_second.as_u128()
        );
        assert_eq!(curr_ts, registrar.reward_accrual_ts);

        // update issued reward amount
        registrar.issued_reward_amount += TOTAL_REWARD_AMOUNT / 10;

        // forward 364 days
        curr_ts += (364 * SECONDS_PER_DAY) as i64;
        registrar.accrue_rewards(curr_ts);
        assert_eq!(
            (TOTAL_REWARD_AMOUNT as core::primitive::u128) * EXP_SCALE * 12 / 100 / (SECS_PER_YEAR as core::primitive::u128),
            registrar.current_reward_amount_per_second.as_u128()
        );
        assert_eq!(curr_ts, registrar.reward_accrual_ts);
        // println!("{}", registrar.current_reward_amount_per_second);

        // forward 1 day more
        curr_ts += (1 * SECONDS_PER_DAY) as i64;
        registrar.accrue_rewards(curr_ts);

        assert_eq!(
            ((TOTAL_REWARD_AMOUNT - registrar.issued_reward_amount) as core::primitive::u128) * EXP_SCALE * 12
                / 100
                / (SECS_PER_YEAR as core::primitive::u128),
            registrar.current_reward_amount_per_second.as_u128()
        );
        // println!("{}", registrar.current_reward_amount_per_second);

        // set issued reward amount
        registrar.issued_reward_amount += TOTAL_REWARD_AMOUNT / 10;

        // forward 1 year
        curr_ts += SECONDS_PER_YEAR as i64;
        registrar.accrue_rewards(curr_ts);

        assert_eq!(
            ((TOTAL_REWARD_AMOUNT - registrar.issued_reward_amount) as core::primitive::u128) * EXP_SCALE * 12
                / 100
                / (SECS_PER_YEAR as core::primitive::u128),
            registrar.current_reward_amount_per_second.as_u128()
        );
        // println!("{}", registrar.current_reward_amount_per_second);

        Ok(())
    }
}
