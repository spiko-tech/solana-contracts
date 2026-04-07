use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use crate::{
    error::MinterError,
    events::emit_daily_limit_updated,
    helpers::{create_pda_account, daily_limit_seeds, require_admin, verify_pda},
    state::{DailyLimit, MinterConfig, DAILY_LIMIT_SEED, DISCRIMINATOR_DAILY_LIMIT},
};

/// Set the daily mint limit for a specific token. Admin only.
///
/// Accounts:
///   0. `[signer, writable]` Admin caller (payer if creating DailyLimit PDA)
///   1. `[]`                 MinterConfig PDA (to read permission_manager)
///   2. `[]`                 PermissionConfig PDA (from permission_manager, for admin check)
///   3. `[writable]`         DailyLimit PDA for the token (created if needed)
///   4. `[]`                 System program
///
/// Data:
///   [0..32]  token mint address (32 bytes, used for PDA derivation)
///   [32..40] limit amount (u64, little-endian)
pub struct SetDailyLimit<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub daily_limit: &'a AccountView,
    pub system_program: &'a AccountView,
    pub token_mint: [u8; 32],
    pub limit: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for SetDailyLimit<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, perm_config, daily_limit, system_program, ..] = accounts else {
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
        let limit = u64::from_le_bytes(data[32..40].try_into().unwrap());

        Ok(Self {
            caller,
            config,
            perm_config,
            daily_limit,
            system_program,
            token_mint,
            limit,
        })
    }
}

impl<'a> SetDailyLimit<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let permission_manager_id = {
            if !self.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_admin(
            self.caller,
            self.perm_config,
            &permission_manager_id,
            MinterError::Unauthorized.into(),
        )?;

        let bump = verify_pda(
            self.daily_limit,
            &[DAILY_LIMIT_SEED, &self.token_mint],
            program_id,
        )?;

        let needs_creation = self.daily_limit.lamports() == 0;

        if needs_creation {
            // Create the DailyLimit PDA
            let bump_bytes = [bump];
            let seeds = daily_limit_seeds(&self.token_mint, &bump_bytes);
            let signer = Signer::from(&seeds);

            create_pda_account(
                self.caller,
                self.daily_limit,
                DailyLimit::LEN,
                program_id,
                &[signer],
            )?;
        }

        {
            let mut data = self.daily_limit.try_borrow_mut()?;
            let dl = DailyLimit::from_bytes_mut(&mut data)?;
            dl.discriminator = DISCRIMINATOR_DAILY_LIMIT;
            dl.bump = bump;
            dl.set_limit(self.limit);
            // If newly created, used_amount and last_day are already zero
        }

        emit_daily_limit_updated(
            &self.caller.address().to_bytes(),
            &self.token_mint,
            self.limit,
        );

        Ok(())
    }
}
