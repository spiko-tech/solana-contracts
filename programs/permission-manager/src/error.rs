use pinocchio::error::ProgramError;

#[derive(Clone, Debug, PartialEq)]
pub enum PermissionError {
    AlreadyInitialized = 0,
    NotInitialized,
    Unauthorized,
    InvalidRole,
    InvalidPda,
    NoPendingAdmin,
    NotPendingAdmin,
}

impl From<PermissionError> for ProgramError {
    fn from(e: PermissionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
