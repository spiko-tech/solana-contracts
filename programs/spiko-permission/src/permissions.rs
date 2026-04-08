use pinocchio::{account::AccountView, address::Address, error::ProgramError};

use permission_manager::state::{
    has_role, PermissionConfig, UserPermissions, DISCRIMINATOR_PERMISSION_CONFIG,
    DISCRIMINATOR_USER_PERMISSION, PERMISSION_CONFIG_SEED, USER_PERMISSION_SEED,
};

use spiko_common::verify_pda;

/// Read a UserPermissions PDA from the permission_manager and check
/// that the user has the specified role.
///
/// - `expected_user`: the address of the user whose permissions are being checked;
///   the function verifies that `user_perms_account` is the PDA derived from
///   `["user_perm", expected_user]` under `permission_manager_id`.
/// - `user_perms_account`: UserPermissions PDA (owned by permission_manager_id)
/// - `permission_manager_id`: the permission_manager program's address
/// - `role_bit`: the role bit to check (e.g. ROLE_MINTER, ROLE_PAUSER, etc.)
/// - `error_on_fail`: the error to return if the check fails
///
/// Returns Ok(()) if authorized, Err(error_on_fail) if not.
pub fn require_permission(
    expected_user: &Address,
    user_perms_account: &AccountView,
    permission_manager_id: &Address,
    role_bit: u8,
    error_on_fail: ProgramError,
) -> Result<(), ProgramError> {
    // Verify account is owned by the permission_manager program
    if !user_perms_account.owned_by(permission_manager_id) {
        return Err(error_on_fail);
    }

    // Verify the PDA is derived from the expected user's address
    if verify_pda(
        user_perms_account,
        &[USER_PERMISSION_SEED, expected_user.as_ref()],
        permission_manager_id,
    )
    .is_err()
    {
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

/// Verify the caller is the admin by reading the permission_manager's
/// PermissionConfig PDA directly (cross-program read).
///
/// - `caller`: the signer account
/// - `perm_config_account`: PermissionConfig PDA (owned by permission_manager)
/// - `permission_manager_id`: the permission_manager program's address
/// - `error_on_fail`: the error to return if the check fails
///
/// Returns Ok(()) if the caller is the admin, Err(error_on_fail) otherwise.
pub fn require_admin(
    caller: &AccountView,
    perm_config_account: &AccountView,
    permission_manager_id: &Address,
    error_on_fail: ProgramError,
) -> Result<(), ProgramError> {
    // Verify the config account is owned by the permission_manager program
    if !perm_config_account.owned_by(permission_manager_id) {
        return Err(error_on_fail);
    }

    // Verify it's the actual PermissionConfig PDA
    verify_pda(
        perm_config_account,
        &[PERMISSION_CONFIG_SEED],
        permission_manager_id,
    )?;

    let data = perm_config_account.try_borrow()?;
    if data.len() < PermissionConfig::LEN || data[0] != DISCRIMINATOR_PERMISSION_CONFIG {
        return Err(error_on_fail);
    }
    let config = PermissionConfig::from_bytes(&data)?;

    if caller.address() != &config.admin {
        return Err(error_on_fail);
    }

    Ok(())
}
