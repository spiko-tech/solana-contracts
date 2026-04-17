use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_MINT_APPROVER;

use spiko_events::EventSerialize;

use crate::{
    error::MinterError,
    events::MintApprovedEvent,
    helpers::{compute_operation_id, cpi_spiko_token_mint, require_permission, verify_pda},
    state::{MintOperation, MinterConfig, MINT_OPERATION_SEED, STATUS_DONE, STATUS_PENDING},
};

use super::accounts::ApproveMintAccounts;
use super::data::ApproveMintData;

/// Approve a blocked (PENDING) mint operation. Executes the mint via CPI.
pub struct ApproveMint<'a> {
    pub accounts: ApproveMintAccounts<'a>,
    pub data: ApproveMintData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for ApproveMint<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: ApproveMintAccounts::try_from(accounts)?,
            data: ApproveMintData::try_from(data)?,
        })
    }
}

impl<'a> ApproveMint<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let (permission_manager_id, config_bump) = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.bump,
            )
        };

        require_permission(
            self.accounts.caller.address(),
            self.accounts.caller_perms,
            &permission_manager_id,
            ROLE_MINT_APPROVER,
            MinterError::Unauthorized.into(),
        )?;

        let operation_id = compute_operation_id(
            &self.data.user,
            &self.data.token_mint_key,
            self.data.amount,
            self.data.salt,
        );

        verify_pda(
            self.accounts.mint_operation,
            &[MINT_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        if self.accounts.token_mint.address().to_bytes() != self.data.token_mint_key {
            return Err(ProgramError::InvalidArgument);
        }

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        {
            if !self.accounts.mint_operation.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.accounts.mint_operation.try_borrow()?;
            let op = MintOperation::from_bytes(&data)?;

            if op.status != STATUS_PENDING {
                return Err(MinterError::InvalidMintOperationStatus.into());
            }

            if now > op.deadline() {
                return Err(MinterError::MintDeadlinePassed.into());
            }
        }

        cpi_spiko_token_mint(
            self.accounts.config,
            config_bump,
            self.accounts.token_config,
            self.accounts.token_mint,
            self.accounts.recipient_token_account,
            self.accounts.mint_authority,
            self.accounts.minter_user_perms,
            self.accounts.recipient_perms,
            self.accounts.token_2022_program,
            self.accounts.spiko_token_program,
            self.accounts.st_event_authority,
            self.accounts.st_self_program,
            self.data.amount,
        )?;

        {
            let mut data = self.accounts.mint_operation.try_borrow_mut()?;
            let op = MintOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_DONE;
        }

        let event = MintApprovedEvent::new(
            self.accounts.caller.address().clone(),
            Address::new_from_array(self.data.user),
            Address::new_from_array(self.data.token_mint_key),
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
