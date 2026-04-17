use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

/// Errors that may be returned by the CustodialGatekeeper program.
#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum GatekeeperError {
    /// (0) Program has already been initialized
    #[error("Program has already been initialized")]
    AlreadyInitialized = 0,
    /// (1) Program has not been initialized
    #[error("Program has not been initialized")]
    NotInitialized,
    /// (2) Caller is not authorized for this operation
    #[error("Caller is not authorized for this operation")]
    Unauthorized,
    /// (3) Operation already exists
    #[error("Operation already exists")]
    OperationExists,
    /// (4) Operation is not in PENDING status
    #[error("Operation is not in PENDING status")]
    NotPending,
    /// (5) Deadline has already passed
    #[error("Deadline has already passed")]
    DeadlinePassed,
    /// (6) Deadline has not yet passed
    #[error("Deadline has not yet passed")]
    DeadlineNotPassed,
    /// (7) Invalid max delay value
    #[error("Invalid max delay value")]
    InvalidMaxDelay,
    /// (8) Unauthorized sender (not WHITELISTED)
    #[error("Unauthorized sender (not WHITELISTED)")]
    UnauthorizedFrom,
    /// (9) Unauthorized recipient (not WHITELISTED_EXT)
    #[error("Unauthorized recipient (not WHITELISTED_EXT)")]
    UnauthorizedTo,
}

impl From<GatekeeperError> for ProgramError {
    fn from(e: GatekeeperError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
