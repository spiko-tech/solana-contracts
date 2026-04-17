use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the AcceptOwnership instruction.
///
///   0. `[signer]`   New admin (must be the pending_admin)
///   1. `[writable]` PermissionConfig PDA
///   2. `[]`         Event authority PDA
///   3. `[]`         Self program (this program)
pub struct AcceptOwnershipAccounts<'a> {
    pub new_admin: &'a AccountView,
    pub config: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for AcceptOwnershipAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [new_admin, config, event_authority, self_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !new_admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            new_admin,
            config,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for AcceptOwnershipAccounts<'a> {}
