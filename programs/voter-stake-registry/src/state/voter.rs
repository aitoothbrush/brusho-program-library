use crate::state::deposit_entry::DepositEntry;
use crate::state::registrar::Registrar;
use crate::{error::*, u128, Lockup};
use anchor_lang::prelude::*;

/// User account for minting voting rights.
#[account(zero_copy)]
pub struct Voter {
    voter_authority: Pubkey,
    registrar: Pubkey,
    deposits: [DepositEntry; 16],

    /// Global reward_index as of the most recent balance-changing action
    reward_index: u128,
    /// Rewards amount available for claim
    reward_claimable_amount: u64,

    voter_bump: u8,
    voter_weight_record_bump: u8,
    reserved1: [u8; 6],
    reserved2: [u64; 8],
}
const_assert!(std::mem::size_of::<Voter>() == 2 * 32 + 16 * 88 + 16 + 8 + 1 + 1 + 6 + 64);
const_assert!(std::mem::size_of::<Voter>() % 8 == 0);

/// impl: factory function and getters
impl Voter {
    pub fn new(
        voter_authority: Pubkey,
        registrar: Pubkey,
        reward_index: u128,
        voter_bump: u8,
        voter_weight_record_bump: u8,
    ) -> Voter {
        Voter {
            voter_authority,
            registrar,
            deposits: [DepositEntry::default(); 16],
            reward_index,
            reward_claimable_amount: 0,
            voter_bump,
            voter_weight_record_bump,
            reserved1: [0; 6],
            reserved2: [0; 8],
        }
    }

    #[inline(always)]
    pub fn get_voter_authority(&self) -> Pubkey {
        self.voter_authority
    }

    #[inline(always)]
    pub fn get_registrar(&self) -> Pubkey {
        self.registrar
    }

    #[inline(always)]
    pub fn get_deposits(&self) -> &[DepositEntry] {
        &self.deposits
    }

    #[inline(always)]
    pub fn get_reward_index(&self) -> u128 {
        self.reward_index
    }

    #[inline(always)]
    pub fn get_reward_claimable_amount(&self) -> u64 {
        self.reward_claimable_amount
    }

    #[inline(always)]
    pub fn get_voter_bump(&self) -> u8 {
        self.voter_bump
    }

    #[inline(always)]
    pub fn get_voter_weight_record_bump(&self) -> u8 {
        self.voter_weight_record_bump
    }
}

impl Voter {
    pub fn deposit_entry_at(&self, index: u8) -> Result<&DepositEntry> {
        let index = index as usize;
        require_gt!(
            self.deposits.len(),
            index,
            VsrError::OutOfBoundsDepositEntryIndex
        );
        let d = &self.deposits[index];
        Ok(d)
    }

    #[inline]
    pub fn is_active(&self, index: u8) -> Result<bool> {
        let d = self.deposit_entry_at(index)?;
        Ok(d.is_active())
    }

    fn deposit_entry_at_mut(&mut self, index: u8) -> Result<&mut DepositEntry> {
        let index = index as usize;
        require_gt!(
            self.deposits.len(),
            index,
            VsrError::OutOfBoundsDepositEntryIndex
        );
        let d = &mut self.deposits[index];
        Ok(d)
    }

    fn accrue_rewards(&mut self, curr_ts: i64, registrar: &Registrar) -> Result<()> {
        require_eq!(
            curr_ts,
            registrar.reward_accrual_ts,
            VsrError::InternalProgramError
        );

        if registrar.reward_index.as_u128() > self.reward_index.as_u128() {
            let permanently_locked = self.permanently_locked(curr_ts)?;

            self.reward_claimable_amount = self
                .reward_claimable_amount
                .checked_add(
                    u64::try_from(
                        registrar
                            .reward_index
                            .sub(self.reward_index)
                            .mul_scalar(permanently_locked as core::primitive::u128)
                            .truncate()
                    )
                    .unwrap(),
                )
                .unwrap();
            self.reward_index = registrar.reward_index;
        }

        Ok(())
    }

