use pinocchio::{account::AccountView, error::ProgramError};

use spiko_common::InstructionAccounts;

/// Accounts for the OnRedeem instruction.
///
///   0. `[signer]`            TokenConfig PDA (from spiko_token — proves CPI origin)
///   1. `[signer, writable]`  User (payer for PDA creation)
///   2. `[]`                  RedemptionConfig PDA
///   3. `[writable]`          RedemptionOperation PDA (to be created)
///   4. `[]`                  TokenMinimum PDA
///   5. `[]`                  Token-2022 Mint
///   6. `[]`                  System program
///   7. `[]`                  Event authority PDA
///   8. `[]`                  Self program
pub struct OnRedeemAccounts<'a> {
    pub token_config: &'a AccountView,
    pub user: &'a AccountView,
    pub config: &'a AccountView,
    pub redemption_op: &'a AccountView,
    pub token_minimum: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub system_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for OnRedeemAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [token_config, user, config, redemption_op, token_minimum, token_mint, system_program, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // token_config MUST be a signer (PDA signature from spiko_token)
        if !token_config.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // user must also be a signer (forwarded from the original tx)
        if !user.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            token_config,
            user,
            config,
            redemption_op,
            token_minimum,
            token_mint,
            system_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> InstructionAccounts<'a> for OnRedeemAccounts<'a> {}
