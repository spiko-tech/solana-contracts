use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use spiko_common::AccountDeserialize;

use crate::{
    error::RedemptionError,
    events::build_redemption_canceled_event,
    helpers::{compute_operation_id, cpi_token_2022_transfer, verify_pda},
    state::{
        RedemptionConfig, RedemptionOperation, REDEMPTION_OPERATION_SEED, STATUS_CANCELED,
        STATUS_PENDING, VAULT_SEED,
    },
};

use super::accounts::CancelRedemptionAccounts;
use super::data::CancelRedemptionData;

/// Cancel a redemption after the deadline has passed. Anyone can call.
/// Tokens are refunded from the vault to the user.
pub struct CancelRedemption<'a> {
    pub accounts: CancelRedemptionAccounts<'a>,
    pub data: CancelRedemptionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for CancelRedemption<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: CancelRedemptionAccounts::try_from(accounts)?,
            data: CancelRedemptionData::try_from(data)?,
        })
    }
}

impl<'a> CancelRedemption<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        {
            if !self.accounts.config.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let _config = RedemptionConfig::from_bytes(&data)?;
        }

        let mint_key_bytes = self.accounts.token_mint.address().to_bytes();
        let operation_id = compute_operation_id(
            &self.data.user,
            &mint_key_bytes,
            self.data.amount,
            self.data.salt,
        );

        verify_pda(
            self.accounts.redemption_op,
            &[REDEMPTION_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        let vault_bump = verify_pda(self.accounts.vault_authority, &[VAULT_SEED], program_id)?;

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        {
            if !self.accounts.redemption_op.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.accounts.redemption_op.try_borrow()?;
            let op = RedemptionOperation::from_bytes(&data)?;

            if op.status != STATUS_PENDING {
                return Err(RedemptionError::NotPending.into());
            }

            if now <= op.deadline() {
                return Err(RedemptionError::DeadlineNotPassed.into());
            }
        }

        cpi_token_2022_transfer(
            self.accounts.vault,
            self.accounts.token_mint,
            self.accounts.user_token_account,
            self.accounts.vault_authority,
            vault_bump,
            self.accounts.token_2022_program,
            self.accounts.extra_account_meta_list,
            self.accounts.permission_manager_program,
            self.accounts.spiko_token_program,
            self.accounts.token_config,
            self.accounts.vault_authority_perms,
            self.accounts.user_perms,
            self.accounts.hook_event_authority,
            self.accounts.hook_program,
            self.data.amount,
        )?;

        {
            let mut data = self.accounts.redemption_op.try_borrow_mut()?;
            let op = RedemptionOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_CANCELED;
        }

        let event_data = build_redemption_canceled_event(
            &self.accounts.caller.address().to_bytes(),
            &self.data.user,
            &self.accounts.token_mint.address().to_bytes(),
            self.data.amount,
            self.data.salt,
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
