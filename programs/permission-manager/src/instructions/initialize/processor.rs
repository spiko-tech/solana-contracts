use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use crate::error::PermissionError;
use crate::events::build_permission_manager_initialized_event;
use crate::helpers::{config_seeds, create_pda_account, user_perm_seeds, verify_pda};
use crate::state::{
    PermissionConfig, UserPermissions, PERMISSION_CONFIG_SEED, USER_PERMISSION_SEED, ZERO_ADDRESS,
};

use super::accounts::InitializeAccounts;

/// Initialize the PermissionManager singleton config.
pub struct Initialize<'a> {
    pub accounts: InitializeAccounts<'a>,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Initialize<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: InitializeAccounts::try_from(accounts)?,
        })
    }
}

impl<'a> Initialize<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let config_bump = verify_pda(self.accounts.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        if self.accounts.config.owned_by(program_id) {
            return Err(PermissionError::AlreadyInitialized.into());
        }

        let admin_perm_bump = verify_pda(
            self.accounts.admin_perms,
            &[USER_PERMISSION_SEED, self.accounts.admin.address().as_ref()],
            program_id,
        )?;

        let config_bump_ref = [config_bump];
        let config_signer_seeds = config_seeds(&config_bump_ref);
        let config_signers = [Signer::from(&config_signer_seeds)];

        create_pda_account(
            self.accounts.admin,
            self.accounts.config,
            PermissionConfig::LEN,
            program_id,
            &config_signers,
        )?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = PermissionConfig::from_bytes_mut_init(&mut data)?;
            config.bump = config_bump;
            config.admin = Address::new_from_array(self.accounts.admin.address().to_bytes());
            config.pending_admin = ZERO_ADDRESS;
        }

        let admin_perm_bump_ref = [admin_perm_bump];
        let admin_perm_signer_seeds =
            user_perm_seeds(self.accounts.admin.address().as_ref(), &admin_perm_bump_ref);
        let admin_perm_signers = [Signer::from(&admin_perm_signer_seeds)];

        create_pda_account(
            self.accounts.admin,
            self.accounts.admin_perms,
            UserPermissions::LEN,
            program_id,
            &admin_perm_signers,
        )?;

        {
            let mut data = self.accounts.admin_perms.try_borrow_mut()?;
            let perms = UserPermissions::from_bytes_mut_init(&mut data)?;
            perms.bump = admin_perm_bump;
            perms.roles = [0u8; 32];
        }

        let event_data =
            build_permission_manager_initialized_event(&self.accounts.admin.address().to_bytes());
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
