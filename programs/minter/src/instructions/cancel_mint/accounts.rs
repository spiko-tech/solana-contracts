use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the CancelMint instruction.
///
///   0. `[signer]`   Caller (must have ROLE_MINT_APPROVER)
///   1. `[]`         MinterConfig PDA
///   2. `[writable]` MintOperation PDA
///   3. `[]`         Caller's UserPermissions PDA (from permission_manager)
///   4. `[]`         Event authority PDA
///   5. `[]`         Self program
pub struct CancelMintAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint_operation: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CancelMintAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, mint_operation, caller_perms, event_authority, self_program, ..] =
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
            mint_operation,
            caller_perms,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for CancelMintAccounts<'a> {}
