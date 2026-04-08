use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use permission_manager::state::ROLE_PAUSER;

use crate::{
    error::TokenError, events::emit_token_paused, helpers::require_permission, state::TokenConfig,
};

/// Pause the token -- blocks all transfers, mints, and burns.
///
/// Accounts:
///   0. `[signer]`   Caller (must have PAUSER role)
///   1. `[writable]` TokenConfig PDA
///   2. `[]`         Caller's UserPermissions PDA (from permission_manager)
pub struct Pause<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub caller_perms: &'a AccountView,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Pause<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, caller_perms, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            caller,
            config,
            caller_perms,
        })
    }
}

impl<'a> Pause<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        let permission_manager_id = {
            let data = self.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        require_permission(
            self.caller.address(),
            self.caller_perms,
            &permission_manager_id,
            ROLE_PAUSER,
            TokenError::Unauthorized.into(),
        )?;

        {
            let mut data = self.config.try_borrow_mut()?;
            let config = TokenConfig::from_bytes_mut(&mut data)?;
            config.paused = 1;
        }

        emit_token_paused(
            &self.caller.address().to_bytes(),
            &self.config.address().to_bytes(),
        );

        Ok(())
    }
}
