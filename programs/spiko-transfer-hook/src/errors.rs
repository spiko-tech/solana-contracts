use anchor_lang::prelude::*;

#[error_code]
pub enum TransferHookError {
    #[msg("Token is paused")]
    Paused,
    #[msg("Unauthorized")]
    Unauthorized,
}
