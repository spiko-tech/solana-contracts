use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use crate::{
    error::RedemptionError,
    events::{event_authority_pda, TokenMinimumUpdatedEvent},
    helpers::{create_pda_account, require_admin, token_minimum_seeds, verify_pda},
    state::{RedemptionConfig, TokenMinimum, TOKEN_MINIMUM_SEED},
};

use spiko_events::EventSerialize;

use super::accounts::SetMinimumAccounts;
use super::data::SetMinimumData;

/// Set the minimum redemption amount for a specific token. Admin only.
pub struct SetMinimum<'a> {
    pub accounts: SetMinimumAccounts<'a>,
    pub data: SetMinimumData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for SetMinimum<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: SetMinimumAccounts::try_from(accounts)?,
            data: SetMinimumData::try_from(data)?,
        })
    }
}

impl<'a> SetMinimum<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let permission_manager_id = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = RedemptionConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_admin(
            self.accounts.caller,
            self.accounts.perm_config,
            &permission_manager_id,
            RedemptionError::Unauthorized.into(),
        )?;

        let bump = verify_pda(
            self.accounts.token_minimum,
            &[TOKEN_MINIMUM_SEED, &self.data.token_mint],
            program_id,
        )?;

        let needs_creation = self.accounts.token_minimum.lamports() == 0;

        if needs_creation {
            let bump_bytes = [bump];
            let seeds = token_minimum_seeds(&self.data.token_mint, &bump_bytes);
            let signer = Signer::from(&seeds);

            create_pda_account(
                self.accounts.caller,
                self.accounts.token_minimum,
                TokenMinimum::LEN,
                program_id,
                &[signer],
            )?;
        }

        {
            let mut data = self.accounts.token_minimum.try_borrow_mut()?;
            let tm = if needs_creation {
                TokenMinimum::from_bytes_mut_init(&mut data)?
            } else {
                TokenMinimum::from_bytes_mut(&mut data)?
            };
            tm.bump = bump;
            tm.set_minimum_amount(self.data.minimum);
        }

        let event = TokenMinimumUpdatedEvent::new(
            self.accounts.caller.address().clone(),
            Address::new_from_array(self.data.token_mint),
            self.data.minimum,
        );
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
