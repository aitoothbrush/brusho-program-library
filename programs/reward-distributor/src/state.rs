use anchor_lang::prelude::*;
use itertools::Itertools;

#[account]
#[derive(Default)]
pub struct Distributor {
    pub realm: Pubkey,
    pub realm_authority: Pubkey,
    pub rewards_mint: Pubkey,
    pub vault: Pubkey,
    pub name: String,
    pub authority: Pubkey,
    pub oracles: Vec<Pubkey>,
    pub current_period: u32,
    pub bump: u8,
}

#[macro_export]
macro_rules! distributor_seeds {
    ( $distributor:expr ) => {
        &[
            b"distributor",
            $distributor.realm.as_ref(),
            $distributor.rewards_mint.as_ref(),
            $distributor.name.as_bytes(),
            &[$distributor.bump],
        ]
    };
}

pub use distributor_seeds;

#[account]
#[derive(Default)]
pub struct DistributionTree {
    pub distributor: Pubkey,
    pub period: u32,
    pub oracle_reports: Vec<Option<OracleReport>>,
    pub bump: u8,
}

impl DistributionTree {
    pub fn oracle_choice(&self) -> Option<OracleReport> {
        let most_choice_pair = self
            .oracle_reports
            .clone()
            .into_iter()
            .flatten()
            .chunk_by(|&x| x)
            .into_iter()
            .map(|(key, group)| (key, group.count()))
            .sorted_by(|a, b| a.1.cmp(&b.1))
            .last();

        return match most_choice_pair {
            Some(choice) => {
                if choice.1 >= (self.oracle_reports.len() + 1) / 2 {
                    Some(choice.0)
                } else {
                    None
                }
            }
            None => None,
        };
    }

    /// Returns root of merkle tree if the choice can be made.
    pub fn root(&self) -> Option<[u8; 32]> {
        let choice = self.oracle_choice();
        return match choice {
            Some(choice) => Some(choice.root),
            None => None,
        };
    }

    /// Returns max depth of merkle tree if the choice can be made.
    pub fn max_depth(&self) -> Option<u32> {
        let choice = self.oracle_choice();
        return match choice {
            Some(choice) => Some(choice.max_depth),
            None => None,
        };
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, PartialEq)]
pub struct OracleReport {
    pub root: [u8; 32],
    pub max_depth: u32,
}

#[account]
#[derive(Default)]
pub struct Canopy {
    pub canopy_data: Pubkey,
    pub authority: Pubkey,
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

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::*;

    use crate::state::OracleReport;

    use super::DistributionTree;


    #[test]
    pub fn distribution_tree_choice_test() -> Result<()> {
        let mut distribution_tree = DistributionTree {
            distributor: Pubkey::default(),
            period: 1,
            oracle_reports: vec![None; 5],
            bump: 255
        };

        assert!(distribution_tree.oracle_choice().is_none());

        let oracle_report_1 = OracleReport {
            root: [1; 32],
            max_depth: 10
        };

        distribution_tree.oracle_reports[0] = Some(oracle_report_1);
        assert!(distribution_tree.oracle_choice().is_none());

        distribution_tree.oracle_reports[1] = Some(oracle_report_1);
        assert!(distribution_tree.oracle_choice().is_none());

        distribution_tree.oracle_reports[2] = Some(oracle_report_1);
        assert!(distribution_tree.oracle_choice().is_some());
        assert!(distribution_tree.oracle_choice().unwrap() == oracle_report_1);

        let oracle_report_2 = OracleReport {
            root: [2; 32],
            max_depth: 10
        };
        distribution_tree.oracle_reports[3] = Some(oracle_report_2);
        distribution_tree.oracle_reports[4] = Some(oracle_report_2);
        assert!(distribution_tree.oracle_choice().unwrap() == oracle_report_1);

        Ok(())
    }
}