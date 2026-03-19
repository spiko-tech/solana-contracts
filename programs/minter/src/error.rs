use pinocchio::error::ProgramError;

/// Custom error codes for the Minter program.
#[derive(Clone, Debug, PartialEq)]
pub enum MinterError {
    /// Account is already initialized.
    AlreadyInitialized = 0,
    /// Account is not initialized.
    NotInitialized,
    /// Caller is not authorized for this operation.
    Unauthorized,
    /// Mint operation is not in PENDING status.
    NotPending,
    /// Mint operation deadline has passed.
    DeadlinePassed,
    /// Mint operation deadline has not passed yet (for cancel of expired).
    DeadlineNotPassed,
    /// Mint operation already exists.
    OperationExists,
    /// Mint operation has already been completed or canceled.
    OperationFinalized,
    /// Invalid PDA seeds or bump.
    InvalidPda,
    /// Invalid daily limit value.
    InvalidLimit,
    /// Arithmetic overflow.
    ArithmeticOverflow,
}

impl From<MinterError> for ProgramError {
    fn from(e: MinterError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
