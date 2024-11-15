use anchor_lang::prelude::*;

#[error_code]
pub enum BnmError {
    #[msg("")]
    InvalidMakerNameLength,
    #[msg("")]
    InvaliMetadataUrlLength,
    #[msg("")]
    InvalidRealmAuthority,
}
