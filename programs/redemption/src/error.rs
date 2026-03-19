use pinocchio::error::ProgramError;

/// Custom error codes for the Redemption program.
#[derive(Clone, Debug, PartialEq)]
pub enum RedemptionError {
    /// Account is already initialized.
    AlreadyInitialized = 0,
    /// Account is not initialized.
    NotInitialized,
    /// Caller is not authorized for this operation.
    Unauthorized,
    /// Redemption operation is not in PENDING status.
    NotPending,
    /// Redemption deadline has not passed (cannot cancel yet).
    DeadlineNotPassed,
    /// Redemption deadline has passed (cannot execute).
    DeadlinePassed,
    /// Redemption operation already exists.
    OperationExists,
    /// Amount is below the minimum for this token.
    BelowMinimum,
    /// Invalid PDA seeds or bump.
    InvalidPda,
    /// Invalid vault account.
    InvalidVault,
}

impl From<RedemptionError> for ProgramError {
    fn from(e: RedemptionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
