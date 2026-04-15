use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the TransferOwnership instruction.
///
///   0. `[signer]`   Current admin
///   1. `[writable]` PermissionConfig PDA
///   2. `[]`         Event authority PDA
///   3. `[]`         Self program (this program)
pub struct TransferOwnershipAccounts<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for TransferOwnershipAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [admin, config, event_authority, self_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            admin,
            config,
            event_authority,
            self_program,
        })
    }
}
