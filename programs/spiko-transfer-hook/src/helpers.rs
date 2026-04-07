use pinocchio::{cpi::Seed, error::ProgramError};

pub use spiko_common::{create_pda_account, verify_pda};
pub use spiko_permission::require_permission;

use crate::state::EXTRA_ACCOUNT_METAS_SEED;

/// Check that a token is not paused by reading the TokenConfig data.
#[inline]
pub fn require_not_paused(config_data: &[u8]) -> Result<(), ProgramError> {
    use spiko_token::state::TokenConfig;
    let config = TokenConfig::from_bytes(config_data)?;
    if config.is_paused() {
        return Err(crate::error::TransferHookError::TokenPaused.into());
    }
    Ok(())
}

// -----------------------------------------------------------------
// Signer seeds builder
// -----------------------------------------------------------------

/// Build Seed array for ExtraAccountMetaList PDA.
/// Seeds: ["extra-account-metas", mint_pubkey, bump]
#[inline]
pub fn extra_account_metas_seeds<'a>(mint_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(EXTRA_ACCOUNT_METAS_SEED),
        Seed::from(mint_key),
        Seed::from(bump.as_ref()),
    ]
}
