use anchor_lang::prelude::*;

#[error_code]
pub enum RdError {
    #[msg("")]
    Authorization,
    #[msg("")]
    InvalidAsset,
    #[msg("")]
    InvalidProof,
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
    CannotReportAtPresent,
    #[msg("")]
    InvalidOracleReports,
    #[msg("")]
    DistributionTreeNotActivated,
}
