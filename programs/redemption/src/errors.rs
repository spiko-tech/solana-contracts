use anchor_lang::prelude::*;

#[error_code]
pub enum RedemptionError {
    #[msg("Unauthorized")]
    Unauthorized,
}