    pub fn activate(
        &mut self,
        index: u8,
        curr_ts: i64,
        lockup: Lockup,
        registrar: &mut Registrar,
    ) -> Result<()> {
        self.accrue_rewards(curr_ts, registrar)?;

        let d = self.deposit_entry_at_mut(index)?;
        require!(!d.is_active(), VsrError::InternalProgramError);

        *d = DepositEntry::new_from_lockup(lockup)?;
        Ok(())
    }

    pub fn deactivate(&mut self, index: u8, curr_ts: i64, registrar: &mut Registrar) -> Result<()> {
        self.accrue_rewards(curr_ts, registrar)?;

        let d = self.deposit_entry_at_mut(index)?;
        // Deduct the permanent lock amount if it's lockup is not vesting kind
        if !d.get_lockup().is_vesting() {
            registrar.permanently_locked_amount = registrar
                .permanently_locked_amount
                .checked_sub(d.get_amount_deposited_native())
                .unwrap();
        }

        d.deactivate()?;
        Ok(())
    }

    pub fn deposit(
        &mut self,
        index: u8,
        curr_ts: i64,
        amount: u64,
        registrar: &mut Registrar,
    ) -> Result<()> {
        self.accrue_rewards(curr_ts, registrar)?;

        let d = self.deposit_entry_at_mut(index)?;
        d.deposit(curr_ts, amount)?;

        // Accumulate the permanent lock amount if it's lockup is not vesting kind
        if !d.get_lockup().is_vesting() {
            registrar.permanently_locked_amount = registrar
                .permanently_locked_amount
                .checked_add(amount)
                .unwrap();
        }

        Ok(())
    }

    pub fn withdraw(
        &mut self,
        index: u8,
        curr_ts: i64,
        amount: u64,
        registrar: &mut Registrar,
    ) -> Result<u64> {
        self.accrue_rewards(curr_ts, registrar)?;

        let d = self.deposit_entry_at_mut(index)?;
        d.withdraw(curr_ts, amount)?;

        Ok(d.get_amount_deposited_native())
    }

    pub fn claim_reward(&mut self, curr_ts: i64, registrar: &mut Registrar) -> Result<u64> {
        self.accrue_rewards(curr_ts, registrar)?;

        let claimed_amount = self.reward_claimable_amount;
        self.reward_claimable_amount = 0;

        Ok(claimed_amount)
    }

    /// The full vote weight available to the voter
    pub fn weight(&self, curr_ts: i64, registrar: &Registrar) -> Result<u64> {
        self.deposits
            .iter()
            .filter(|d| d.is_active())
            .try_fold(0u64, |sum, d| {
                d.voting_power(&registrar.voting_config, curr_ts)
                    .map(|vp| sum.checked_add(vp).unwrap())
            })
    }

    /// The vote weight available to the voter when ignoring any lockup effects
    pub fn weight_baseline(&self, registrar: &Registrar) -> Result<u64> {
        self.deposits
            .iter()
            .filter(|d| d.is_active())
            .try_fold(0u64, |sum, d| {
                registrar
                    .voting_config
                    .baseline_vote_weight(d.get_amount_deposited_native())
                    .map(|vp| sum.checked_add(vp).unwrap())
            })
    }

    pub fn amount_deposited_native(&self) -> u64 {
        self.deposits.iter().fold(0u64, |sum, d| {
            sum.checked_add(d.get_amount_deposited_native()).unwrap()
        })
    }

    pub fn permanently_locked(&self, curr_ts: i64) -> Result<u64> {
        self.deposits
            .iter()
            .filter(|d| d.is_active())
            .filter(|d| !d.get_lockup().is_vesting())
            .try_fold(0u64, |sum, d| {
                Ok(sum.checked_add(d.amount_locked(curr_ts)?).unwrap())
            })
    }

