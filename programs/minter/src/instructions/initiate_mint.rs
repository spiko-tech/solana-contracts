use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use permission_manager::state::ROLE_MINT_INITIATOR;

use crate::{
    error::MinterError,
    events::{emit_mint_blocked, emit_mint_executed},
    helpers::{
        compute_operation_id, cpi_spiko_token_mint, create_pda_account, mint_operation_seeds,
        require_permission, verify_pda,
    },
    state::{
        DailyLimit, MintOperation, MinterConfig, DAILY_LIMIT_SEED, DISCRIMINATOR_MINT_OPERATION,
        MINT_OPERATION_SEED, SECONDS_PER_DAY, STATUS_NULL, STATUS_PENDING,
    },
};

/// Initiate a mint operation. If within daily limit, executes immediately.
/// Otherwise, creates a PENDING operation requiring approval.
///
/// Accounts:
///   0.  `[signer]`            Caller (must have ROLE_MINT_INITIATOR)
///   1.  `[]`                  MinterConfig PDA
///   2.  `[writable]`          DailyLimit PDA for this token
///   3.  `[writable]`          MintOperation PDA (created if blocked; unused if immediate)
///   4.  `[]`                  Caller's UserPermissions PDA (from permission_manager)
///   5.  `[]`                  Spiko Token program (for CPI if immediate mint)
///   6.  `[]`                  TokenConfig PDA (on spiko-token, for CPI)
///   7.  `[writable]`          Token-2022 Mint (for CPI)
///   8.  `[writable]`          Recipient's token account (for CPI)
///   9.  `[]`                  Mint authority PDA (spiko-token, for CPI)
///  10.  `[]`                  Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)
///  11.  `[]`                  Recipient's UserPermissions PDA (whitelist check, for CPI)
///  12.  `[]`                  Token-2022 program
///  13.  `[]`                  System program (for PDA creation if blocked)
///
/// Data:
///   [0..32]  user/recipient address (32 bytes)
///   [32..40] amount (u64, little-endian)
///   [40..48] salt (u64, little-endian)
pub struct InitiateMint<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub daily_limit: &'a AccountView,
    pub mint_operation: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub recipient_token_account: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub minter_user_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub system_program: &'a AccountView,
    pub user: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitiateMint<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, daily_limit, mint_operation, caller_perms, spiko_token_program, token_config, token_mint, recipient_token_account, mint_authority, minter_user_perms, recipient_perms, token_2022_program, system_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 32 bytes user + 8 bytes amount + 8 bytes salt = 48 bytes
        if data.len() < 48 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut user = [0u8; 32];
        user.copy_from_slice(&data[0..32]);
        let amount = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let salt = u64::from_le_bytes(data[40..48].try_into().unwrap());

        Ok(Self {
            caller,
            config,
            daily_limit,
            mint_operation,
            caller_perms,
            spiko_token_program,
            token_config,
            token_mint,
            recipient_token_account,
            mint_authority,
            minter_user_perms,
            recipient_perms,
            token_2022_program,
            system_program,
            user,
            amount,
            salt,
        })
    }
}

impl<'a> InitiateMint<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let (permission_manager_id, max_delay, config_bump) = {
            if !self.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.max_delay(),
                config.bump,
            )
        };

        require_permission(
            self.caller_perms,
            &permission_manager_id,
            ROLE_MINT_INITIATOR,
            MinterError::Unauthorized.into(),
        )?;

        let mint_key_bytes = self.token_mint.address().to_bytes();
        let operation_id =
            compute_operation_id(&self.user, &mint_key_bytes, self.amount, self.salt);

        let op_bump = verify_pda(
            self.mint_operation,
            &[MINT_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        {
            let op_data = self.mint_operation.try_borrow()?;
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
            self.daily_limit,
            &[DAILY_LIMIT_SEED, &mint_key_bytes],
            program_id,
        )?;

        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let current_day = now / SECONDS_PER_DAY;

        let within_limit = {
            if !self.daily_limit.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let mut data = self.daily_limit.try_borrow_mut()?;
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
                .checked_add(self.amount)
                .ok_or::<ProgramError>(MinterError::ArithmeticOverflow.into())?
                <= limit
            {
                dl.set_used_amount(used + self.amount);
                true
            } else {
                false
            }
        };

        if within_limit {
            cpi_spiko_token_mint(
                self.config, // MinterConfig PDA signs as the "caller" for spiko_token
                config_bump,
                self.token_config,
                self.token_mint,
                self.recipient_token_account,
                self.mint_authority,
                self.minter_user_perms,
                self.recipient_perms,
                self.token_2022_program,
                self.spiko_token_program,
                self.amount,
            )?;

            emit_mint_executed(
                &self.caller.address().to_bytes(),
                &self.user,
                &self.token_mint.address().to_bytes(),
                self.amount,
                self.salt,
            );
        } else {
            let op_bump_bytes = [op_bump];
            let op_seeds = mint_operation_seeds(&operation_id, &op_bump_bytes);
            let op_signer = Signer::from(&op_seeds);

            create_pda_account(
                self.caller,
                self.mint_operation,
                MintOperation::LEN,
                program_id,
                &[op_signer],
            )?;

            {
                let mut data = self.mint_operation.try_borrow_mut()?;
                let op = MintOperation::from_bytes_mut(&mut data)?;
                op.discriminator = DISCRIMINATOR_MINT_OPERATION;
                op.bump = op_bump;
                op.status = STATUS_PENDING;
                op.set_deadline(now + max_delay);
            }

            emit_mint_blocked(
                &self.caller.address().to_bytes(),
                &self.user,
                &self.token_mint.address().to_bytes(),
                self.amount,
                self.salt,
            );
        }

        Ok(())
    }
}
