use anchor_lang::prelude::*;

#[error_code]
pub enum RdError {
    #[msg("")]
    InvalidAsset,
    #[msg("")]
    InvalidProof,
    #[msg("")]
    InvalidRecipient,
    #[msg("")]
    ExpiredPeriod,
    #[msg("")]
    InvalidRealmAuthority,
    #[msg("")]
    InvalidCanopyLength,
    #[msg("")]
    IllegalPeriod,
    #[msg("")]
    IllegalPeriodRewardsLimit,
    #[msg("")]
    IllegalCanopyData,
    #[msg("")]
    SecurityControl,
    #[msg("")]
    DistributionTreeIsInactive,
    #[msg("")]
    CannotSetCanopyForActiveDistributionTree,
}
