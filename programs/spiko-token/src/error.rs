use pinocchio::error::ProgramError;

/// Custom error codes for the Spiko Token program.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenError {
    /// Token is paused; no transfers/mints/burns allowed.
    TokenPaused = 0,
    /// Sender is not whitelisted for this token.
    UnauthorizedFrom,
    /// Receiver is not whitelisted for this token.
    UnauthorizedTo,
    /// Caller is not authorized for this operation.
    Unauthorized,
    /// Account is already initialized.
    AlreadyInitialized,
    /// Account is not initialized.
    NotInitialized,
    /// Invalid PDA seeds or bump.
    InvalidPda,
    /// Insufficient balance for transfer or burn.
    InsufficientBalance,
    /// Insufficient allowance for delegated transfer.
    InsufficientAllowance,
    /// Invalid mint account.
    InvalidMint,
    /// Redemption contract not set on TokenConfig.
    RedemptionContractNotSet,
    /// Redemption program address does not match the stored contract.
    RedemptionContractMismatch,
}

impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
