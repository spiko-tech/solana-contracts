use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum TokenError {
    #[error("Token is paused")]
    TokenPaused = 0,
    #[error("Sender is not whitelisted")]
    UnauthorizedFrom,
    #[error("Recipient is not whitelisted")]
    UnauthorizedTo,
    #[error("Caller is not authorized")]
    Unauthorized,
    #[error("Token has already been initialized")]
    AlreadyInitialized,
    #[error("Token has not been initialized")]
    NotInitialized,
    #[error("Invalid PDA derivation")]
    InvalidPda,
    #[error("Insufficient token balance")]
    InsufficientBalance,
    #[error("Insufficient allowance")]
    InsufficientAllowance,
    #[error("Invalid mint")]
    InvalidMint,
    #[error("Redemption contract not set")]
    RedemptionContractNotSet,
    #[error("Redemption contract mismatch")]
    RedemptionContractMismatch,
}

impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
