use codama::CodamaErrors;
use pinocchio::error::ProgramError;
use thiserror::Error;

/// Errors that may be returned by the Spiko Token program.
#[derive(Clone, Debug, PartialEq, Error, CodamaErrors)]
pub enum TokenError {
    /// (0) Token is paused
    #[error("Token is paused")]
    TokenPaused = 0,
    /// (1) Sender is not whitelisted
    #[error("Sender is not whitelisted")]
    UnauthorizedFrom,
    /// (2) Recipient is not whitelisted
    #[error("Recipient is not whitelisted")]
    UnauthorizedTo,
    /// (3) Caller is not authorized
    #[error("Caller is not authorized")]
    Unauthorized,
    /// (4) Token has already been initialized
    #[error("Token has already been initialized")]
    AlreadyInitialized,
    /// (5) Token has not been initialized
    #[error("Token has not been initialized")]
    NotInitialized,
    /// (6) Invalid PDA derivation
    #[error("Invalid PDA derivation")]
    InvalidPda,
    /// (7) Insufficient token balance
    #[error("Insufficient token balance")]
    InsufficientBalance,
    /// (8) Insufficient allowance
    #[error("Insufficient allowance")]
    InsufficientAllowance,
    /// (9) Invalid mint
    #[error("Invalid mint")]
    InvalidMint,
    /// (10) Redemption contract not set
    #[error("Redemption contract not set")]
    RedemptionContractNotSet,
    /// (11) Redemption contract mismatch
    #[error("Redemption contract mismatch")]
    RedemptionContractMismatch,
}

impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
