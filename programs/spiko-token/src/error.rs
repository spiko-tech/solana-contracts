use pinocchio::error::ProgramError;

#[derive(Clone, Debug, PartialEq)]
pub enum TokenError {
    TokenPaused = 0,
    UnauthorizedFrom,
    UnauthorizedTo,
    Unauthorized,
    AlreadyInitialized,
    NotInitialized,
    InvalidPda,
    InsufficientBalance,
    InsufficientAllowance,
    InvalidMint,
    RedemptionContractNotSet,
    RedemptionContractMismatch,
}

impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
