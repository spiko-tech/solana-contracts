use pinocchio::{account::AccountView, address::Address, cpi::Seed, error::ProgramError};

pub use spiko_common::{create_pda_account, verify_pda};

use crate::state::{can_manage_role, PermissionConfig, UserPermissions};

/// Verify that the caller is the admin stored in PermissionConfig.
///
/// - `caller`: the signer account
/// - `config_account`: the PermissionConfig PDA (must be owned by program_id)
///
/// Returns Ok(()) if the caller's address matches config.admin.
pub fn require_admin(
    caller: &AccountView,
    config_account: &AccountView,
    program_id: &Address,
) -> Result<(), ProgramError> {
    // Verify the config account is owned by this program
    if !config_account.owned_by(program_id) {
        return Err(crate::error::PermissionError::NotInitialized.into());
    }

    let data = config_account.try_borrow()?;
    let config = PermissionConfig::from_bytes(&data)?;

    if caller.address() != &config.admin {
        return Err(crate::error::PermissionError::Unauthorized.into());
    }

    Ok(())
}

/// Verify that the caller is either the admin, or holds a role that can
/// manage `target_role` according to the hardcoded hierarchy.
///
/// - `caller`: the signer account
/// - `config_account`: the PermissionConfig PDA (must be owned by program_id)
/// - `caller_perms`: the caller's UserPermissions PDA
/// - `target_role`: the role being granted/revoked
/// - `program_id`: the permission_manager program id
///
/// Returns Ok(()) if the caller is the admin, or if the caller's
/// UserPermissions contains a role R such that `can_manage_role(R, target_role)`.
pub fn require_admin_or_role(
    caller: &AccountView,
    config_account: &AccountView,
    caller_perms: &AccountView,
    target_role: u8,
    program_id: &Address,
) -> Result<(), ProgramError> {
    // Path 1: caller is admin — can manage any role
    if require_admin(caller, config_account, program_id).is_ok() {
        return Ok(());
    }

    // Path 2: caller holds a role that can manage target_role
    if !caller_perms.owned_by(program_id) {
        return Err(crate::error::PermissionError::Unauthorized.into());
    }

    let data = caller_perms.try_borrow()?;
    let perms = UserPermissions::from_bytes(&data)?;

    // Check each defined role that could be a grantor for target_role.
    // Since the hierarchy is small and hardcoded, we only need to check
    // the specific grantor roles that map to target_role.
    // For extensibility, we iterate over the 8 currently defined roles (0..=7).
    for grantor_role in 0..=7u8 {
        if can_manage_role(grantor_role, target_role) && perms.has_role(grantor_role) {
            return Ok(());
        }
    }

    Err(crate::error::PermissionError::Unauthorized.into())
}

/// Build Seed array for a UserPermissions PDA.
/// Seeds: ["user_perm", user_pubkey, bump]
#[inline]
pub fn user_perm_seeds<'a>(user_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(crate::state::USER_PERMISSION_SEED),
        Seed::from(user_key),
        Seed::from(bump.as_ref()),
    ]
}

/// Build Seed array for the PermissionConfig PDA.
/// Seeds: ["permission_config", bump]
#[inline]
pub fn config_seeds<'a>(bump: &'a [u8; 1]) -> [Seed<'a>; 2] {
    [
        Seed::from(crate::state::PERMISSION_CONFIG_SEED),
        Seed::from(bump.as_ref()),
    ]
}
