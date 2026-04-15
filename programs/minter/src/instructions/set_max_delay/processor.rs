use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};
use spiko_common::AccountDeserialize;

use crate::{
    error::MinterError, events::build_max_delay_updated_event, helpers::require_admin,
    state::MinterConfig,
};

use super::accounts::SetMaxDelayAccounts;
use super::data::SetMaxDelayData;

/// Set the maximum pending duration for blocked mints. Admin only.
pub struct SetMaxDelay<'a> {
    pub accounts: SetMaxDelayAccounts<'a>,
    pub data: SetMaxDelayData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for SetMaxDelay<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: SetMaxDelayAccounts::try_from(accounts)?,
            data: SetMaxDelayData::try_from(data)?,
        })
    }
}

impl<'a> SetMaxDelay<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let permission_manager_id = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_admin(
            self.accounts.caller,
            self.accounts.perm_config,
            &permission_manager_id,
            MinterError::Unauthorized.into(),
        )?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = MinterConfig::from_bytes_mut(&mut data)?;
            config.set_max_delay(self.data.max_delay);
        }

        let event_data = build_max_delay_updated_event(
            &self.accounts.caller.address().to_bytes(),
            self.data.max_delay,
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
