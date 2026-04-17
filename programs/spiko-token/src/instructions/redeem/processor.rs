use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};
use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_WHITELISTED;

use spiko_events::EventSerialize;

use crate::{
    error::TokenError,
    events::RedeemInitiatedEvent,
    helpers::{read_mint_decimals, require_not_paused, require_permission, token_config_seeds},
    state::TokenConfig,
};

use super::accounts::RedeemTokenAccounts;
use super::data::RedeemTokenData;

/// Redeem tokens: transfer from user to vault, then CPI to Redemption::on_redeem.
pub struct RedeemToken<'a> {
    pub accounts: RedeemTokenAccounts<'a>,
    pub data: RedeemTokenData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for RedeemToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: RedeemTokenAccounts::try_from(accounts)?,
            data: RedeemTokenData::try_from(data)?,
        })
    }
}

impl<'a> RedeemToken<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.accounts.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        {
            let config_data = self.accounts.config.try_borrow()?;
            require_not_paused(&config_data)?;
        }

        let (permission_manager_id, config_bump, redemption_contract) = {
            let config_data = self.accounts.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;

            if !config.has_redemption_contract() {
                return Err(TokenError::RedemptionContractNotSet.into());
            }

            if self.accounts.redemption_program.address() != &config.redemption_contract {
                return Err(TokenError::RedemptionContractMismatch.into());
            }

            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.bump,
                Address::new_from_array(config.redemption_contract.to_bytes()),
            )
        };

        let _ = redemption_contract;

        require_permission(
            self.accounts.user.address(),
            self.accounts.user_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TokenError::UnauthorizedFrom.into(),
        )?;

        // Extract the vault's owner from the vault token account data
        // (bytes 32..64) and verify the vault_authority_perms PDA matches.
        let vault_owner = {
            let vault_data = self.accounts.vault.try_borrow()?;
            if vault_data.len() < 64 {
                return Err(TokenError::UnauthorizedTo.into());
            }
            let owner_bytes: &[u8; 32] = vault_data[32..64].try_into().unwrap();
            Address::new_from_array(*owner_bytes)
        };

        require_permission(
            &vault_owner,
            self.accounts.vault_authority_perms,
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
                InstructionAccount::writable(self.accounts.user_source.address()), // source
                InstructionAccount::readonly(self.accounts.mint.address()),        // mint
                InstructionAccount::writable(self.accounts.vault.address()),       // destination
                InstructionAccount::readonly_signer(self.accounts.user.address()), // authority (user signs)
                // Transfer Hook extra accounts (must match ExtraAccountMetaList order):
                InstructionAccount::readonly(self.accounts.extra_account_meta_list.address()), // [meta] ExtraAccountMetaList
                InstructionAccount::readonly(self.accounts.permission_manager_program.address()), // [0] PermissionManager program
                InstructionAccount::readonly(self.accounts.spiko_token_program.address()), // [1] SpikoToken program
                InstructionAccount::readonly(self.accounts.config.address()), // [2] TokenConfig PDA
                InstructionAccount::readonly(self.accounts.user_perms.address()), // [3] Sender UserPerms
                InstructionAccount::readonly(self.accounts.vault_authority_perms.address()), // [4] Recipient UserPerms
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
                    self.accounts.user_source,
                    self.accounts.mint,
                    self.accounts.vault,
                    self.accounts.user,
                    self.accounts.extra_account_meta_list,
                    self.accounts.permission_manager_program,
                    self.accounts.spiko_token_program,
                    self.accounts.config,
                    self.accounts.user_perms,
                    self.accounts.vault_authority_perms,
                    self.accounts.hook_event_authority,
                    self.accounts.hook_program,
                    self.accounts.token_2022_program,
                ],
            )?;
        }

        let user_key_bytes = self.accounts.user.address().to_bytes();
        let mut ix_data = [0u8; 49];
        ix_data[0] = 4; // discriminator for on_redeem
        ix_data[1..33].copy_from_slice(&user_key_bytes);
        ix_data[33..41].copy_from_slice(&self.data.amount.to_le_bytes());
        ix_data[41..49].copy_from_slice(&self.data.salt.to_le_bytes());

        let ix_accounts = [
            InstructionAccount::readonly_signer(self.accounts.config.address()), // token_config (PDA signer)
            InstructionAccount::writable_signer(self.accounts.user.address()),   // user (payer)
            InstructionAccount::readonly(self.accounts.redemption_config.address()), // redemption_config
            InstructionAccount::writable(self.accounts.redemption_op.address()), // redemption_op
            InstructionAccount::readonly(self.accounts.token_minimum.address()), // token_minimum
            InstructionAccount::readonly(self.accounts.mint.address()),          // token_mint
            InstructionAccount::readonly(self.accounts.system_program.address()), // system_program
            InstructionAccount::readonly(self.accounts.rd_event_authority.address()), // redemption event authority PDA
            InstructionAccount::readonly(self.accounts.rd_self_program.address()), // redemption self program
        ];

        let instruction = InstructionView {
            program_id: self.accounts.redemption_program.address(),
            accounts: &ix_accounts,
            data: &ix_data,
        };

        let mint_key = self.accounts.mint.address();
        let bump_bytes = [config_bump];
        let tc_seeds = token_config_seeds(mint_key.as_ref(), &bump_bytes);
        let tc_signer = Signer::from(&tc_seeds);

        pinocchio::cpi::invoke_signed(
            &instruction,
            &[
                self.accounts.config,
                self.accounts.user,
                self.accounts.redemption_config,
                self.accounts.redemption_op,
                self.accounts.token_minimum,
                self.accounts.mint,
                self.accounts.system_program,
                self.accounts.rd_event_authority,
                self.accounts.rd_self_program,
                self.accounts.redemption_program,
            ],
            &[tc_signer],
        )?;

        let event = RedeemInitiatedEvent::new(
            self.accounts.user.address().clone(),
            self.accounts.mint.address().clone(),
            self.data.amount,
            self.data.salt,
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
