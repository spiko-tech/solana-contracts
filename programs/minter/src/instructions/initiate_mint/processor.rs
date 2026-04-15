use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};
use spiko_common::{AccountDeserialize, AccountSize};

use permission_manager::state::ROLE_MINT_INITIATOR;

use crate::{
    error::MinterError,
    events::{build_mint_blocked_event, build_mint_executed_event},
    helpers::{
        compute_operation_id, cpi_spiko_token_mint, create_pda_account, mint_operation_seeds,
        require_permission, verify_pda,
    },
    state::{
        DailyLimit, MintOperation, MinterConfig, DAILY_LIMIT_SEED, MINT_OPERATION_SEED,
        SECONDS_PER_DAY, STATUS_NULL, STATUS_PENDING,
    },
};

use super::accounts::InitiateMintAccounts;
use super::data::InitiateMintData;

/// Initiate a mint operation. If within daily limit, executes immediately.
/// Otherwise, creates a PENDING operation requiring approval.
pub struct InitiateMint<'a> {
    pub accounts: InitiateMintAccounts<'a>,
    pub data: InitiateMintData,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitiateMint<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: InitiateMintAccounts::try_from(accounts)?,
            data: InitiateMintData::try_from(data)?,
        })
    }
}

impl<'a> InitiateMint<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let (permission_manager_id, max_delay, config_bump) = {
            if !self.accounts.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.accounts.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.max_delay(),
                config.bump,
            )
        };

        require_permission(
            self.accounts.caller.address(),
            self.accounts.caller_perms,
            &permission_manager_id,
            ROLE_MINT_INITIATOR,
            MinterError::Unauthorized.into(),
        )?;

        let mint_key_bytes = self.accounts.token_mint.address().to_bytes();
        let operation_id = compute_operation_id(
            &self.data.user,
            &mint_key_bytes,
            self.data.amount,
            self.data.salt,
        );

        let op_bump = verify_pda(
            self.accounts.mint_operation,
            &[MINT_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        {
            let op_data = self.accounts.mint_operation.try_borrow()?;
            if !op_data.is_empty() && op_data[0] != 0 {
                // Account exists with data — check if it's already been used
                if op_data.len() >= MintOperation::LEN {
                    let op = MintOperation::from_bytes(&op_data)?;
                    if op.status != STATUS_NULL {
                        return Err(MinterError::OperationExists.into());
                    }
                }
            }
        }

        verify_pda(
            self.accounts.daily_limit,
            &[DAILY_LIMIT_SEED, &mint_key_bytes],
            program_id,
        )?;

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let current_day = now / SECONDS_PER_DAY;

        let within_limit = {
            if !self.accounts.daily_limit.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let mut data = self.accounts.daily_limit.try_borrow_mut()?;
            let dl = DailyLimit::from_bytes_mut(&mut data)?;

            if current_day != dl.last_day() {
                dl.set_used_amount(0);
                dl.set_last_day(current_day);
            }

            let limit = dl.limit();
            let used = dl.used_amount();

            if limit == 0 {
                // No limit configured — all mints are blocked
                false
            } else if used
                .checked_add(self.data.amount)
                .ok_or::<ProgramError>(MinterError::ArithmeticOverflow.into())?
                <= limit
            {
                dl.set_used_amount(used + self.data.amount);
                true
            } else {
                false
            }
        };

        if within_limit {
            cpi_spiko_token_mint(
                self.accounts.config, // MinterConfig PDA signs as the "caller" for spiko_token
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

            let event_data = build_mint_executed_event(
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
        } else {
            let op_bump_bytes = [op_bump];
            let op_seeds = mint_operation_seeds(&operation_id, &op_bump_bytes);
            let op_signer = Signer::from(&op_seeds);

            create_pda_account(
                self.accounts.caller,
                self.accounts.mint_operation,
                MintOperation::LEN,
                program_id,
                &[op_signer],
            )?;

            {
                let mut data = self.accounts.mint_operation.try_borrow_mut()?;
                let op = MintOperation::from_bytes_mut_init(&mut data)?;
                op.bump = op_bump;
                op.status = STATUS_PENDING;
                op.set_deadline(now + max_delay);
            }

            let event_data = build_mint_blocked_event(
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
        }

        Ok(())
    }
}
