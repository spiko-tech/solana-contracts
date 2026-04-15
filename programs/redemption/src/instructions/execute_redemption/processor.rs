use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_REDEMPTION_EXECUTOR;

use crate::{
    error::RedemptionError,
    events::build_redemption_executed_event,
    helpers::{compute_operation_id, cpi_spiko_token_burn, require_permission, verify_pda},
    state::{
        RedemptionConfig, RedemptionOperation, REDEMPTION_OPERATION_SEED, STATUS_EXECUTED,
        STATUS_PENDING, VAULT_SEED,
    },
};

use super::accounts::ExecuteRedemptionAccounts;
use super::data::ExecuteRedemptionData;

/// Execute a pending redemption. Burns the held tokens via CPI to spiko_token.burn().
pub struct ExecuteRedemption<'a> {
    pub accounts: ExecuteRedemptionAccounts<'a>,
    pub data: ExecuteRedemptionData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for ExecuteRedemption<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: ExecuteRedemptionAccounts::try_from(accounts)?,
            data: ExecuteRedemptionData::try_from(data)?,
        })
    }
}

impl<'a> ExecuteRedemption<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let permission_manager_id = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = RedemptionConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_permission(
            self.accounts.operator.address(),
            self.accounts.operator_perms,
            &permission_manager_id,
            ROLE_REDEMPTION_EXECUTOR,
            RedemptionError::Unauthorized.into(),
        )?;

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

            if now > op.deadline() {
                return Err(RedemptionError::DeadlinePassed.into());
            }
        }

        cpi_spiko_token_burn(
            self.accounts.vault_authority,
            vault_bump,
            self.accounts.token_config,
            self.accounts.token_mint,
            self.accounts.vault,
            self.accounts.mint_authority,
            self.accounts.vault_authority_perms,
            self.accounts.token_2022_program,
            self.accounts.spiko_token_program,
            self.accounts.st_event_authority,
            self.accounts.st_self_program,
            self.data.amount,
        )?;

        {
            let mut data = self.accounts.redemption_op.try_borrow_mut()?;
            let op = RedemptionOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_EXECUTED;
        }

        let event_data = build_redemption_executed_event(
            &self.accounts.operator.address().to_bytes(),
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
