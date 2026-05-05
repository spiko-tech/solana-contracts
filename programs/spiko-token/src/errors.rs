use anchor_lang::prelude::*;

#[error_code]
pub enum SpTokenError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Token is paused")]
    TokenPaused,
    #[msg("Token is not paused")]
    NotPaused,
}
