use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use crate::{
    error::GatekeeperError,
    events::{event_authority_pda, DailyLimitUpdatedEvent},
    helpers::{create_pda_account, require_admin, verify_pda, withdrawal_daily_limit_seeds},
    state::{GatekeeperConfig, WithdrawalDailyLimit, WITHDRAWAL_DAILY_LIMIT_SEED},
};

use spiko_events::EventSerialize;

use super::accounts::SetDailyLimitAccounts;
use super::data::SetDailyLimitData;

/// Set the daily withdrawal limit for a specific token. Admin only.
pub struct SetDailyLimit<'a> {
    pub accounts: SetDailyLimitAccounts<'a>,
    pub data: SetDailyLimitData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for SetDailyLimit<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: SetDailyLimitAccounts::try_from(accounts)?,
            data: SetDailyLimitData::try_from(data)?,
        })
    }
}

impl<'a> SetDailyLimit<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let permission_manager_id = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(GatekeeperError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = GatekeeperConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_admin(
            self.accounts.caller,
            self.accounts.perm_config,
            &permission_manager_id,
            GatekeeperError::Unauthorized.into(),
        )?;

        let bump = verify_pda(
            self.accounts.daily_limit,
            &[WITHDRAWAL_DAILY_LIMIT_SEED, &self.data.token_mint],
            program_id,
        )?;

        let needs_creation = self.accounts.daily_limit.lamports() == 0;

        if needs_creation {
            let bump_bytes = [bump];
            let seeds = withdrawal_daily_limit_seeds(&self.data.token_mint, &bump_bytes);
            let signer = Signer::from(&seeds);

            create_pda_account(
                self.accounts.caller,
                self.accounts.daily_limit,
                WithdrawalDailyLimit::LEN,
                program_id,
                &[signer],
            )?;
        }

        {
            let mut data = self.accounts.daily_limit.try_borrow_mut()?;
            let dl = if needs_creation {
                WithdrawalDailyLimit::from_bytes_mut_init(&mut data)?
            } else {
                WithdrawalDailyLimit::from_bytes_mut(&mut data)?
            };
            dl.bump = bump;
            dl.set_limit(self.data.limit);
        }

        let event = DailyLimitUpdatedEvent::new(
            self.accounts.caller.address().clone(),
            Address::new_from_array(self.data.token_mint),
            self.data.limit,
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
