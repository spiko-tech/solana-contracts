use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};
use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_BURNER;

use crate::{
    error::TokenError,
    events::build_burn_event,
    helpers::{mint_authority_seeds, require_not_paused, require_permission},
    state::{TokenConfig, MINT_AUTHORITY_SEED},
};

use super::accounts::BurnTokenAccounts;
use super::data::BurnTokenData;

/// Burn tokens from the caller's own token account (self-burn).
pub struct BurnToken<'a> {
    pub accounts: BurnTokenAccounts<'a>,
    pub data: BurnTokenData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for BurnToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: BurnTokenAccounts::try_from(accounts)?,
            data: BurnTokenData::try_from(data)?,
        })
    }
}

impl<'a> BurnToken<'a> {
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

        require_permission(
            self.accounts.caller.address(),
            self.accounts.caller_perms,
            &permission_manager_id,
            ROLE_BURNER,
            TokenError::Unauthorized.into(),
        )?;

        {
            let src_data = self.accounts.source_token_account.try_borrow()?;
            if src_data.len() < 64 {
                return Err(TokenError::Unauthorized.into());
            }
            let owner_bytes: &[u8; 32] = src_data[32..64].try_into().unwrap();
            let token_account_owner = Address::new_from_array(*owner_bytes);
            if self.accounts.caller.address() != &token_account_owner {
                return Err(TokenError::Unauthorized.into());
            }
        }

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

        let event_data = build_burn_event(
            &self.accounts.caller.address().to_bytes(),
            &self.accounts.mint.address().to_bytes(),
            &self.accounts.source_token_account.address().to_bytes(),
            self.data.amount,
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
