use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use crate::{
    error::MinterError,
    events::emit_minter_initialized,
    helpers::{create_pda_account, minter_config_seeds, verify_pda},
    state::{MinterConfig, DISCRIMINATOR_MINTER_CONFIG, MINTER_CONFIG_SEED},
};

/// Initialize the Minter singleton config.
///
/// Accounts:
///   0. `[signer, writable]` Admin / deployer (payer for PDA creation)
///   1. `[writable]`         MinterConfig PDA (to be created)
///   2. `[]`                 System program
///
/// Data:
///   [0..8]   max_delay (i64, little-endian, seconds)
///   [8..40]  permission_manager program ID (32 bytes)
pub struct InitializeMinter<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub system_program: &'a AccountView,
    pub max_delay: i64,
    pub permission_manager: [u8; 32],
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitializeMinter<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [admin, config, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 8 bytes max_delay + 32 bytes permission_manager = 40
        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let max_delay = i64::from_le_bytes(data[0..8].try_into().unwrap());
        let mut permission_manager = [0u8; 32];
        permission_manager.copy_from_slice(&data[8..40]);

        Ok(Self {
            admin,
            config,
            system_program,
            max_delay,
            permission_manager,
        })
    }
}

impl<'a> InitializeMinter<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Verify MinterConfig PDA
        let bump = verify_pda(self.config, &[MINTER_CONFIG_SEED], program_id)?;

        // 2. Check not already initialized (account should have zero lamports / zero data)
        {
            let data = self.config.try_borrow()?;
            if !data.is_empty() && data[0] != 0 {
                return Err(MinterError::AlreadyInitialized.into());
            }
        }

        // 3. Create MinterConfig PDA
        let bump_bytes = [bump];
        let seeds = minter_config_seeds(&bump_bytes);
        let signer = Signer::from(&seeds);

        create_pda_account(
            self.admin,
            self.config,
            MinterConfig::LEN,
            program_id,
            &[signer],
        )?;

        // 4. Write config data
        {
            let mut data = self.config.try_borrow_mut()?;
            let config = MinterConfig::from_bytes_mut(&mut data)?;
            config.discriminator = DISCRIMINATOR_MINTER_CONFIG;
            config.bump = bump;
            config.set_max_delay(self.max_delay);
            config.permission_manager = Address::new_from_array(self.permission_manager);
        }

        emit_minter_initialized(&self.admin.address().to_bytes(), self.max_delay);

        Ok(())
    }
}
