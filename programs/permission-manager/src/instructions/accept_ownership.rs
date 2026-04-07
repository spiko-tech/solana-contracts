use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use crate::error::PermissionError;
use crate::events::emit_ownership_transferred;
use crate::helpers::verify_pda;
use crate::state::{PermissionConfig, PERMISSION_CONFIG_SEED, ZERO_ADDRESS};

/// Accept ownership — the pending admin finalizes the transfer.
///
/// Accounts:
///   0. `[signer]`   New admin (must be the pending_admin)
///   1. `[writable]` PermissionConfig PDA
pub struct AcceptOwnership<'a> {
    pub new_admin: &'a AccountView,
    pub config: &'a AccountView,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for AcceptOwnership<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [new_admin, config, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !new_admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self { new_admin, config })
    }
}

impl<'a> AcceptOwnership<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        verify_pda(self.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        if !self.config.owned_by(program_id) {
            return Err(PermissionError::NotInitialized.into());
        }

        {
            let data = self.config.try_borrow()?;
            let config = PermissionConfig::from_bytes(&data)?;

            if !config.has_pending_admin() {
                return Err(PermissionError::NoPendingAdmin.into());
            }

            if self.new_admin.address() != &config.pending_admin {
                return Err(PermissionError::NotPendingAdmin.into());
            }
        }

        {
            let mut data = self.config.try_borrow_mut()?;
            let config = PermissionConfig::from_bytes_mut(&mut data)?;
            config.admin = Address::new_from_array(self.new_admin.address().to_bytes());
            config.pending_admin = ZERO_ADDRESS;
        }

        pinocchio_log::log!("OwnershipTransferred");
        emit_ownership_transferred(&self.new_admin.address().to_bytes());

        Ok(())
    }
}
