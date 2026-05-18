use anchor_lang::prelude::*;

#[error_code]
pub enum GatekeeperError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
}
