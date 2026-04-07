use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use crate::{
    error::RedemptionError,
    events::emit_redemption_initialized,
    helpers::{create_pda_account, redemption_config_seeds, verify_pda},
    state::{RedemptionConfig, DISCRIMINATOR_REDEMPTION_CONFIG, REDEMPTION_CONFIG_SEED},
};

/// Initialize the Redemption singleton config.
///
/// Accounts:
///   0. `[signer, writable]` Admin / deployer (payer for PDA creation)
///   1. `[writable]`         RedemptionConfig PDA (to be created)
///   2. `[]`                 System program
///
/// Data:
///   [0..32] permission_manager program ID (32 bytes)
pub struct InitializeRedemption<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub system_program: &'a AccountView,
    pub permission_manager: [u8; 32],
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitializeRedemption<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [admin, config, system_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 32 bytes permission_manager
        if data.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut permission_manager = [0u8; 32];
        permission_manager.copy_from_slice(&data[0..32]);

        Ok(Self {
            admin,
            config,
            system_program,
            permission_manager,
        })
    }
}

impl<'a> InitializeRedemption<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let bump = verify_pda(self.config, &[REDEMPTION_CONFIG_SEED], program_id)?;

        {
            let data = self.config.try_borrow()?;
            if !data.is_empty() && data[0] != 0 {
                return Err(RedemptionError::AlreadyInitialized.into());
            }
        }

        let bump_bytes = [bump];
        let seeds = redemption_config_seeds(&bump_bytes);
        let signer = Signer::from(&seeds);

        create_pda_account(
            self.admin,
            self.config,
            RedemptionConfig::LEN,
            program_id,
            &[signer],
        )?;

        // 4. Write config data
        {
            let mut data = self.config.try_borrow_mut()?;
            let config = RedemptionConfig::from_bytes_mut(&mut data)?;
            config.discriminator = DISCRIMINATOR_REDEMPTION_CONFIG;
            config.bump = bump;
            config.permission_manager = Address::new_from_array(self.permission_manager);
        }

        emit_redemption_initialized(&self.admin.address().to_bytes());

        Ok(())
    }
}
