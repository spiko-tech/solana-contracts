use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use crate::events::build_role_granted_event;
use crate::helpers::{create_pda_account, require_admin_or_role, user_perm_seeds, verify_pda};
use crate::state::{UserPermissions, PERMISSION_CONFIG_SEED, USER_PERMISSION_SEED};

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

        {
            let mut data = self.accounts.user_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut(&mut data)?;
            perms.set_role(self.data.role_id);
        }

        let event_data = build_role_granted_event(
            &self.accounts.caller.address().to_bytes(),
            &self.accounts.target_user.address().to_bytes(),
            self.data.role_id,
        );
        spiko_events::emit_event(
            program_id,
            self.accounts.event_authority,
            self.accounts.self_program,
            &event_data,
            crate::event_authority_pda::BUMP,
        )?;

        Ok(())
    }
}
