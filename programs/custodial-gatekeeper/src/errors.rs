use anchor_lang::prelude::*;

#[error_code]
pub enum GatekeeperError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Operation already exists")]
    OperationAlreadyExists,
    #[msg("Operation is not pending")]
    NotPending,
    #[msg("Deadline has passed")]
    DeadlinePassed,
    #[msg("Deadline has not passed")]
    DeadlineNotPassed,
    #[msg("Invalid max delay")]
    InvalidMaxDelay,
    #[msg("Unauthorized sender")]
    UnauthorizedSender,
    #[msg("Unauthorized recipient")]
    UnauthorizedRecipient,
    #[msg("Invalid operation ID")]
    InvalidOperationId,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
