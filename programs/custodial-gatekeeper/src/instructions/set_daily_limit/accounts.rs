use pinocchio::{account::AccountView, error::ProgramError};

use spiko_common::InstructionAccounts;

/// Accounts for the SetDailyLimit instruction.
///
///   0. `[signer, writable]` Admin caller (payer if creating PDA)
///   1. `[]`                 GatekeeperConfig PDA
///   2. `[]`                 PermissionConfig PDA (from permission_manager, for admin check)
///   3. `[writable]`         WithdrawalDailyLimit PDA (created if needed)
///   4. `[]`                 System program
///   5. `[]`                 Event authority PDA
///   6. `[]`                 Self program
pub struct SetDailyLimitAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub daily_limit: &'a AccountView,
    pub system_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for SetDailyLimitAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, perm_config, daily_limit, system_program, event_authority, self_program, ..] =
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
            daily_limit,
            system_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> InstructionAccounts<'a> for SetDailyLimitAccounts<'a> {}
