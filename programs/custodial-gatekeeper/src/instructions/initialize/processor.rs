use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use crate::{
    error::GatekeeperError,
    events::{event_authority_pda, GatekeeperInitializedEvent},
    helpers::{create_pda_account, gatekeeper_config_seeds, verify_pda},
    state::{GatekeeperConfig, GATEKEEPER_CONFIG_SEED},
};

use spiko_events::EventSerialize;

use super::accounts::InitializeAccounts;
use super::data::InitializeData;

/// Initialize the CustodialGatekeeper singleton config.
pub struct Initialize<'a> {
    pub accounts: InitializeAccounts<'a>,
    pub data: InitializeData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Initialize<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: InitializeAccounts::try_from(accounts)?,
            data: InitializeData::try_from(data)?,
        })
    }
}

impl<'a> Initialize<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let bump = verify_pda(self.accounts.config, &[GATEKEEPER_CONFIG_SEED], program_id)?;

        {
            let data = self.accounts.config.try_borrow()?;
            if !data.is_empty() && data[0] != 0 {
                return Err(GatekeeperError::AlreadyInitialized.into());
            }
        }

        if self.data.max_delay <= 0 {
            return Err(GatekeeperError::InvalidMaxDelay.into());
        }

        let bump_bytes = [bump];
        let seeds = gatekeeper_config_seeds(&bump_bytes);
        let signer = Signer::from(&seeds);

        create_pda_account(
            self.accounts.admin,
            self.accounts.config,
            GatekeeperConfig::LEN,
            program_id,
            &[signer],
        )?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = GatekeeperConfig::from_bytes_mut_init(&mut data)?;
            config.bump = bump;
            config.set_max_delay(self.data.max_delay);
            config.permission_manager = Address::new_from_array(self.data.permission_manager);
        }

        let event = GatekeeperInitializedEvent::new(self.accounts.admin.address().clone());
        spiko_events::emit_event(
            program_id,
            self.accounts.event_authority,
            self.accounts.self_program,
            &event.to_bytes(),
            event_authority_pda::BUMP,
        )?;

        Ok(())
    }
}
