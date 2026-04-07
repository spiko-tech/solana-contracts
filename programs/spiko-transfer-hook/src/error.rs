use pinocchio::error::ProgramError;

#[derive(Clone, Debug, PartialEq)]
pub enum TransferHookError {
    TokenPaused = 0,
    UnauthorizedFrom,
    UnauthorizedTo,
    Unauthorized,
    AlreadyInitialized,
    NotInitialized,
    InvalidPda,
    InvalidMint,
}

impl From<TransferHookError> for ProgramError {
    fn from(e: TransferHookError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
