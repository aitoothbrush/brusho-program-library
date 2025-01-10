use anchor_lang::prelude::*;

#[error_code]
pub enum BnmError {
    #[msg("")]
    InvalidMakerNameLength,
    #[msg("")]
    InvalidBrushNoLength,
    #[msg("")]
    InvalidMetadataUrlLength,
    #[msg("")]
    InvalidRealmAuthority,
    #[msg("")]
    InactiveMaker,
}
