use pinocchio::error::ProgramError;

/// Custom error codes for the Spiko Transfer Hook program.
#[derive(Clone, Debug, PartialEq)]
pub enum TransferHookError {
    /// Token is paused; no transfers allowed.
    TokenPaused = 0,
    /// Sender is not whitelisted.
    UnauthorizedFrom,
    /// Receiver is not whitelisted.
    UnauthorizedTo,
    /// Unauthorized operation.
    Unauthorized,
    /// Account is already initialized.
    AlreadyInitialized,
    /// Account is not initialized.
    NotInitialized,
    /// Invalid PDA seeds or bump.
    InvalidPda,
    /// Invalid mint account.
    InvalidMint,
}

impl From<TransferHookError> for ProgramError {
    fn from(e: TransferHookError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
