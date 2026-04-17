use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_WHITELISTED;
use spiko_token::state::TokenConfig;

use spiko_events::EventSerialize;

use crate::{
    error::TransferHookError,
    events::TransferEvent,
    helpers::{require_not_paused, require_permission},
};

use super::accounts::TransferHookExecuteAccounts;
use super::data::TransferHookExecuteData;

/// Transfer Hook Execute handler.
///
/// Token-2022 calls this instruction automatically whenever a
/// `TransferChecked` is executed on a mint that has the Transfer Hook
/// extension pointing to this program.
///
/// The handler enforces:
///   - Token is not paused (via TokenConfig, owned by spiko-token)
///   - Source wallet is WHITELISTED (via UserPermissions PDA)
///   - Destination wallet is WHITELISTED (via UserPermissions PDA)
pub struct TransferHookExecute<'a> {
    pub accounts: TransferHookExecuteAccounts<'a>,
    pub data: TransferHookExecuteData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for TransferHookExecute<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: TransferHookExecuteAccounts::try_from(accounts)?,
            data: TransferHookExecuteData::try_from(data)?,
        })
    }
}

impl<'a> TransferHookExecute<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.accounts.extra_account_meta_list.owned_by(program_id) {
            return Err(ProgramError::IllegalOwner);
        }

        if !self
            .accounts
            .token_config
            .owned_by(self.accounts.spiko_token_program.address())
        {
            return Err(TransferHookError::NotInitialized.into());
        }

        {
            let config_data = self.accounts.token_config.try_borrow()?;
            require_not_paused(&config_data)?;
        }

        let permission_manager_id = {
            let config_data = self.accounts.token_config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        if self.accounts.permission_manager_program.address() != &permission_manager_id {
            return Err(TransferHookError::Unauthorized.into());
        }

        // Extract sender owner from source token account (bytes 32..64)
        let sender_owner = {
            let src_data = self.accounts.source.try_borrow()?;
            if src_data.len() < 64 {
                return Err(TransferHookError::UnauthorizedFrom.into());
            }
            let owner_bytes: &[u8; 32] = src_data[32..64].try_into().unwrap();
            Address::new_from_array(*owner_bytes)
        };

        require_permission(
            &sender_owner,
            self.accounts.sender_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TransferHookError::UnauthorizedFrom.into(),
        )?;

        // Extract recipient owner from destination token account (bytes 32..64)
        let recipient_owner = {
            let dest_data = self.accounts.destination.try_borrow()?;
            if dest_data.len() < 64 {
                return Err(TransferHookError::UnauthorizedTo.into());
            }
            let owner_bytes: &[u8; 32] = dest_data[32..64].try_into().unwrap();
            Address::new_from_array(*owner_bytes)
        };

        require_permission(
            &recipient_owner,
            self.accounts.recipient_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TransferHookError::UnauthorizedTo.into(),
        )?;

        let event = TransferEvent::new(
            self.accounts.authority.address().clone(),
            self.accounts.mint.address().clone(),
            self.accounts.source.address().clone(),
            self.accounts.destination.address().clone(),
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
