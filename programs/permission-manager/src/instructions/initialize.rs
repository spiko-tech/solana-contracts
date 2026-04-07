use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use crate::error::PermissionError;
use crate::events::emit_permission_manager_initialized;
use crate::helpers::{config_seeds, create_pda_account, user_perm_seeds, verify_pda};
use crate::state::{
    PermissionConfig, UserPermissions, DISCRIMINATOR_PERMISSION_CONFIG,
    DISCRIMINATOR_USER_PERMISSION, PERMISSION_CONFIG_SEED, USER_PERMISSION_SEED, ZERO_ADDRESS,
};

/// Initialize the PermissionManager singleton config.
///
/// Accounts:
///   0. `[signer, writable]` Admin / deployer (payer, becomes initial admin)
///   1. `[writable]`         PermissionConfig PDA (to be created)
///   2. `[writable]`         Admin's UserPermissions PDA (to be created)
///   3. `[]`                 System program
pub struct Initialize<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub admin_perms: &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Initialize<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [admin, config, admin_perms, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Verify system program
        if system_program.address() != &pinocchio_system::ID {
            return Err(ProgramError::IncorrectProgramId);
        }

        Ok(Self {
            admin,
            config,
            admin_perms,
            system_program,
        })
    }
}

impl<'a> Initialize<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Derive and verify PermissionConfig PDA
        let config_bump = verify_pda(self.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        // Check config is not already initialized (should be owned by system program)
        if self.config.owned_by(program_id) {
            return Err(PermissionError::AlreadyInitialized.into());
        }

        // 2. Derive and verify admin's UserPermissions PDA
        let admin_perm_bump = verify_pda(
            self.admin_perms,
            &[USER_PERMISSION_SEED, self.admin.address().as_ref()],
            program_id,
        )?;

        // 3. Create the PermissionConfig PDA account
        let config_bump_ref = [config_bump];
        let config_signer_seeds = config_seeds(&config_bump_ref);
        let config_signers = [Signer::from(&config_signer_seeds)];

        create_pda_account(
            self.admin,
            self.config,
            PermissionConfig::LEN,
            program_id,
            &config_signers,
        )?;

        // 4. Write config data
        {
            let mut data = self.config.try_borrow_mut()?;
            let config = PermissionConfig::from_bytes_mut(&mut data)?;
            config.discriminator = DISCRIMINATOR_PERMISSION_CONFIG;
            config.bump = config_bump;
            config.admin = Address::new_from_array(self.admin.address().to_bytes());
            config.pending_admin = ZERO_ADDRESS;
        }

        // 5. Create the admin's UserPermissions PDA account
        let admin_perm_bump_ref = [admin_perm_bump];
        let admin_perm_signer_seeds =
            user_perm_seeds(self.admin.address().as_ref(), &admin_perm_bump_ref);
        let admin_perm_signers = [Signer::from(&admin_perm_signer_seeds)];

        create_pda_account(
            self.admin,
            self.admin_perms,
            UserPermissions::LEN,
            program_id,
            &admin_perm_signers,
        )?;

        // 6. Write admin's permissions (empty roles — admin is identified by config.admin, not a role bit)
        {
            let mut data = self.admin_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut(&mut data)?;
            perms.discriminator = DISCRIMINATOR_USER_PERMISSION;
            perms.bump = admin_perm_bump;
            perms.roles = [0u8; 32];
        }

        emit_permission_manager_initialized(&self.admin.address().to_bytes());

        Ok(())
    }
}
