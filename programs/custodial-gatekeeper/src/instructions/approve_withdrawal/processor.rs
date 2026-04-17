use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_CUSTODIAL_GATEKEEPER_APPROVER;

use crate::{
    error::GatekeeperError,
    events::{event_authority_pda, WithdrawalApprovedEvent},
    helpers::{compute_operation_id, cpi_token_2022_transfer, require_permission, verify_pda},
    state::{
        GatekeeperConfig, WithdrawalOperation, STATUS_DONE, STATUS_PENDING, VAULT_SEED,
        WITHDRAWAL_OPERATION_SEED,
    },
};

use spiko_events::EventSerialize;

use super::accounts::ApproveWithdrawalAccounts;
use super::data::ApproveWithdrawalData;

/// Approve a pending withdrawal. Transfers tokens from vault to recipient.
pub struct ApproveWithdrawal<'a> {
    pub accounts: ApproveWithdrawalAccounts<'a>,
    pub data: ApproveWithdrawalData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for ApproveWithdrawal<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: ApproveWithdrawalAccounts::try_from(accounts)?,
            data: ApproveWithdrawalData::try_from(data)?,
        })
    }
}

impl<'a> ApproveWithdrawal<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let permission_manager_id = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(GatekeeperError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = GatekeeperConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        // Verify approver has ROLE_CUSTODIAL_GATEKEEPER_APPROVER
        require_permission(
            self.accounts.approver.address(),
            self.accounts.approver_perms,
            &permission_manager_id,
            ROLE_CUSTODIAL_GATEKEEPER_APPROVER,
            GatekeeperError::Unauthorized.into(),
        )?;

        let mint_key_bytes = self.accounts.token_mint.address().to_bytes();
        let operation_id = compute_operation_id(
            &self.data.recipient,
            &mint_key_bytes,
            self.data.amount,
            self.data.salt,
        );

        verify_pda(
            self.accounts.withdrawal_op,
            &[WITHDRAWAL_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        let vault_bump = verify_pda(self.accounts.vault_authority, &[VAULT_SEED], program_id)?;

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        {
            if !self.accounts.withdrawal_op.owned_by(program_id) {
                return Err(GatekeeperError::NotInitialized.into());
            }
            let data = self.accounts.withdrawal_op.try_borrow()?;
            let op = WithdrawalOperation::from_bytes(&data)?;

            if op.status != STATUS_PENDING {
                return Err(GatekeeperError::NotPending.into());
            }

            if now > op.deadline() {
                return Err(GatekeeperError::DeadlinePassed.into());
            }
        }

        // Transfer tokens from vault to recipient
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

        // Mark operation as done
        {
            let mut data = self.accounts.withdrawal_op.try_borrow_mut()?;
            let op = WithdrawalOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_DONE;
        }

        let event = WithdrawalApprovedEvent::new(
            self.accounts.approver.address().clone(),
            Address::new_from_array(self.data.recipient),
            self.accounts.token_mint.address().clone(),
            self.data.amount,
            self.data.salt,
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
