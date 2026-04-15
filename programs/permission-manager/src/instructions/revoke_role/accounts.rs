use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the RevokeRole instruction.
///
///   0. `[signer]`   Caller
///   1. `[]`         PermissionConfig PDA
///   2. `[writable]` Target user's UserPermissions PDA
///   3. `[]`         Target user address (for PDA derivation, not a signer)
///   4. `[]`         Caller's UserPermissions PDA (for role-hierarchy check)
///   5. `[]`         Event authority PDA
///   6. `[]`         Self program (this program)
pub struct RevokeRoleAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub user_perms: &'a AccountView,
    pub target_user: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for RevokeRoleAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, user_perms, target_user, caller_perms, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            caller,
            config,
            user_perms,
            target_user,
            caller_perms,
            event_authority,
            self_program,
        })
    }
}
