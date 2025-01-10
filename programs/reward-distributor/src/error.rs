use anchor_lang::prelude::*;

#[error_code]
pub enum RdError {
    #[msg("")]
    Authorization,
    #[msg("")]
    InvalidAsset,
    #[msg("")]
    InvalidDistributionProof,
    #[msg("")]
    InvalidRecipient,
    #[msg("")]
    AlreadyClaimedPeriod,
    #[msg("")]
    InvalidRealmAuthority,
    #[msg("")]
    InvalidCanopyLength,
    #[msg("")]
    IllegalPeriod,
    #[msg("")]
    InvalidDistributorName,
    #[msg("")]
    OraclesCountExceeds,
    #[msg("")]
    InvalidOracleReport,
    #[msg("")]
    CannotReportAtPresent,
    #[msg("")]
    OracleReportsNotAvailable,
    #[msg("")]
    DistributionTreeNotActivated,
}
