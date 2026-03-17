use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use crate::error::PermissionError;
use crate::helpers::{require_admin_or_role, verify_pda};
use crate::state::{UserPermissions, PERMISSION_CONFIG_SEED, USER_PERMISSION_SEED};

/// Revoke a role from a user.
///
/// The caller must be the admin, or must hold a role that has authority
/// over the target role (e.g. WHITELISTER can revoke WHITELISTED).
///
/// Accounts:
///   0. `[signer]`   Caller
///   1. `[]`         PermissionConfig PDA
///   2. `[writable]` Target user's UserPermissions PDA
///   3. `[]`         Target user address (for PDA derivation, not a signer)
///   4. `[]`         Caller's UserPermissions PDA (for role-hierarchy check)
///
/// Data:
///   [0] role_id (u8)
pub struct RevokeRole<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub user_perms: &'a AccountView,
    pub target_user: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub role_id: u8,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for RevokeRole<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, user_perms, target_user, caller_perms, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self {
            caller,
            config,
            user_perms,
            target_user,
            caller_perms,
            role_id: data[0],
        })
    }
}

impl<'a> RevokeRole<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Verify PermissionConfig PDA
        verify_pda(self.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        // 2. Check caller is admin OR holds a role that can manage target role
        require_admin_or_role(
            self.caller,
            self.config,
            self.caller_perms,
            self.role_id,
            program_id,
        )?;

        // 3. Verify target user's UserPermissions PDA is owned by this program
        if !self.user_perms.owned_by(program_id) {
            return Err(PermissionError::NotInitialized.into());
        }

        // 4. Verify the PDA address matches
        verify_pda(
            self.user_perms,
            &[USER_PERMISSION_SEED, self.target_user.address().as_ref()],
            program_id,
        )?;

        // 5. Clear the role bit
        {
            let mut data = self.user_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut(&mut data)?;
            perms.clear_role(self.role_id);
        }

        pinocchio_log::log!("RoleRevoked: role={}", self.role_id as u64);

        Ok(())
    }
}
