use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};
use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_WHITELISTED;

use crate::{
    error::TokenError,
    helpers::{read_mint_decimals, require_not_paused, require_permission},
    state::TokenConfig,
};

use super::accounts::TransferTokenAccounts;
use super::data::TransferTokenData;

/// Transfer tokens between whitelisted accounts.
pub struct TransferToken<'a> {
    pub accounts: TransferTokenAccounts<'a>,
    pub data: TransferTokenData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for TransferToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: TransferTokenAccounts::try_from(accounts)?,
            data: TransferTokenData::try_from(data)?,
        })
    }
}

impl<'a> TransferToken<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.accounts.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        {
            let config_data = self.accounts.config.try_borrow()?;
            require_not_paused(&config_data)?;
        }

        let permission_manager_id = {
            let config_data = self.accounts.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_permission(
            self.accounts.sender.address(),
            self.accounts.sender_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TokenError::UnauthorizedFrom.into(),
        )?;

        // Extract the recipient's owner from the destination token account
        // data (bytes 32..64) and verify the recipient_perms PDA matches.
        let recipient_owner = {
            let dest_data = self.accounts.destination.try_borrow()?;
            if dest_data.len() < 64 {
                return Err(TokenError::UnauthorizedTo.into());
            }
            let owner_bytes: &[u8; 32] = dest_data[32..64].try_into().unwrap();
            Address::new_from_array(*owner_bytes)
        };

        require_permission(
            &recipient_owner,
            self.accounts.recipient_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TokenError::UnauthorizedTo.into(),
        )?;

        {
            let decimals = read_mint_decimals(self.accounts.mint)?;
            let mut ix_data = [0u8; 10];
            ix_data[0] = 12; // TransferChecked opcode
            ix_data[1..9].copy_from_slice(&self.data.amount.to_le_bytes());
            ix_data[9] = decimals;

            let ix_accounts = [
                // Standard TransferChecked accounts:
                InstructionAccount::writable(self.accounts.source.address()), // source
                InstructionAccount::readonly(self.accounts.mint.address()),   // mint
                InstructionAccount::writable(self.accounts.destination.address()), // destination
                InstructionAccount::readonly_signer(self.accounts.sender.address()), // authority
                // Transfer Hook extra accounts (must match ExtraAccountMetaList order):
                InstructionAccount::readonly(self.accounts.extra_account_meta_list.address()), // [meta] ExtraAccountMetaList
                InstructionAccount::readonly(self.accounts.permission_manager_program.address()), // [0] PermissionManager program
                InstructionAccount::readonly(self.accounts.spiko_token_program.address()), // [1] SpikoToken program
                InstructionAccount::readonly(self.accounts.config.address()), // [2] TokenConfig
                InstructionAccount::readonly(self.accounts.sender_perms.address()), // [3] Sender UserPerms
                InstructionAccount::readonly(self.accounts.recipient_perms.address()), // [4] Recipient UserPerms
                InstructionAccount::readonly(self.accounts.hook_event_authority.address()), // [5] Hook event authority
                InstructionAccount::readonly(self.accounts.hook_program.address()), // [6] Hook program
            ];

            let instruction = InstructionView {
                program_id: self.accounts.token_2022_program.address(),
                accounts: &ix_accounts,
                data: &ix_data,
            };

            pinocchio::cpi::invoke(
                &instruction,
                &[
                    self.accounts.source,
                    self.accounts.mint,
                    self.accounts.destination,
                    self.accounts.sender,
                    self.accounts.extra_account_meta_list,
                    self.accounts.permission_manager_program,
                    self.accounts.spiko_token_program,
                    self.accounts.config,
                    self.accounts.sender_perms,
                    self.accounts.recipient_perms,
                    self.accounts.hook_event_authority,
                    self.accounts.hook_program,
                    self.accounts.token_2022_program,
                ],
            )?;
        }

        Ok(())
    }
}
