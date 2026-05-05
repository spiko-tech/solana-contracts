use anchor_lang::prelude::*;

#[error_code]
pub enum RedemptionError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Operation already exists")]
    OperationAlreadyExists,
    #[msg("Operation is not pending")]
    NotPending,
    #[msg("Deadline has passed")]
    DeadlinePassed,
    #[msg("Deadline has not passed yet")]
    DeadlineNotPassed,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid operation id")]
    InvalidOperationId,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
