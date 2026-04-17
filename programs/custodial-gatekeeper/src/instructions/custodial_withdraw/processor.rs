use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use permission_manager::state::{ROLE_WHITELISTED, ROLE_WHITELISTED_EXT};

use crate::{
    error::GatekeeperError,
    events::{event_authority_pda, WithdrawalBlockedEvent, WithdrawalInitiatedEvent},
    helpers::{
        compute_operation_id, cpi_token_2022_transfer, create_pda_account, require_permission,
        verify_pda, withdrawal_operation_seeds,
    },
    state::{
        GatekeeperConfig, WithdrawalDailyLimit, WithdrawalOperation, SECONDS_PER_DAY, STATUS_NULL,
        STATUS_PENDING, VAULT_SEED, WITHDRAWAL_DAILY_LIMIT_SEED, WITHDRAWAL_OPERATION_SEED,
    },
};

use spiko_events::EventSerialize;

use super::accounts::CustodialWithdrawAccounts;
use super::data::CustodialWithdrawData;

/// Handle a custodial withdrawal request.
///
/// 1. Verify sender has ROLE_WHITELISTED
/// 2. Verify recipient has ROLE_WHITELISTED_EXT
/// 3. Transfer tokens from sender to vault via Token-2022 TransferChecked
/// 4. If within daily limit: immediately transfer from vault to recipient
/// 5. If over daily limit: create a pending WithdrawalOperation PDA
pub struct CustodialWithdraw<'a> {
    pub accounts: CustodialWithdrawAccounts<'a>,
    pub data: CustodialWithdrawData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for CustodialWithdraw<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: CustodialWithdrawAccounts::try_from(accounts)?,
            data: CustodialWithdrawData::try_from(data)?,
        })
    }
}

