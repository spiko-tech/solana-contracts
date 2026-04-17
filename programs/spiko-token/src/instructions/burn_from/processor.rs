use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};
use spiko_common::AccountDeserialize;

use spiko_events::EventSerialize;

use crate::{
    error::TokenError,
    events::BurnEvent,
    helpers::{mint_authority_seeds, require_admin, require_not_paused},
    state::{TokenConfig, MINT_AUTHORITY_SEED},
};

use super::accounts::BurnFromAccounts;
use super::data::BurnFromData;

/// Burn tokens from any token account (admin-only).
pub struct BurnFrom<'a> {
    pub accounts: BurnFromAccounts<'a>,
    pub data: BurnFromData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for BurnFrom<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: BurnFromAccounts::try_from(accounts)?,
            data: BurnFromData::try_from(data)?,
        })
    }
}

impl<'a> BurnFrom<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.accounts.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        {
            let config_data = self.accounts.config.try_borrow()?;
            require_not_paused(&config_data)?;
        }

        let (permission_manager_id, mint_auth_bump) = {
            let config_data = self.accounts.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;
            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.mint_authority_bump,
            )
        };

        require_admin(
            self.accounts.caller,
            self.accounts.perm_config,
            &permission_manager_id,
            TokenError::Unauthorized.into(),
        )?;

        let mint_key = self.accounts.mint.address();
        let _ma_bump = crate::helpers::verify_pda(
            self.accounts.mint_authority,
            &[MINT_AUTHORITY_SEED, mint_key.as_ref()],
            program_id,
        )?;

        let bump_bytes = [mint_auth_bump];
        let ma_seeds = mint_authority_seeds(mint_key.as_ref(), &bump_bytes);
        let ma_signer = Signer::from(&ma_seeds);

        pinocchio_token_2022::instructions::Burn {
            account: self.accounts.source_token_account,
            mint: self.accounts.mint,
            authority: self.accounts.mint_authority,
            amount: self.data.amount,
            token_program: self.accounts.token_2022_program.address(),
        }
        .invoke_signed(&[ma_signer])?;

        let event = BurnEvent::new(
            self.accounts.caller.address().clone(),
            self.accounts.mint.address().clone(),
            self.accounts.source_token_account.address().clone(),
            self.data.amount,
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
