use pinocchio::{account::AccountView, error::ProgramError};

use spiko_common::InstructionAccounts;

/// Accounts for the SetMinimum instruction.
///
///   0. `[signer, writable]` Admin caller (payer if creating TokenMinimum PDA)
///   1. `[]`                 RedemptionConfig PDA (to read permission_manager)
///   2. `[]`                 PermissionConfig PDA (from permission_manager, for admin check)
///   3. `[writable]`         TokenMinimum PDA (created if needed)
///   4. `[]`                 System program
///   5. `[]`                 Event authority PDA
///   6. `[]`                 Self program
pub struct SetMinimumAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub token_minimum: &'a AccountView,
    pub system_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for SetMinimumAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, perm_config, token_minimum, system_program, event_authority, self_program, ..] =
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
            perm_config,
            token_minimum,
            system_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> InstructionAccounts<'a> for SetMinimumAccounts<'a> {}
