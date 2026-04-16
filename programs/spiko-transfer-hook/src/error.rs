use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum TransferHookError {
    #[error("Token is paused")]
    TokenPaused = 0,
    #[error("Sender is not whitelisted")]
    UnauthorizedFrom,
    #[error("Recipient is not whitelisted")]
    UnauthorizedTo,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Already initialized")]
    AlreadyInitialized,
    #[error("Not initialized")]
    NotInitialized,
    #[error("Invalid PDA")]
    InvalidPda,
    #[error("Invalid mint")]
    InvalidMint,
}

impl From<TransferHookError> for ProgramError {
    fn from(e: TransferHookError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
