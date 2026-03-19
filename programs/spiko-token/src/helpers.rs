use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Seed, Signer},
    error::ProgramError,
    ProgramResult,
};

use permission_manager::state::{has_role, UserPermissions, DISCRIMINATOR_USER_PERMISSION};

use crate::state::{MINT_AUTHORITY_SEED, TOKEN_CONFIG_SEED};

// -----------------------------------------------------------------
// PDA verification helpers
// -----------------------------------------------------------------

/// Verify that an account's address matches the expected PDA and return the bump.
#[inline]
pub fn verify_pda(
    account: &AccountView,
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<u8, ProgramError> {
    let (expected, bump) = Address::find_program_address(seeds, program_id);
    if account.address() != &expected {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(bump)
}

// -----------------------------------------------------------------
// PDA account creation
// -----------------------------------------------------------------

/// Create a PDA account via CPI to the system program. The PDA signs.
pub fn create_pda_account(
    payer: &AccountView,
    pda_account: &AccountView,
    space: usize,
    owner: &Address,
    signer_seeds: &[Signer],
) -> ProgramResult {
    pinocchio_system::create_account_with_minimum_balance_signed(
        pda_account,
        space,
        owner,
        payer,
        None,
        signer_seeds,
    )
}

// -----------------------------------------------------------------
// Permission check: direct role check via PDA read
// -----------------------------------------------------------------

/// Read a UserPermissions PDA from the permission_manager and check
/// that the user has the specified role.
///
/// - `user_perms_account`: UserPermissions PDA (owned by permission_manager_id)
/// - `permission_manager_id`: the permission_manager program's address
/// - `role_bit`: the role bit to check (e.g. ROLE_MINTER, ROLE_PAUSER, etc.)
/// - `error_on_fail`: the error to return if the check fails
///
/// Returns Ok(()) if authorized, Err(error_on_fail) if not.
pub fn require_permission(
    user_perms_account: &AccountView,
    permission_manager_id: &Address,
    role_bit: u8,
    error_on_fail: ProgramError,
) -> Result<(), ProgramError> {
    // Verify account is owned by the permission_manager program
    if !user_perms_account.owned_by(permission_manager_id) {
        return Err(error_on_fail);
    }

    let user_data = user_perms_account.try_borrow()?;
    if user_data.len() < UserPermissions::LEN || user_data[0] != DISCRIMINATOR_USER_PERMISSION {
        return Err(error_on_fail);
    }
    let user_perms = UserPermissions::from_bytes(&user_data)?;

    if !has_role(&user_perms.roles, role_bit) {
        return Err(error_on_fail);
    }

    Ok(())
}

/// Check that a token is not paused.
#[inline]
pub fn require_not_paused(config_data: &[u8]) -> Result<(), ProgramError> {
    use crate::state::TokenConfig;
    let config = TokenConfig::from_bytes(config_data)?;
    if config.is_paused() {
        return Err(crate::error::TokenError::TokenPaused.into());
    }
    Ok(())
}

// -----------------------------------------------------------------
// Signer seeds builders
// -----------------------------------------------------------------

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
