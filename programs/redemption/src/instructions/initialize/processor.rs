use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use crate::{
    error::RedemptionError,
    events::build_redemption_initialized_event,
    helpers::{create_pda_account, redemption_config_seeds, verify_pda},
    state::{RedemptionConfig, REDEMPTION_CONFIG_SEED},
};

use super::accounts::InitializeRedemptionAccounts;
use super::data::InitializeRedemptionData;

/// Initialize the Redemption singleton config.
pub struct InitializeRedemption<'a> {
    pub accounts: InitializeRedemptionAccounts<'a>,
    pub data: InitializeRedemptionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitializeRedemption<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: InitializeRedemptionAccounts::try_from(accounts)?,
            data: InitializeRedemptionData::try_from(data)?,
        })
    }
}

impl<'a> InitializeRedemption<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let bump = verify_pda(self.accounts.config, &[REDEMPTION_CONFIG_SEED], program_id)?;

        {
            let data = self.accounts.config.try_borrow()?;
            if !data.is_empty() && data[0] != 0 {
                return Err(RedemptionError::AlreadyInitialized.into());
            }
        }

        let bump_bytes = [bump];
        let seeds = redemption_config_seeds(&bump_bytes);
        let signer = Signer::from(&seeds);

        create_pda_account(
            self.accounts.admin,
            self.accounts.config,
            RedemptionConfig::LEN,
            program_id,
            &[signer],
        )?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = RedemptionConfig::from_bytes_mut_init(&mut data)?;
            config.bump = bump;
            config.permission_manager = Address::new_from_array(self.data.permission_manager);
        }

        let event_data =
            build_redemption_initialized_event(&self.accounts.admin.address().to_bytes());
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
