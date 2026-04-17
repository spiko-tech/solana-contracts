use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the SetRedemptionContract instruction.
///
///   0. `[signer]`   Admin caller
///   1. `[writable]` TokenConfig PDA
///   2. `[]`         PermissionConfig PDA (from permission_manager, proves admin identity)
///   3. `[]`         Event authority PDA
///   4. `[]`         Self program (this program)
pub struct SetRedemptionContractAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for SetRedemptionContractAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, perm_config, event_authority, self_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            caller,
            config,
            perm_config,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for SetRedemptionContractAccounts<'a> {}
