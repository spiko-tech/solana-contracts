use anchor_lang::prelude::*;

#[error_code]
pub enum MinterError {
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
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid operation id")]
    InvalidOperationId,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
