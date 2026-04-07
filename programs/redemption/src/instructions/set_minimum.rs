use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use crate::{
    error::RedemptionError,
    events::emit_token_minimum_updated,
    helpers::{create_pda_account, require_admin, token_minimum_seeds, verify_pda},
    state::{RedemptionConfig, TokenMinimum, DISCRIMINATOR_TOKEN_MINIMUM, TOKEN_MINIMUM_SEED},
};

/// Set the minimum redemption amount for a specific token. Admin only.
///
/// Accounts:
///   0. `[signer, writable]` Admin caller (payer if creating TokenMinimum PDA)
///   1. `[]`                 RedemptionConfig PDA (to read permission_manager)
///   2. `[]`                 PermissionConfig PDA (from permission_manager, for admin check)
///   3. `[writable]`         TokenMinimum PDA (created if needed)
///   4. `[]`                 System program
///
/// Data:
///   [0..32] token mint address (32 bytes)
///   [32..40] minimum amount (u64, little-endian)
pub struct SetMinimum<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub token_minimum: &'a AccountView,
    pub system_program: &'a AccountView,
    pub token_mint: [u8; 32],
    pub minimum: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for SetMinimum<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, perm_config, token_minimum, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut token_mint = [0u8; 32];
        token_mint.copy_from_slice(&data[0..32]);
        let minimum = u64::from_le_bytes(data[32..40].try_into().unwrap());

        Ok(Self {
            caller,
            config,
            perm_config,
            token_minimum,
            system_program,
            token_mint,
            minimum,
        })
    }
}

impl<'a> SetMinimum<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let permission_manager_id = {
            if !self.config.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let config = RedemptionConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_admin(
            self.caller,
            self.perm_config,
            &permission_manager_id,
            RedemptionError::Unauthorized.into(),
        )?;

        let bump = verify_pda(
            self.token_minimum,
            &[TOKEN_MINIMUM_SEED, &self.token_mint],
            program_id,
        )?;

        let needs_creation = self.token_minimum.lamports() == 0;

        if needs_creation {
            let bump_bytes = [bump];
            let seeds = token_minimum_seeds(&self.token_mint, &bump_bytes);
            let signer = Signer::from(&seeds);

            create_pda_account(
                self.caller,
                self.token_minimum,
                TokenMinimum::LEN,
                program_id,
                &[signer],
            )?;
        }

        {
            let mut data = self.token_minimum.try_borrow_mut()?;
            let tm = TokenMinimum::from_bytes_mut(&mut data)?;
            tm.discriminator = DISCRIMINATOR_TOKEN_MINIMUM;
            tm.bump = bump;
            tm.set_minimum_amount(self.minimum);
        }

        emit_token_minimum_updated(
            &self.caller.address().to_bytes(),
            &self.token_mint,
            self.minimum,
        );

        Ok(())
    }
}
