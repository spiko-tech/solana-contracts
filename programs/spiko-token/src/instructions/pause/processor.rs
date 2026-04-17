use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};
use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_PAUSER;

use spiko_events::EventSerialize;

use crate::{
    error::TokenError, events::TokenPausedEvent, helpers::require_permission, state::TokenConfig,
};

use super::accounts::PauseAccounts;

/// Pause the token -- blocks all transfers, mints, and burns.
pub struct Pause<'a> {
    pub accounts: PauseAccounts<'a>,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Pause<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: PauseAccounts::try_from(accounts)?,
        })
    }
}

impl<'a> Pause<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.accounts.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        let permission_manager_id = {
            let data = self.accounts.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_permission(
            self.accounts.caller.address(),
            self.accounts.caller_perms,
            &permission_manager_id,
            ROLE_PAUSER,
            TokenError::Unauthorized.into(),
        )?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = TokenConfig::from_bytes_mut(&mut data)?;
            config.paused = 1;
        }

        let event = TokenPausedEvent::new(
            self.accounts.caller.address().clone(),
            self.accounts.config.address().clone(),
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