impl<'a> CustodialWithdraw<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let (permission_manager_id, max_delay) = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(GatekeeperError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = GatekeeperConfig::from_bytes(&data)?;
            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.max_delay(),
            )
        };

        // Verify sender has ROLE_WHITELISTED
        require_permission(
            self.accounts.sender.address(),
            self.accounts.sender_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            GatekeeperError::UnauthorizedFrom.into(),
        )?;

        // Verify recipient has ROLE_WHITELISTED_EXT
        require_permission(
            &Address::new_from_array(self.data.recipient),
            self.accounts.recipient_perms,
            &permission_manager_id,
            ROLE_WHITELISTED_EXT,
            GatekeeperError::UnauthorizedTo.into(),
        )?;

        let mint_key_bytes = self.accounts.token_mint.address().to_bytes();

        // Verify daily limit PDA
        verify_pda(
            self.accounts.daily_limit,
            &[WITHDRAWAL_DAILY_LIMIT_SEED, &mint_key_bytes],
            program_id,
        )?;

        if !self.accounts.daily_limit.owned_by(program_id) {
            return Err(GatekeeperError::NotInitialized.into());
        }

        let vault_bump = verify_pda(self.accounts.vault_authority, &[VAULT_SEED], program_id)?;

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let today = now / SECONDS_PER_DAY;

        // Check daily limit and determine if instant or pending
        let within_limit = {
            let mut dl_data = self.accounts.daily_limit.try_borrow_mut()?;
            let dl = WithdrawalDailyLimit::from_bytes_mut(&mut dl_data)?;

            // Reset if new day
            if today > dl.last_day() {
                dl.set_used_amount(0);
                dl.set_last_day(today);
            }

            let new_used = dl
                .used_amount()
                .checked_add(self.data.amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;

            if new_used <= dl.limit() {
                dl.set_used_amount(new_used);
                true
            } else {
                false
            }
        };

        // Step 1: Transfer tokens from sender to vault via Token-2022 TransferChecked
        // (sender signs directly, no PDA signing needed)
        {
            let decimals = {
                let mint_data = self.accounts.token_mint.try_borrow()?;
                if mint_data.len() < 45 {
                    return Err(ProgramError::InvalidAccountData);
                }
                mint_data[44]
            };

            let mut ix_data = [0u8; 10];
            ix_data[0] = 12; // TransferChecked opcode
            ix_data[1..9].copy_from_slice(&self.data.amount.to_le_bytes());
            ix_data[9] = decimals;

            let ix_accounts = [
                InstructionAccount::writable(self.accounts.sender_token_account.address()),
                InstructionAccount::readonly(self.accounts.token_mint.address()),
                InstructionAccount::writable(self.accounts.vault_token_account.address()),
                InstructionAccount::readonly_signer(self.accounts.sender.address()),
                // Transfer Hook extra accounts:
                InstructionAccount::readonly(self.accounts.extra_account_meta_list.address()),
                InstructionAccount::readonly(self.accounts.permission_manager_program.address()),
                InstructionAccount::readonly(self.accounts.spiko_token_program.address()),
                InstructionAccount::readonly(self.accounts.token_config.address()),
                InstructionAccount::readonly(self.accounts.sender_perms.address()),
                InstructionAccount::readonly(self.accounts.vault_authority_perms.address()),
                InstructionAccount::readonly(self.accounts.hook_event_authority.address()),
                InstructionAccount::readonly(self.accounts.hook_program.address()),
            ];

            let instruction = InstructionView {
                program_id: self.accounts.token_2022_program.address(),
                accounts: &ix_accounts,
                data: &ix_data,
            };

            pinocchio::cpi::invoke(
                &instruction,
                &[
                    self.accounts.sender_token_account,
                    self.accounts.token_mint,
                    self.accounts.vault_token_account,
                    self.accounts.sender,
                    self.accounts.extra_account_meta_list,
                    self.accounts.permission_manager_program,
                    self.accounts.spiko_token_program,
                    self.accounts.token_config,
                    self.accounts.sender_perms,
                    self.accounts.vault_authority_perms,
                    self.accounts.hook_event_authority,
                    self.accounts.hook_program,
                    self.accounts.token_2022_program,
                ],
            )?;
        }

        // Emit WithdrawalInitiated on every call
        let event = WithdrawalInitiatedEvent::new(
            self.accounts.sender.address().clone(),
            Address::new_from_array(self.data.recipient),
            self.accounts.token_mint.address().clone(),
            self.data.amount,
            self.data.salt,
            0, // no deadline for instant withdrawals
        );
        spiko_events::emit_event(
            program_id,
            self.accounts.event_authority,
            self.accounts.self_program,
            &event.to_bytes(),
            event_authority_pda::BUMP,
        )?;

        if within_limit {
            // Step 2a: Instant transfer from vault to recipient
            cpi_token_2022_transfer(
                self.accounts.vault_token_account,
                self.accounts.token_mint,
                self.accounts.recipient_token_account,
                self.accounts.vault_authority,
                vault_bump,
                self.accounts.token_2022_program,
                self.accounts.extra_account_meta_list,
                self.accounts.permission_manager_program,
                self.accounts.spiko_token_program,
                self.accounts.token_config,
                self.accounts.vault_authority_perms,
                self.accounts.recipient_perms,
                self.accounts.hook_event_authority,
                self.accounts.hook_program,
                self.data.amount,
            )?;
        } else {
            // Step 2b: Create pending operation
            let operation_id = compute_operation_id(
                &self.data.recipient,
                &mint_key_bytes,
                self.data.amount,
                self.data.salt,
            );

            let op_bump = verify_pda(
                self.accounts.withdrawal_op,
                &[WITHDRAWAL_OPERATION_SEED, &operation_id],
                program_id,
            )?;

            // Check operation doesn't already exist
            {
                let op_data = self.accounts.withdrawal_op.try_borrow()?;
                if !op_data.is_empty() && op_data[0] != 0 {
                    if op_data.len() >= WithdrawalOperation::LEN {
                        let op = WithdrawalOperation::from_bytes(&op_data)?;
                        if op.status != STATUS_NULL {
                            return Err(GatekeeperError::OperationExists.into());
                        }
                    }
                }
            }

            let deadline = now + max_delay;

            let op_bump_bytes = [op_bump];
            let op_seeds = withdrawal_operation_seeds(&operation_id, &op_bump_bytes);
            let op_signer = Signer::from(&op_seeds);

            create_pda_account(
                self.accounts.sender,
                self.accounts.withdrawal_op,
                WithdrawalOperation::LEN,
                program_id,
                &[op_signer],
            )?;

            {
                let mut data = self.accounts.withdrawal_op.try_borrow_mut()?;
                let op = WithdrawalOperation::from_bytes_mut_init(&mut data)?;
                op.bump = op_bump;
                op.status = STATUS_PENDING;
                op.set_deadline(deadline);
                op.recipient = Address::new_from_array(self.data.recipient);
                op.mint = self.accounts.token_mint.address().clone();
                op.set_amount(self.data.amount);
            }

            let blocked_event = WithdrawalBlockedEvent::new(
                self.accounts.sender.address().clone(),
                Address::new_from_array(self.data.recipient),
                self.accounts.token_mint.address().clone(),
                self.data.amount,
                self.data.salt,
                deadline,
            );
            spiko_events::emit_event(
                program_id,
                self.accounts.event_authority,
                self.accounts.self_program,
                &blocked_event.to_bytes(),
                event_authority_pda::BUMP,
            )?;
        }

        Ok(())
    }
}
