use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};
use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_MINTER;

use crate::{
    error::TokenError,
    events::build_mint_event,
    helpers::{mint_authority_seeds, require_not_paused, require_permission},
    state::{TokenConfig, MINT_AUTHORITY_SEED},
};

use super::accounts::MintTokenAccounts;
use super::data::MintTokenData;

/// Mint tokens to a whitelisted recipient.
pub struct MintToken<'a> {
    pub accounts: MintTokenAccounts<'a>,
    pub data: MintTokenData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for MintToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: MintTokenAccounts::try_from(accounts)?,
            data: MintTokenData::try_from(data)?,
        })
    }
}

impl<'a> MintToken<'a> {
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
            ROLE_MINTER,
            TokenError::Unauthorized.into(),
        )?;

        // Extract the recipient's owner from the token account data (bytes 32..64)
        // and verify the recipient_perms PDA is derived from that owner.
        let recipient_owner = {
            let ata_data = self.accounts.recipient_token_account.try_borrow()?;
            if ata_data.len() < 64 {
                return Err(TokenError::UnauthorizedTo.into());
            }
            let owner_bytes: &[u8; 32] = ata_data[32..64].try_into().unwrap();
            Address::new_from_array(*owner_bytes)
        };

        require_permission(
            &recipient_owner,
            self.accounts.recipient_perms,
            &permission_manager_id,
            permission_manager::state::ROLE_WHITELISTED,
            TokenError::UnauthorizedTo.into(),
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

        pinocchio_token_2022::instructions::MintTo {
            mint: self.accounts.mint,
            account: self.accounts.recipient_token_account,
            mint_authority: self.accounts.mint_authority,
            amount: self.data.amount,
            token_program: self.accounts.token_2022_program.address(),
        }
        .invoke_signed(&[ma_signer])?;

        let event_data = build_mint_event(
            &self.accounts.caller.address().to_bytes(),
            &self.accounts.mint.address().to_bytes(),
            &self.accounts.recipient_token_account.address().to_bytes(),
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
