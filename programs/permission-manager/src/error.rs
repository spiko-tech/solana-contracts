use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum PermissionError {
    #[error("Program has already been initialized")]
    AlreadyInitialized = 0,
    #[error("Program has not been initialized")]
    NotInitialized,
    #[error("Caller is not authorized for this operation")]
    Unauthorized,
    #[error("Invalid role identifier")]
    InvalidRole,
    #[error("Invalid PDA derivation")]
    InvalidPda,
    #[error("No pending admin set")]
    NoPendingAdmin,
    #[error("Caller is not the pending admin")]
    NotPendingAdmin,
    #[error("Cannot grant role due to group exclusion")]
    GroupExclusion,
    #[error("Cannot revoke role due to group protection")]
    GroupProtected,
}

impl From<PermissionError> for ProgramError {
    fn from(e: PermissionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
