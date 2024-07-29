use anchor_lang::prelude::*;

#[error_code]
pub enum VsrError {
    // 6000 / 0x1770
    #[msg("Exchange rate must be greater than zero")]
    InvalidRate,
    // 6001 / 0x1771
    #[msg("6001")]
    RatesFull,
    // 6002 / 0x1772
    #[msg("6002")]
    InvalidVotingMint,
    // 6003 / 0x1773
    #[msg("6003")]
    DepositEntryNotFound,
    // 6004 / 0x1774
    #[msg("6004")]
    DepositEntryFull,
    // 6005 / 0x1775
    #[msg("6005")]
    VotingTokenNonZero,
    // 6006 / 0x1776
    #[msg("6006")]
    OutOfBoundsDepositEntryIndex,
    // 6007 / 0x1777
    #[msg("6007")]
    UnusedDepositEntryIndex,
    // 6008 / 0x1778
    #[msg("6008")]
    InsufficientUnlockedTokens,
    // 6009 / 0x1779
    #[msg("6009")]
    UnableToConvert,
    // 6010 / 0x177a
    #[msg("6010")]
    InvalidLockupPeriod,
    // 6011 / 0x177b
    #[msg("6011")]
    InvalidEndTs,
    // 6012 / 0x177c
    #[msg("6012")]
    InvalidDays,
    // 6013 / 0x177d
    #[msg("6013")]
    VotingMintConfigIndexAlreadyInUse,
    // 6014 / 0x177e
    #[msg("6014")]
    OutOfBoundsVotingMintConfigIndex,
    // 6015 / 0x177f
    #[msg("Exchange rate decimals cannot be larger than registrar decimals")]
    InvalidDecimals,
    // 6016 / 0x1780
    #[msg("6016")]
    InvalidToDepositAndWithdrawInOneSlot,
    // 6017 / 0x1781
    #[msg("6017")]
    ShouldBeTheFirstIxInATx,
    // 6018 / 0x1782
    #[msg("6018")]
    ForbiddenCpi,
    // 6019 / 0x1783
    #[msg("6019")]
    InvalidMint,
    // 6020 / 0x1784
    #[msg("6020")]
    DebugInstruction,
    // 6021 / 0x1785
    #[msg("6021")]
    ClawbackNotAllowedOnDeposit,
    // 6022 / 0x1786
    #[msg("6022")]
    DepositStillLocked,
    // 6023 / 0x1787
    #[msg("6023")]
    InvalidAuthority,
    // 6024 / 0x1788
    #[msg("6024")]
    InvalidTokenOwnerRecord,
    // 6025 / 0x1789
    #[msg("6025")]
    InvalidRealmAuthority,
    // 6026 / 0x178a
    #[msg("6026")]
    VoterWeightOverflow,
    // 6027 / 0x178b
    #[msg("6027")]
    LockupSaturationMustBePositive,
    // 6028 / 0x178c
    #[msg("6028")]
    VotingMintConfiguredWithDifferentIndex,
    // 6029 / 0x178d
    #[msg("6029")]
    InternalProgramError,
    // 6030 / 0x178e
    #[msg("6030")]
    InsufficientLockedTokens,
    // 6031 / 0x178f
    #[msg("6031")]
    MustKeepTokensLocked,
    // 6032 / 0x1790
    #[msg("6032")]
    InvalidLockupKind,
    // 6033 / 0x1791
    #[msg("6033")]
    InvalidChangeToClawbackDepositEntry,
    // 6034 / 0x1792
    #[msg("6034")]
    InternalErrorBadLockupVoteWeight,
    // 6035 / 0x1793
    #[msg("6035")]
    DepositStartTooFarInFuture,
    // 6036 / 0x1794
    #[msg("6036")]
    VaultTokenNonZero,
    // 6037 / 0x1795
    #[msg("6037")]
    InvalidTimestampArguments,

    #[msg("")]
    NodeDepositReservedEntryIndex,
    #[msg("")]
    InactiveDepositEntry,
    #[msg("")]
    NotOrdinaryDepositEntry,
    #[msg("")]
    NotNodeDepositEntry,
    #[msg("")]
    NodeDepositUnreleasableAtPresent,
    #[msg("")]
    DepositEntryExhausted,
    #[msg("")]
    ZeroDepositAmount,
    #[msg("")]
    NodeSecurityDepositMustBePositive,
    #[msg("")]
    DuplicateNodeDeposit,
    #[msg("")]
    ActiveDepositEntryIndex,
    #[msg("")]
    InvalidLockupDuration,
}
