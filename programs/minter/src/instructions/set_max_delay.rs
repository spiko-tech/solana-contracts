use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use crate::{
    error::MinterError, events::emit_max_delay_updated, helpers::require_admin, state::MinterConfig,
};

/// Set the maximum pending duration for blocked mints. Admin only.
///
/// Accounts:
///   0. `[signer]`   Admin caller
///   1. `[writable]` MinterConfig PDA
///   2. `[]`         PermissionConfig PDA (from permission_manager, for admin check)
///
/// Data:
///   [0..8] max_delay (i64, little-endian, seconds)
pub struct SetMaxDelay<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub max_delay: i64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for SetMaxDelay<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, perm_config, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.len() < 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let max_delay = i64::from_le_bytes(data[0..8].try_into().unwrap());

        Ok(Self {
            caller,
            config,
            perm_config,
            max_delay,
        })
    }
}

impl<'a> SetMaxDelay<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Read MinterConfig to get permission_manager address
        let permission_manager_id = {
            if !self.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        // 2. Verify caller is admin
        require_admin(
            self.caller,
            self.perm_config,
            &permission_manager_id,
            MinterError::Unauthorized.into(),
        )?;

        // 3. Update max_delay
        {
            let mut data = self.config.try_borrow_mut()?;
            let config = MinterConfig::from_bytes_mut(&mut data)?;
            config.set_max_delay(self.max_delay);
        }

        emit_max_delay_updated(&self.caller.address().to_bytes(), self.max_delay);

        Ok(())
    }
}
