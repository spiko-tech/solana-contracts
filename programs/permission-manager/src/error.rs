use pinocchio::error::ProgramError;

#[derive(Clone, Debug, PartialEq)]
pub enum PermissionError {
    /// Account is already initialized.
    AlreadyInitialized = 0,
    /// Account is not initialized.
    NotInitialized,
    /// Caller is not authorized for this operation.
    Unauthorized,
    /// Invalid role bit index.
    InvalidRole,
    /// Invalid PDA seeds or bump.
    InvalidPda,
    /// No pending ownership transfer exists.
    NoPendingAdmin,
    /// Caller is not the pending admin.
    NotPendingAdmin,
}

impl From<PermissionError> for ProgramError {
    fn from(e: PermissionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
