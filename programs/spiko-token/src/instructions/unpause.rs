use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use permission_manager::state::ROLE_PAUSER;

use crate::{
    error::TokenError, events::emit_token_unpaused, helpers::require_permission, state::TokenConfig,
};

/// Unpause the token -- resumes normal operations.
///
/// Accounts:
///   0. `[signer]`   Caller (must have PAUSER role)
///   1. `[writable]` TokenConfig PDA
///   2. `[]`         Caller's UserPermissions PDA (from permission_manager)
pub struct Unpause<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub caller_perms: &'a AccountView,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for Unpause<'a> {
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

impl<'a> Unpause<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Verify TokenConfig is owned by this program and initialized
        if !self.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        // 2. Read config to get the permission_manager address
        let permission_manager_id = {
            let data = self.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        // 3. Check caller has PAUSER role
        require_permission(
            self.caller_perms,
            &permission_manager_id,
            ROLE_PAUSER,
            TokenError::Unauthorized.into(),
        )?;

        // 4. Set paused = 0
        {
            let mut data = self.config.try_borrow_mut()?;
            let config = TokenConfig::from_bytes_mut(&mut data)?;
            config.paused = 0;
        }

        emit_token_unpaused(
            &self.caller.address().to_bytes(),
            &self.config.address().to_bytes(),
        );

        Ok(())
    }
}
