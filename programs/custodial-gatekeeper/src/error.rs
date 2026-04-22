use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum GatekeeperError {
    #[error("Program has already been initialized")]
    AlreadyInitialized = 0,
    #[error("Program has not been initialized")]
    NotInitialized,
    #[error("Caller is not authorized for this operation")]
    Unauthorized,
    #[error("Operation already exists")]
    OperationExists,
    #[error("Operation is not in PENDING status")]
    NotPending,
    #[error("Deadline has already passed")]
    DeadlinePassed,
    #[error("Deadline has not yet passed")]
    DeadlineNotPassed,
    #[error("Invalid max delay value")]
    InvalidMaxDelay,
    #[error("Unauthorized sender (not WHITELISTED)")]
    UnauthorizedFrom,
    #[error("Unauthorized recipient (not WHITELISTED_EXT)")]
    UnauthorizedTo,
}

impl From<GatekeeperError> for ProgramError {
    fn from(e: GatekeeperError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
