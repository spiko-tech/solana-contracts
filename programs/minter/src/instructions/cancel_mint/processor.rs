use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};
use spiko_common::AccountDeserialize;

use permission_manager::state::ROLE_MINT_APPROVER;

use crate::{
    error::MinterError,
    events::build_mint_canceled_event,
    helpers::{compute_operation_id, require_permission, verify_pda},
    state::{MintOperation, MinterConfig, MINT_OPERATION_SEED, STATUS_DONE, STATUS_PENDING},
};

use super::accounts::CancelMintAccounts;
use super::data::CancelMintData;

/// Cancel a PENDING mint operation. No tokens are minted.
pub struct CancelMint<'a> {
    pub accounts: CancelMintAccounts<'a>,
    pub data: CancelMintData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for CancelMint<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: CancelMintAccounts::try_from(accounts)?,
            data: CancelMintData::try_from(data)?,
        })
    }
}

impl<'a> CancelMint<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let permission_manager_id = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
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

        {
            if !self.accounts.mint_operation.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.accounts.mint_operation.try_borrow()?;
            let op = MintOperation::from_bytes(&data)?;

            if op.status != STATUS_PENDING {
                return Err(MinterError::NotPending.into());
            }
        }

        {
            let mut data = self.accounts.mint_operation.try_borrow_mut()?;
            let op = MintOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_DONE;
        }

        let event_data = build_mint_canceled_event(
            &self.accounts.caller.address().to_bytes(),
            &self.data.user,
            &self.data.token_mint_key,
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
