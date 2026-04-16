use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum MinterError {
    #[error("Already initialized")]
    AlreadyInitialized = 0,
    #[error("Not initialized")]
    NotInitialized,
    #[error("Caller is not authorized")]
    Unauthorized,
    #[error("Invalid PDA")]
    InvalidPda,
    #[error("Daily limit exceeded")]
    DailyLimitExceeded,
    #[error("Mint operation already exists")]
    MintOperationExists,
    #[error("Mint operation not found")]
    MintOperationNotFound,
    #[error("Invalid mint operation status")]
    InvalidMintOperationStatus,
    #[error("Mint deadline passed")]
    MintDeadlinePassed,
    #[error("Mint deadline not passed")]
    MintDeadlineNotPassed,
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
}

impl From<MinterError> for ProgramError {
    fn from(e: MinterError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
