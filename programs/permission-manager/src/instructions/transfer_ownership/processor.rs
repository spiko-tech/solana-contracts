use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use spiko_common::AccountDeserialize;

use crate::events::build_ownership_transfer_started_event;
use crate::helpers::{require_admin, verify_pda};
use crate::state::{PermissionConfig, PERMISSION_CONFIG_SEED};

use super::accounts::TransferOwnershipAccounts;
use super::data::TransferOwnershipData;

/// Transfer ownership — sets a pending admin who must accept.
pub struct TransferOwnership<'a> {
    pub accounts: TransferOwnershipAccounts<'a>,
    pub data: TransferOwnershipData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for TransferOwnership<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: TransferOwnershipAccounts::try_from(accounts)?,
            data: TransferOwnershipData::try_from(data)?,
        })
    }
}

impl<'a> TransferOwnership<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        verify_pda(self.accounts.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        require_admin(self.accounts.admin, self.accounts.config, program_id)?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = PermissionConfig::from_bytes_mut(&mut data)?;
            config.pending_admin = Address::new_from_array(self.data.new_admin.to_bytes());
        }

        let event_data = build_ownership_transfer_started_event(
            &self.accounts.admin.address().to_bytes(),
            &self.data.new_admin.to_bytes(),
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
