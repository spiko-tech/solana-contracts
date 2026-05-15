use anchor_lang::prelude::*;

#[error_code]
pub enum TransferHookError {
    #[msg("Unauthorized")]
    Unauthorized,
}
