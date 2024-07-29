use crate::state::deposit_entry::DepositEntry;
use crate::state::registrar::Registrar;
use crate::{error::*, Lockup};
use anchor_lang::prelude::*;

/// User account for minting voting rights.
#[account]
pub struct Voter {
    voter_authority: Pubkey,
    registrar: Pubkey,
    deposits: [DepositEntry; 10],
    voter_bump: u8,
    voter_weight_record_bump: u8,
    reserved: [u8; 94],
}
const_assert!(std::mem::size_of::<Voter>() == 2 * 32 + 10 * 72 + 2 + 94);
const_assert!(std::mem::size_of::<Voter>() % 8 == 0);

/// impl: factory function and getters
impl Voter {
    pub fn new(
        voter_authority: Pubkey,
        registrar: Pubkey,
        voter_bump: u8,
        voter_weight_record_bump: u8,
    ) -> Voter {
        Voter {
            voter_authority,
            registrar,
            deposits: [DepositEntry::default(); 10],
            voter_bump,
            voter_weight_record_bump,
            reserved: [0; 94],
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
    pub fn get_voter_bump(&self) -> u8 {
        self.voter_bump
    }

    #[inline(always)]
    pub fn get_voter_weight_record_bump(&self) -> u8 {
        self.voter_weight_record_bump
    }
}

impl Voter {
    #[inline]
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

    #[inline]
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

    #[inline]
    pub fn activate(&mut self, index: u8, lockup: Lockup) -> Result<()> {
        let d = self.deposit_entry_at_mut(index)?;
        require!(!d.is_active(), VsrError::InternalProgramError);

        *d = DepositEntry::new_from_lockup(lockup)?;
        Ok(())
    }

    #[inline]
    pub fn deactivate(&mut self, index: u8) -> Result<()> {
        let d = self.deposit_entry_at_mut(index)?;
        d.deactivate()
    }

    #[inline]
    pub fn deposit(&mut self, index: u8, amount: u64, registrar: &Registrar) -> Result<()> {
        let d = self.deposit_entry_at_mut(index)?;
        let curr_ts = registrar.clock_unix_timestamp();
        d.deposit(curr_ts, amount)?;
        Ok(())
    }

    pub fn withdraw(&mut self, index: u8, amount: u64, registrar: &Registrar) -> Result<()> {
        let index = index as usize;
        require_gt!(
            self.deposits.len(),
            index,
            VsrError::OutOfBoundsDepositEntryIndex
        );
        let d = &mut self.deposits[index];
        let curr_ts = registrar.clock_unix_timestamp();
        d.withdraw(curr_ts, amount)?;

        Ok(())
    }

    /// The full vote weight available to the voter
    pub fn weight(&self, registrar: &Registrar) -> Result<u64> {
        let curr_ts = registrar.clock_unix_timestamp();
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

    pub fn permanently_locked(&self, registrar: &Registrar) -> Result<u64> {
        let curr_ts = registrar.clock_unix_timestamp();
        self.deposits
            .iter()
            .filter(|d| d.is_active())
            .filter(|d| !d.get_lockup().is_vesting())
            .try_fold(0u64, |sum, d| {
                Ok(sum.checked_add(d.amount_locked(curr_ts)?).unwrap())
            })
    }

    pub fn vesting_locked(&self, registrar: &Registrar) -> Result<u64> {
        let curr_ts = registrar.clock_unix_timestamp();
        self.deposits
            .iter()
            .filter(|d| d.is_active())
            .filter(|d| d.get_lockup().is_vesting())
            .try_fold(0u64, |sum, d| {
                Ok(sum.checked_add(d.amount_locked(curr_ts)?).unwrap())
            })
    }

    pub fn vesting_unlocked(&self, registrar: &Registrar) -> Result<u64> {
        let curr_ts = registrar.clock_unix_timestamp();
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
    use super::*;
    use std::fmt::Debug;

    impl Debug for DepositEntry {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Ok(())
        }
    }

    #[test]
    pub fn initialization_test() -> Result<()> {
        let voter_authority = Pubkey::new_unique();
        let registrar = Pubkey::new_unique();
        let voter = Voter::new(voter_authority, registrar, 0, 0);

        let deposits = voter.deposits;
        for i in 0..deposits.len() {
            assert_eq!(voter.deposit_entry_at(i as u8)?, &deposits[i]);
            assert!(!voter.is_active(i as u8)?)
        }

        assert_eq!(
            voter.deposit_entry_at(deposits.len() as u8),
            Err(error!(VsrError::OutOfBoundsDepositEntryIndex)) as Result<&DepositEntry>
        );
        assert_eq!(
            voter.is_active(deposits.len() as u8),
            Err(error!(VsrError::OutOfBoundsDepositEntryIndex)) as Result<bool>
        );
        Ok(())
    }

    #[test]
    pub fn activate_deactivate_test() -> Result<()> {
        let voter_authority = Pubkey::new_unique();
        let registrar = Pubkey::new_unique();
        let mut voter = Voter::new(voter_authority, registrar, 0, 0);
        let index: u8 = 0;

        assert_eq!(
            voter.activate(voter.deposits.len() as u8, Lockup::default()),
            Err(error!(VsrError::OutOfBoundsDepositEntryIndex)) as Result<()>
        );
        assert_eq!(
            voter.deactivate(voter.deposits.len() as u8),
            Err(error!(VsrError::OutOfBoundsDepositEntryIndex)) as Result<()>
        );

        assert!(!voter.is_active(index)?);

        voter.activate(index, Lockup::default())?;
        assert!(voter.is_active(index)?);
        assert_eq!(
            voter.activate(index, Lockup::default()),
            Err(error!(VsrError::InternalProgramError)) as Result<()>
        );

        voter.deactivate(index)?;
        assert!(!voter.is_active(index)?);
        assert_eq!(
            voter.deactivate(index),
            Err(error!(VsrError::InternalProgramError)) as Result<()>
        );

        Ok(())
    }

}
