use anchor_lang::prelude::*;

#[error_code]
pub enum GatekeeperError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Operation is not pending")]
    NotPending,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Daily limit exceeded")]
    DailyLimitExceeded,
    #[msg("Invalid amount")]
    InvalidAmount,
}
