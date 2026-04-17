use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

/// Errors that may be returned by the Permission Manager program.
#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum PermissionError {
    /// (0) Program has already been initialized
    #[error("Program has already been initialized")]
    AlreadyInitialized = 0,
    /// (1) Program has not been initialized
    #[error("Program has not been initialized")]
    NotInitialized,
    /// (2) Caller is not authorized for this operation
    #[error("Caller is not authorized for this operation")]
    Unauthorized,
    /// (3) Invalid role identifier
    #[error("Invalid role identifier")]
    InvalidRole,
    /// (4) Invalid PDA derivation
    #[error("Invalid PDA derivation")]
    InvalidPda,
    /// (5) No pending admin set
    #[error("No pending admin set")]
    NoPendingAdmin,
    /// (6) Caller is not the pending admin
    #[error("Caller is not the pending admin")]
    NotPendingAdmin,
    /// (7) Cannot grant role due to group exclusion
    #[error("Cannot grant role due to group exclusion")]
    GroupExclusion,
    /// (8) Cannot revoke role due to group protection
    #[error("Cannot revoke role due to group protection")]
    GroupProtected,
}

impl From<PermissionError> for ProgramError {
    fn from(e: PermissionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
