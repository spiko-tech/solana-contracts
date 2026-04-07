use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use crate::events::emit_role_granted;
use crate::helpers::{create_pda_account, require_admin_or_role, user_perm_seeds, verify_pda};
use crate::state::{
    UserPermissions, DISCRIMINATOR_USER_PERMISSION, PERMISSION_CONFIG_SEED, USER_PERMISSION_SEED,
};

/// Grant a role to a user.
///
/// The caller must be the admin, or must hold a role that has authority
/// over the target role (e.g. WHITELISTER can grant WHITELISTED).
///
/// Accounts:
///   0. `[signer, writable]` Caller (payer if target PDA needs creation)
///   1. `[]`                 PermissionConfig PDA
///   2. `[writable]`         Target user's UserPermissions PDA (created if needed)
///   3. `[]`                 System program
///   4. `[]`                 Target user address (for PDA derivation, not a signer)
///   5. `[]`                 Caller's UserPermissions PDA (for role-hierarchy check)
///
/// Data:
///   [0] role_id (u8)
pub struct GrantRole<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub user_perms: &'a AccountView,
    pub system_program: &'a AccountView,
    pub target_user: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub role_id: u8,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for GrantRole<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, user_perms, system_program, target_user, caller_perms, ..] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        if system_program.address() != &pinocchio_system::ID {
            return Err(ProgramError::IncorrectProgramId);
        }

        Ok(Self {
            caller,
            config,
            user_perms,
            system_program,
            target_user,
            caller_perms,
            role_id: data[0],
        })
    }
}

impl<'a> GrantRole<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        verify_pda(self.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        require_admin_or_role(
            self.caller,
            self.config,
            self.caller_perms,
            self.role_id,
            program_id,
        )?;

        let user_perm_bump = verify_pda(
            self.user_perms,
            &[USER_PERMISSION_SEED, self.target_user.address().as_ref()],
            program_id,
        )?;

        let needs_creation = !self.user_perms.owned_by(program_id);

        if needs_creation {
            let bump_ref = [user_perm_bump];
            let seeds = user_perm_seeds(self.target_user.address().as_ref(), &bump_ref);
            let signers = [Signer::from(&seeds)];

            create_pda_account(
                self.caller,
                self.user_perms,
                UserPermissions::LEN,
                program_id,
                &signers,
            )?;

            let mut data = self.user_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut(&mut data)?;
            perms.discriminator = DISCRIMINATOR_USER_PERMISSION;
            perms.bump = user_perm_bump;
            perms.roles = [0u8; 32];
        }

        {
            let mut data = self.user_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut(&mut data)?;
            perms.set_role(self.role_id);
        }

        emit_role_granted(
            &self.caller.address().to_bytes(),
            &self.target_user.address().to_bytes(),
            self.role_id,
        );

        Ok(())
    }
}
