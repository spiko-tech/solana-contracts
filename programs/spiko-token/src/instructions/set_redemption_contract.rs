use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use crate::{
    error::TokenError, events::emit_redemption_contract_set, helpers::require_admin,
    state::TokenConfig,
};

/// Set (or update) the redemption contract address stored in TokenConfig.
///
/// Only the admin (as identified in the PermissionManager's PermissionConfig)
/// can call this. The redemption_contract field is set to the provided
/// program address, or zeroed out to clear it.
///
/// Accounts:
///   0. `[signer]`   Admin caller
///   1. `[writable]` TokenConfig PDA
///   2. `[]`         PermissionConfig PDA (from permission_manager, proves admin identity)
///
/// Data:
///   [0..32]  redemption_contract address (32 bytes; all zeros to clear)
pub struct SetRedemptionContract<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub redemption_contract: [u8; 32],
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for SetRedemptionContract<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, perm_config, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut redemption_contract = [0u8; 32];
        redemption_contract.copy_from_slice(&data[0..32]);

        Ok(Self {
            caller,
            config,
            perm_config,
            redemption_contract,
        })
    }
}

impl<'a> SetRedemptionContract<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        let permission_manager_id = {
            let data = self.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_admin(
            self.caller,
            self.perm_config,
            &permission_manager_id,
            TokenError::Unauthorized.into(),
        )?;

        {
            let mut data = self.config.try_borrow_mut()?;
            let config = TokenConfig::from_bytes_mut(&mut data)?;
            config.redemption_contract = Address::new_from_array(self.redemption_contract);
        }

        emit_redemption_contract_set(
            &self.caller.address().to_bytes(),
            &self.config.address().to_bytes(),
            &self.redemption_contract,
        );

        Ok(())
    }
}
