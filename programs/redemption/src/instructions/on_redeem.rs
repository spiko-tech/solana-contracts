use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use crate::{
    error::RedemptionError,
    events::emit_redemption_initiated,
    helpers::{compute_operation_id, create_pda_account, redemption_operation_seeds, verify_pda},
    state::{
        RedemptionConfig, RedemptionOperation, TokenMinimum, DISCRIMINATOR_REDEMPTION_OPERATION,
        MAX_DELAY, REDEMPTION_OPERATION_SEED, STATUS_NULL, STATUS_PENDING, TOKEN_MINIMUM_SEED,
    },
};

/// Handle a redemption initiated via CPI from spiko_token::redeem.
///
/// This instruction can ONLY be called via CPI from the spiko_token program.
/// The proof of origin is that `token_config` (a PDA owned by spiko_token)
/// must be a signer — only the spiko_token program can produce that signature.
///
/// The spiko_token::redeem instruction has already:
///   - Verified whitelist on user and vault authority
///   - Transferred tokens from user to vault
///
/// This instruction:
///   1. Verifies token_config is a signer (proves CPI from spiko_token)
///   2. Validates amount >= TokenMinimum for this mint
///   3. Computes operation_id = SHA256(user, mint, amount, salt)
///   4. Creates PENDING RedemptionOperation PDA
///   5. Sets deadline = now + MAX_DELAY
///
/// Accounts:
///   0. `[signer]`            TokenConfig PDA (from spiko_token — proves CPI origin)
///   1. `[signer, writable]`  User (payer for PDA creation)
///   2. `[]`                  RedemptionConfig PDA
///   3. `[writable]`          RedemptionOperation PDA (to be created)
///   4. `[]`                  TokenMinimum PDA
///   5. `[]`                  Token-2022 Mint
///   6. `[]`                  System program
///
/// Data (48 bytes, after discriminator is stripped by dispatch):
///   [0..32]  user address (32 bytes)
///   [32..40] amount (u64, little-endian)
///   [40..48] salt (u64, little-endian)
pub struct OnRedeem<'a> {
    pub token_config: &'a AccountView,
    pub user: &'a AccountView,
    pub config: &'a AccountView,
    pub redemption_op: &'a AccountView,
    pub token_minimum: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub system_program: &'a AccountView,
    pub user_address: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for OnRedeem<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [token_config, user, config, redemption_op, token_minimum, token_mint, system_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // token_config MUST be a signer (PDA signature from spiko_token)
        if !token_config.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // user must also be a signer (forwarded from the original tx)
        if !user.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 32 + 8 + 8 = 48 bytes
        if data.len() < 48 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut user_address = [0u8; 32];
        user_address.copy_from_slice(&data[0..32]);
        let amount = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let salt = u64::from_le_bytes(data[40..48].try_into().unwrap());

        Ok(Self {
            token_config,
            user,
            config,
            redemption_op,
            token_minimum,
            token_mint,
            system_program,
            user_address,
            amount,
            salt,
        })
    }
}

impl<'a> OnRedeem<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Verify RedemptionConfig is owned by this program
        {
            if !self.config.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let _config = RedemptionConfig::from_bytes(&data)?;
        }

        let mint_key_bytes = self.token_mint.address().to_bytes();

        // 2. Verify amount >= minimum
        {
            verify_pda(
                self.token_minimum,
                &[TOKEN_MINIMUM_SEED, &mint_key_bytes],
                program_id,
            )?;

            if !self.token_minimum.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }

            let data = self.token_minimum.try_borrow()?;
            let tm = TokenMinimum::from_bytes(&data)?;
            if self.amount < tm.minimum_amount() {
                return Err(RedemptionError::BelowMinimum.into());
            }
        }

        // 3. Compute operation_id = SHA256(user, token_mint, amount, salt)
        let operation_id =
            compute_operation_id(&self.user_address, &mint_key_bytes, self.amount, self.salt);

        // 4. Verify RedemptionOperation PDA
        let op_bump = verify_pda(
            self.redemption_op,
            &[REDEMPTION_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        // 5. Check operation doesn't already exist
        {
            let op_data = self.redemption_op.try_borrow()?;
            if !op_data.is_empty() && op_data[0] != 0 {
                if op_data.len() >= RedemptionOperation::LEN {
                    let op = RedemptionOperation::from_bytes(&op_data)?;
                    if op.status != STATUS_NULL {
                        return Err(RedemptionError::OperationExists.into());
                    }
                }
            }
        }

        // 6. Get current time
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        // 7. Create RedemptionOperation PDA
        let op_bump_bytes = [op_bump];
        let op_seeds = redemption_operation_seeds(&operation_id, &op_bump_bytes);
        let op_signer = Signer::from(&op_seeds);

        create_pda_account(
            self.user,
            self.redemption_op,
            RedemptionOperation::LEN,
            program_id,
            &[op_signer],
        )?;

        // 8. Write operation data
        {
            let mut data = self.redemption_op.try_borrow_mut()?;
            let op = RedemptionOperation::from_bytes_mut(&mut data)?;
            op.discriminator = DISCRIMINATOR_REDEMPTION_OPERATION;
            op.bump = op_bump;
            op.status = STATUS_PENDING;
            op.set_deadline(now + MAX_DELAY);
            op.user = Address::new_from_array(self.user_address);
        }

        emit_redemption_initiated(
            &self.user_address,
            &self.token_mint.address().to_bytes(),
            self.amount,
            self.salt,
            now + MAX_DELAY,
        );

        Ok(())
    }
}
