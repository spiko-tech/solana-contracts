use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use spiko_common::AccountDeserialize;

use crate::error::PermissionError;
use crate::events::OwnershipTransferredEvent;
use crate::helpers::verify_pda;
use crate::state::{PermissionConfig, PERMISSION_CONFIG_SEED, ZERO_ADDRESS};
use spiko_events::EventSerialize;

use super::accounts::AcceptOwnershipAccounts;

/// Accept ownership — the pending admin finalizes the transfer.
pub struct AcceptOwnership<'a> {
    pub accounts: AcceptOwnershipAccounts<'a>,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for AcceptOwnership<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: AcceptOwnershipAccounts::try_from(accounts)?,
        })
    }
}

impl<'a> AcceptOwnership<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        verify_pda(self.accounts.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        if !self.accounts.config.owned_by(program_id) {
            return Err(PermissionError::NotInitialized.into());
        }

        {
            let data = self.accounts.config.try_borrow()?;
            let config = PermissionConfig::from_bytes(&data)?;

            if !config.has_pending_admin() {
                return Err(PermissionError::NoPendingAdmin.into());
            }

            if self.accounts.new_admin.address() != &config.pending_admin {
                return Err(PermissionError::NotPendingAdmin.into());
            }
        }

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = PermissionConfig::from_bytes_mut(&mut data)?;
            config.admin = Address::new_from_array(self.accounts.new_admin.address().to_bytes());
            config.pending_admin = ZERO_ADDRESS;
        }

        let event = OwnershipTransferredEvent::new(self.accounts.new_admin.address().clone());
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
