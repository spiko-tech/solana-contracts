use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the Pause instruction.
///
///   0. `[signer]`   Caller (must have PAUSER role)
///   1. `[writable]` TokenConfig PDA
///   2. `[]`         Caller's UserPermissions PDA (from permission_manager)
///   3. `[]`         Event authority PDA
///   4. `[]`         Self program (this program)
pub struct PauseAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for PauseAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, caller_perms, event_authority, self_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            caller,
            config,
            caller_perms,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for PauseAccounts<'a> {}