    pub fn vesting_locked(&self, curr_ts: i64) -> Result<u64> {
        self.deposits
            .iter()
            .filter(|d| d.is_active())
            .filter(|d| d.get_lockup().is_vesting())
            .try_fold(0u64, |sum, d| {
                Ok(sum.checked_add(d.amount_locked(curr_ts)?).unwrap())
            })
    }

    pub fn vesting_unlocked(&self, curr_ts: i64) -> Result<u64> {
        self.deposits
            .iter()
            .filter(|d| d.is_active())
            .filter(|d| d.get_lockup().is_vesting())
            .try_fold(0u64, |sum, d| {
                Ok(sum.checked_add(d.amount_unlocked(curr_ts)?).unwrap())
            })
    }
}

#[macro_export]
macro_rules! voter_seeds {
    ( $voter:expr ) => {
        &[
            $voter.get_registrar().as_ref(),
            b"voter".as_ref(),
            $voter.get_voter_authority().as_ref(),
            &[$voter.get_voter_bump()],
        ]
    };
}

pub use voter_seeds;

#[cfg(test)]
mod tests {
    use crate::{DepositConfig, LockupKind, LockupTimeUnit, VotingConfig};

    use super::*;

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
            issued_reward_amount: 0,
            reward_index: u128::new(0),
            reward_accrual_ts: 0,
            permanently_locked_amount: 0,
            time_offset: 0,
            bump: 0,
            max_voter_weight_record_bump: 0,
            reserved3: [0; 14],
            reserved4: [0; 9],
        }
    }

    #[test]
    pub fn activate_deactivate_test() -> Result<()> {
        let voter_authority = Pubkey::new_unique();
        let registrar = Pubkey::new_unique();
        let mut registrar_data = new_registrar_data();
        let mut voter = Voter::new(voter_authority, registrar, u128::new(0), 0, 0);
        let index: u8 = 0;

        assert_eq!(
            voter.activate(
                voter.deposits.len() as u8,
                0,
                Lockup::default(),
                &mut registrar_data
            ),
            Err(error!(VsrError::OutOfBoundsDepositEntryIndex)) as Result<()>
        );
        assert_eq!(
            voter.deactivate(voter.deposits.len() as u8, 0, &mut registrar_data),
            Err(error!(VsrError::OutOfBoundsDepositEntryIndex)) as Result<()>
        );

        assert!(!voter.is_active(index)?);

        voter.activate(index, 0, Lockup::default(), &mut registrar_data)?;
        assert!(voter.is_active(index)?);
        assert_eq!(
            voter.activate(index, 0, Lockup::default(), &mut registrar_data),
            Err(error!(VsrError::InternalProgramError)) as Result<()>
        );

        voter.deactivate(index, 0, &mut registrar_data)?;
        assert!(!voter.is_active(index)?);
        assert_eq!(
            voter.deactivate(index, 0, &mut registrar_data),
            Err(error!(VsrError::InternalProgramError)) as Result<()>
        );

        Ok(())
    }

    #[test]
    pub fn deposit_test() -> Result<()> {
        let voter_authority = Pubkey::new_unique();
        let registrar = Pubkey::new_unique();
        let mut registrar_data = new_registrar_data();
        let mut voter = Voter::new(voter_authority, registrar, u128::new(0), 0, 0);
        assert_eq!(
            voter.deposit(voter.deposits.len() as u8, 0, 100, &mut registrar_data),
            Err(error!(VsrError::OutOfBoundsDepositEntryIndex)) as Result<()>
        );

        // index 0
        let lockup_0 = Lockup::new_from_kind(
            LockupKind::constant(crate::LockupTimeDuration {
                periods: 1,
                unit: LockupTimeUnit::Day,
                filler: [0; 7]
            }),
            0,
            0,
        )?;
        voter.activate(0, 0, lockup_0, &mut registrar_data)?;
        voter.deposit(0, 0, 100, &mut registrar_data)?;

        assert_eq!(
            voter
                .deposit_entry_at(0)
                .unwrap()
                .get_amount_deposited_native(),
            100
        );
        assert_eq!(registrar_data.permanently_locked_amount, 100);

        // index 1
        let lockup_1 = Lockup::new_from_kind(LockupKind::daily(1), 0, 0)?;
        voter.activate(1, 0, lockup_1, &mut registrar_data)?;
        voter.deposit(1, 0, 100, &mut registrar_data)?;

        assert_eq!(
            voter
                .deposit_entry_at(1)
                .unwrap()
                .get_amount_deposited_native(),
            100
        );
        assert_eq!(registrar_data.permanently_locked_amount, 100);

        // index 2
        let lockup_2 = Lockup::new_from_kind(
            LockupKind::monthly(1),
            0,
            0,
        )?;
        voter.activate(2, 0, lockup_2, &mut registrar_data)?;
        voter.deposit(2, 0, 100, &mut registrar_data)?;

        assert_eq!(
            voter
                .deposit_entry_at(2)
                .unwrap()
                .get_amount_deposited_native(),
            100
        );
        assert_eq!(registrar_data.permanently_locked_amount, 100);

        // index 3
        let lockup_3 = Lockup::new_from_kind(
            LockupKind::constant(crate::LockupTimeDuration {
                periods: 1,
                unit: LockupTimeUnit::Day,
                filler: [0; 7]
            }),
            0,
            0,
        )?;
        voter.activate(3, 0, lockup_3, &mut registrar_data)?;
        voter.deposit(3, 0, 100, &mut registrar_data)?;

        assert_eq!(
            voter
                .deposit_entry_at(3)
                .unwrap()
                .get_amount_deposited_native(),
            100
        );
        assert_eq!(registrar_data.permanently_locked_amount, 200);

        Ok(())
    }

    #[test]
    pub fn deactivate_test() -> Result<()> {
        let voter_authority = Pubkey::new_unique();
        let registrar = Pubkey::new_unique();
        let mut registrar_data = new_registrar_data();
        let mut voter = Voter::new(voter_authority, registrar, u128::new(0), 0, 0);
        assert_eq!(
            voter.deposit(voter.deposits.len() as u8, 0, 100, &mut registrar_data),
            Err(error!(VsrError::OutOfBoundsDepositEntryIndex)) as Result<()>
        );

        // index 0
        let lockup_0 = Lockup::new_from_kind(
            LockupKind::constant(crate::LockupTimeDuration {
                periods: 1,
                unit: LockupTimeUnit::Day,
                filler: [0; 7]
            }),
            0,
            0,
        )?;
        voter.activate(0, 0, lockup_0, &mut registrar_data)?;
        voter.deposit(0, 0, 100, &mut registrar_data)?;

        // index 1
        let lockup_1 = Lockup::new_from_kind(LockupKind::daily(1), 0, 0)?;
        voter.activate(1, 0, lockup_1, &mut registrar_data)?;
        voter.deposit(1, 0, 100, &mut registrar_data)?;

        // index 2
        let lockup_2 = Lockup::new_from_kind(LockupKind::monthly(1), 0, 0)?;
        voter.activate(2, 0, lockup_2, &mut registrar_data)?;
        voter.deposit(2, 0, 100, &mut registrar_data)?;

        assert_eq!(registrar_data.permanently_locked_amount, 100);

        // deactivate index 1
        voter.deactivate(1, 0, &mut registrar_data)?;

        // registrar_data.permanently_locked_amount not changed
        assert_eq!(registrar_data.permanently_locked_amount, 100);

        // deactivate index 2
        voter.deactivate(2, 0, &mut registrar_data)?;

        // registrar_data.permanently_locked_amount not changed
        assert_eq!(registrar_data.permanently_locked_amount, 100);

        // deactivate index 0
        voter.deactivate(0, 0, &mut registrar_data)?;

        // registrar_data.permanently_locked_amount changed
        assert_eq!(registrar_data.permanently_locked_amount, 0);

        Ok(())
    }

    #[test]
    fn accrue_rewards_test() -> Result<()> {
        let voter_authority = Pubkey::new_unique();
        let registrar = Pubkey::new_unique();
        let mut registrar_data = new_registrar_data();
        registrar_data.reward_accrual_ts = 1;
        registrar_data.reward_index = u128::new_with_denom(1, 10);

        let mut voter = Voter::new(voter_authority, registrar, u128::new(0), 0, 0);

        // index 0
        let lockup_0 = Lockup::new_from_kind(
            LockupKind::constant(crate::LockupTimeDuration {
                periods: 1,
                unit: LockupTimeUnit::Day,
                filler: [0; 7]
            }),
            0,
            0,
        )?;

        // Error happens if curr_ts != registrar.reward_accrual_ts
        assert_eq!(
            voter.activate(0, 0, lockup_0, &mut registrar_data),
            Err(error!(VsrError::InternalProgramError)) as Result<()>
        );

        voter.activate(0, 1, lockup_0, &mut registrar_data)?;
        assert_eq!(voter.reward_claimable_amount, 0);
        assert_eq!(voter.reward_index.as_u128(), registrar_data.reward_index.as_u128());

        // Increase registrar_data.reward_index
        registrar_data.reward_index = registrar_data.reward_index.add(u128::new_with_denom(1, 10));
        // Error happens if curr_ts != registrar.reward_accrual_ts
        assert_eq!(
            voter.deposit(0, 0, 100, &mut registrar_data),
            Err(error!(VsrError::InternalProgramError)) as Result<()>
        );

        // Successfully deposited 100
        voter.deposit(0, 1, 100, &mut registrar_data)?;
        assert_eq!(voter.reward_claimable_amount, 0);
        assert_eq!(voter.reward_index.as_u128(), registrar_data.reward_index.as_u128());

        // Increase registrar_data.reward_index
        registrar_data.reward_index = registrar_data.reward_index.add(u128::new_with_denom(1, 10));

        // Error happens if curr_ts != registrar.reward_accrual_ts
        assert_eq!(
            voter.withdraw(0, 0, 0, &mut registrar_data),
            Err(error!(VsrError::InternalProgramError)) as Result<u64>
        );
        voter.withdraw(0, 1, 0, &mut registrar_data)?;
        assert_eq!(voter.reward_claimable_amount, 10);
        assert_eq!(voter.reward_index.as_u128(), registrar_data.reward_index.as_u128());

        // Increase registrar_data.reward_index
        registrar_data.reward_index = registrar_data.reward_index.add(u128::new_with_denom(1, 10));
        assert_eq!(
            voter.claim_reward(0, &mut registrar_data),
            Err(error!(VsrError::InternalProgramError)) as Result<u64>
        );
        let claimed_amount = voter.claim_reward(1, &mut registrar_data)?;
        assert_eq!(claimed_amount, 20);
        assert_eq!(voter.reward_claimable_amount, 0);
        assert_eq!(voter.reward_index.as_u128(), registrar_data.reward_index.as_u128());

        // Increase registrar_data.reward_index
        registrar_data.reward_index = registrar_data.reward_index.add(u128::new_with_denom(1, 10));
        assert_eq!(
            voter.deactivate(0, 0, &mut registrar_data),
            Err(error!(VsrError::InternalProgramError)) as Result<()>
        );
        voter.deactivate(0, 1, &mut registrar_data)?;
        assert_eq!(voter.reward_claimable_amount, 10);
        assert_eq!(voter.reward_index.as_u128(), registrar_data.reward_index.as_u128());

        Ok(())
    }
}
