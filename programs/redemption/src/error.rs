use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum RedemptionError {
    #[error("Already initialized")]
    AlreadyInitialized = 0,
    #[error("Not initialized")]
    NotInitialized,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Not pending")]
    NotPending,
    #[error("Deadline not passed")]
    DeadlineNotPassed,
    #[error("Deadline passed")]
    DeadlinePassed,
    #[error("Operation exists")]
    OperationExists,
    #[error("Below minimum")]
    BelowMinimum,
    #[error("Invalid PDA")]
    InvalidPda,
    #[error("Invalid vault")]
    InvalidVault,
}

impl From<RedemptionError> for ProgramError {
    fn from(e: RedemptionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
