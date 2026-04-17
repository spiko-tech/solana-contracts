use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use spiko_common::AccountDeserialize;

use crate::error::PermissionError;
use crate::events::RoleRemovedEvent;
use crate::helpers::{is_admin, require_admin_or_role, verify_pda};
use crate::state::{
    UserPermissions, PERMISSION_CONFIG_SEED, ROLE_WHITELISTED, USER_PERMISSION_SEED,
};
use spiko_events::EventSerialize;

use super::accounts::RevokeRoleAccounts;
use super::data::RevokeRoleData;

/// Revoke a role from a user.
///
/// The caller must be the admin, or must hold a role that has authority
/// over the target role (e.g. WHITELISTER can revoke WHITELISTED).
pub struct RevokeRole<'a> {
    pub accounts: RevokeRoleAccounts<'a>,
    pub data: RevokeRoleData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for RevokeRole<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: RevokeRoleAccounts::try_from(accounts)?,
            data: RevokeRoleData::try_from(data)?,
        })
    }
}

impl<'a> RevokeRole<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        verify_pda(self.accounts.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        require_admin_or_role(
            self.accounts.caller,
            self.accounts.config,
            self.accounts.caller_perms,
            self.data.role_id,
            program_id,
        )?;

        if !self.accounts.user_perms.owned_by(program_id) {
            return Err(PermissionError::NotInitialized.into());
        }

        verify_pda(
            self.accounts.user_perms,
            &[
                USER_PERMISSION_SEED,
                self.accounts.target_user.address().as_ref(),
            ],
            program_id,
        )?;

        // Group protection: non-admin cannot revoke WHITELISTED (protected group).
        if self.data.role_id == ROLE_WHITELISTED
            && !is_admin(self.accounts.caller, self.accounts.config, program_id)
        {
            return Err(PermissionError::GroupProtected.into());
        }

        {
            let mut data = self.accounts.user_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut(&mut data)?;
            perms.clear_role(self.data.role_id);
        }

        let event = RoleRemovedEvent::new(
            self.accounts.caller.address().clone(),
            self.accounts.target_user.address().clone(),
            self.data.role_id,
        );
        spiko_events::emit_event(
            program_id,
            self.accounts.event_authority,
            self.accounts.self_program,
            &event.to_bytes(),
            crate::events::event_authority_pda::BUMP,
        )?;

        Ok(())
    }
}
