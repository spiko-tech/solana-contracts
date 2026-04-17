use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};
use spiko_common::{AccountDeserialize, AccountSize};

use spiko_events::EventSerialize;

use crate::{
    error::MinterError,
    events::MinterInitializedEvent,
    helpers::{create_pda_account, minter_config_seeds, verify_pda},
    state::{MinterConfig, MINTER_CONFIG_SEED},
};

use super::accounts::InitializeMinterAccounts;
use super::data::InitializeMinterData;

/// Initialize the Minter singleton config.
pub struct InitializeMinter<'a> {
    pub accounts: InitializeMinterAccounts<'a>,
    pub data: InitializeMinterData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitializeMinter<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: InitializeMinterAccounts::try_from(accounts)?,
            data: InitializeMinterData::try_from(data)?,
        })
    }
}

impl<'a> InitializeMinter<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let bump = verify_pda(self.accounts.config, &[MINTER_CONFIG_SEED], program_id)?;

        {
            let data = self.accounts.config.try_borrow()?;
            if !data.is_empty() && data[0] != 0 {
                return Err(MinterError::AlreadyInitialized.into());
            }
        }

        let bump_bytes = [bump];
        let seeds = minter_config_seeds(&bump_bytes);
        let signer = Signer::from(&seeds);

        create_pda_account(
            self.accounts.admin,
            self.accounts.config,
            MinterConfig::LEN,
            program_id,
            &[signer],
        )?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = MinterConfig::from_bytes_mut_init(&mut data)?;
            config.bump = bump;
            config.set_max_delay(self.data.max_delay);
            config.permission_manager = Address::new_from_array(self.data.permission_manager);
        }

        let event =
            MinterInitializedEvent::new(self.accounts.admin.address().clone(), self.data.max_delay);
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
