use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};
use spiko_common::AccountDeserialize;

use spiko_events::EventSerialize;

use crate::{
    error::TokenError, events::RedemptionContractSetEvent, helpers::require_admin,
    state::TokenConfig,
};

use super::accounts::SetRedemptionContractAccounts;
use super::data::SetRedemptionContractData;

/// Set (or update) the redemption contract address stored in TokenConfig.
pub struct SetRedemptionContract<'a> {
    pub accounts: SetRedemptionContractAccounts<'a>,
    pub data: SetRedemptionContractData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for SetRedemptionContract<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: SetRedemptionContractAccounts::try_from(accounts)?,
            data: SetRedemptionContractData::try_from(data)?,
        })
    }
}

impl<'a> SetRedemptionContract<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.accounts.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        let permission_manager_id = {
            let data = self.accounts.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_admin(
            self.accounts.caller,
            self.accounts.perm_config,
            &permission_manager_id,
            TokenError::Unauthorized.into(),
        )?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = TokenConfig::from_bytes_mut(&mut data)?;
            config.redemption_contract = Address::new_from_array(self.data.redemption_contract);
        }

        let event = RedemptionContractSetEvent::new(
            self.accounts.caller.address().clone(),
            self.accounts.config.address().clone(),
            Address::new_from_array(self.data.redemption_contract),
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
