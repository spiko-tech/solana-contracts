use pinocchio::{account::AccountView, cpi::Seed, error::ProgramError};
use spiko_common::AccountDeserialize;

pub use spiko_common::{create_pda_account, verify_pda};
pub use spiko_permission::{require_admin, require_permission};

use crate::state::{MINT_AUTHORITY_SEED, TOKEN_CONFIG_SEED};

/// Read the decimals value from a Token-2022 mint account.
/// In the SPL mint layout, decimals is at byte offset 44.
#[inline]
pub fn read_mint_decimals(mint: &AccountView) -> Result<u8, ProgramError> {
    let data = mint.try_borrow()?;
    if data.len() < 45 {
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(data[44])
}

#[inline]
pub fn require_not_paused(config_data: &[u8]) -> Result<(), ProgramError> {
    use crate::state::TokenConfig;
    let config = TokenConfig::from_bytes(config_data)?;
    if config.is_paused() {
        return Err(crate::error::TokenError::TokenPaused.into());
    }
    Ok(())
}

/// Build Seed array for a TokenConfig PDA.
/// Seeds: ["token_config", mint_pubkey, bump]
#[inline]
pub fn token_config_seeds<'a>(mint_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(TOKEN_CONFIG_SEED),
        Seed::from(mint_key),
        Seed::from(bump.as_ref()),
    ]
}

/// Build Seed array for the Mint Authority PDA.
/// Seeds: ["mint_authority", mint_pubkey, bump]
#[inline]
pub fn mint_authority_seeds<'a>(mint_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(MINT_AUTHORITY_SEED),
        Seed::from(mint_key),
        Seed::from(bump.as_ref()),
    ]
}
