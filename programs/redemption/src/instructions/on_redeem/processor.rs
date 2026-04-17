use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use spiko_common::{AccountDeserialize, AccountSize};

use crate::{
    error::RedemptionError,
    events::{event_authority_pda, RedemptionInitiatedEvent},
    helpers::{compute_operation_id, create_pda_account, redemption_operation_seeds, verify_pda},
    state::{
        RedemptionConfig, RedemptionOperation, TokenMinimum, MAX_DELAY, REDEMPTION_OPERATION_SEED,
        STATUS_NULL, STATUS_PENDING, TOKEN_MINIMUM_SEED,
    },
};

use spiko_events::EventSerialize;

use super::accounts::OnRedeemAccounts;
use super::data::OnRedeemData;

/// Handle a redemption initiated via CPI from spiko_token::redeem.
pub struct OnRedeem<'a> {
    pub accounts: OnRedeemAccounts<'a>,
    pub data: OnRedeemData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for OnRedeem<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: OnRedeemAccounts::try_from(accounts)?,
            data: OnRedeemData::try_from(data)?,
        })
    }
}

impl<'a> OnRedeem<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        {
            if !self.accounts.config.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let _config = RedemptionConfig::from_bytes(&data)?;
        }

        let mint_key_bytes = self.accounts.token_mint.address().to_bytes();

        {
            verify_pda(
                self.accounts.token_minimum,
                &[TOKEN_MINIMUM_SEED, &mint_key_bytes],
                program_id,
            )?;

            if !self.accounts.token_minimum.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }

            let data = self.accounts.token_minimum.try_borrow()?;
            let tm = TokenMinimum::from_bytes(&data)?;
            if self.data.amount < tm.minimum_amount() {
                return Err(RedemptionError::BelowMinimum.into());
            }
        }

        let operation_id = compute_operation_id(
            &self.data.user_address,
            &mint_key_bytes,
            self.data.amount,
            self.data.salt,
        );

        let op_bump = verify_pda(
            self.accounts.redemption_op,
            &[REDEMPTION_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        {
            let op_data = self.accounts.redemption_op.try_borrow()?;
            if !op_data.is_empty() && op_data[0] != 0 {
                if op_data.len() >= RedemptionOperation::LEN {
                    let op = RedemptionOperation::from_bytes(&op_data)?;
                    if op.status != STATUS_NULL {
                        return Err(RedemptionError::OperationExists.into());
                    }
                }
            }
        }

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        let op_bump_bytes = [op_bump];
        let op_seeds = redemption_operation_seeds(&operation_id, &op_bump_bytes);
        let op_signer = Signer::from(&op_seeds);

        create_pda_account(
            self.accounts.user,
            self.accounts.redemption_op,
            RedemptionOperation::LEN,
            program_id,
            &[op_signer],
        )?;

        {
            let mut data = self.accounts.redemption_op.try_borrow_mut()?;
            let op = RedemptionOperation::from_bytes_mut_init(&mut data)?;
            op.bump = op_bump;
            op.status = STATUS_PENDING;
            op.set_deadline(now + MAX_DELAY);
            op.user = Address::new_from_array(self.data.user_address);
        }

        let event = RedemptionInitiatedEvent::new(
            Address::new_from_array(self.data.user_address),
            self.accounts.token_mint.address().clone(),
            self.data.amount,
            self.data.salt,
            now + MAX_DELAY,
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
