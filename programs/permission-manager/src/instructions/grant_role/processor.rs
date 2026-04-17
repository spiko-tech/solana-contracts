use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use crate::error::PermissionError;
use crate::events::RoleGrantedEvent;
use crate::helpers::{
    create_pda_account, is_admin, require_admin_or_role, user_perm_seeds, verify_pda,
};
use crate::state::{
    UserPermissions, PERMISSION_CONFIG_SEED, ROLE_WHITELISTED, ROLE_WHITELISTED_EXT,
    USER_PERMISSION_SEED,
};
use spiko_events::EventSerialize;

use super::accounts::GrantRoleAccounts;
use super::data::GrantRoleData;

/// Grant a role to a user.
///
/// The caller must be the admin, or must hold a role that has authority
/// over the target role (e.g. WHITELISTER can grant WHITELISTED).
pub struct GrantRole<'a> {
    pub accounts: GrantRoleAccounts<'a>,
    pub data: GrantRoleData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for GrantRole<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: GrantRoleAccounts::try_from(accounts)?,
            data: GrantRoleData::try_from(data)?,
        })
    }
}

impl<'a> GrantRole<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        verify_pda(self.accounts.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        require_admin_or_role(
            self.accounts.caller,
            self.accounts.config,
            self.accounts.caller_perms,
            self.data.role_id,
            program_id,
        )?;

        let user_perm_bump = verify_pda(
            self.accounts.user_perms,
            &[
                USER_PERMISSION_SEED,
                self.accounts.target_user.address().as_ref(),
            ],
            program_id,
        )?;

        let needs_creation = !self.accounts.user_perms.owned_by(program_id);

        if needs_creation {
            let bump_ref = [user_perm_bump];
            let seeds = user_perm_seeds(self.accounts.target_user.address().as_ref(), &bump_ref);
            let signers = [Signer::from(&seeds)];

            create_pda_account(
                self.accounts.caller,
                self.accounts.user_perms,
                UserPermissions::LEN,
                program_id,
                &signers,
            )?;

            let mut data = self.accounts.user_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut_init(&mut data)?;
            perms.bump = user_perm_bump;
            perms.roles = [0u8; 32];
        }

        // Group exclusion: non-admin cannot grant WHITELISTED_EXT to a user
        // that already holds WHITELISTED, and vice versa.
        if !is_admin(self.accounts.caller, self.accounts.config, program_id) {
            let data = self.accounts.user_perms.try_borrow()?;
            let perms = UserPermissions::from_bytes(&data)?;

            let excluded = match self.data.role_id {
                ROLE_WHITELISTED => perms.has_role(ROLE_WHITELISTED_EXT),
                ROLE_WHITELISTED_EXT => perms.has_role(ROLE_WHITELISTED),
                _ => false,
            };
            if excluded {
                return Err(PermissionError::GroupExclusion.into());
            }
        }

        {
            let mut data = self.accounts.user_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut(&mut data)?;
            perms.set_role(self.data.role_id);
        }

        let event = RoleGrantedEvent::new(
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
