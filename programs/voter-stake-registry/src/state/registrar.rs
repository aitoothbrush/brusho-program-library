use crate::error::*;
use crate::state::lockup::*;
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

const SCALED_FACTOR_BASE: u64 = 1_000_000_000;

/// Instance of a voting rights distributor.
#[account]
pub struct Registrar {
    pub governance_program_id: Pubkey,
    pub realm: Pubkey,
    pub realm_authority: Pubkey,
    pub governing_token_mint: Pubkey,

    /// Storage for voting configuration: voting_config + reserved2.
    pub voting_config: VotingConfig,
    pub reserved2: [u8; 40],
    /// Storage for deposit configuration: deposit_config + reserved2.
    pub deposit_config: DepositConfig,
    pub reserved3: [u8; 40],

    /// Debug only: time offset, to allow tests to move forward in time.
    pub time_offset: i64,
    pub bump: u8,
    pub reserved4: [u8; 63],
}
const_assert!(std::mem::size_of::<Registrar>() == 4 * 32 + 64 + 64 + 8 + 1 + 63);
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
            return Err(error!(VsrError::InvalidVotingMint));
        }

        let mut sum = self.voting_config.baseline_vote_weight(mint.supply)?;
        sum = sum
            .checked_add(self.voting_config.max_extra_lockup_vote_weight(mint.supply)?)
            .ok_or_else(|| error!(VsrError::VoterWeightOverflow))?;
        Ok(sum)
    }
}

#[macro_export]
macro_rules! registrar_seeds {
    ( $registrar:expr ) => {
        &[
            $registrar.realm.as_ref(),
            b"registrar".as_ref(),
            $registrar.realm_governing_token_mint.as_ref(),
            &[$registrar.bump],
        ]
    };
}

pub use registrar_seeds;

/// Exchange rate for an asset that can be used to mint voting rights.
///
/// See documentation of configure_voting_mint for details on how
/// native token amounts convert to vote weight.

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
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
                (base as u128)
                    .checked_mul(factor as u128)?
                    .checked_div(SCALED_FACTOR_BASE as u128)?,
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct DepositConfig {
    pub ordinary_deposit_min_lockup_duration: LockupTimeDuration,
    pub node_deposit_lockup_duration: LockupTimeDuration,
    pub node_security_deposit: u64,
}

const_assert!(std::mem::size_of::<DepositConfig>() == 3 * 8);
const_assert!(std::mem::size_of::<DepositConfig>() % 8 == 0);