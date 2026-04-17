use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the ApproveMint instruction.
///
///   0.  `[signer]`            Caller (must have ROLE_MINT_APPROVER)
///   1.  `[]`                  MinterConfig PDA
///   2.  `[writable]`          MintOperation PDA
///   3.  `[]`                  Caller's UserPermissions PDA (from permission_manager)
///   4.  `[]`                  Spiko Token program (for CPI)
///   5.  `[]`                  TokenConfig PDA (on spiko-token, for CPI)
///   6.  `[writable]`          Token-2022 Mint (for CPI)
///   7.  `[writable]`          Recipient's token account (for CPI)
///   8.  `[]`                  Mint authority PDA (spiko-token, for CPI)
///   9.  `[]`                  Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)
///  10.  `[]`                  Recipient's UserPermissions PDA (whitelist check, for CPI)
///  11.  `[]`                  Token-2022 program
///  12.  `[]`                  Spiko Token event authority PDA (for CPI)
///  13.  `[]`                  Spiko Token self program (for CPI)
///  14.  `[]`                  Event authority PDA
///  15.  `[]`                  Self program
pub struct ApproveMintAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint_operation: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub recipient_token_account: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub minter_user_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub st_event_authority: &'a AccountView,
    pub st_self_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for ApproveMintAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, mint_operation, caller_perms, spiko_token_program, token_config, token_mint, recipient_token_account, mint_authority, minter_user_perms, recipient_perms, token_2022_program, st_event_authority, st_self_program, event_authority, self_program, ..] =
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
            spiko_token_program,
            token_config,
            token_mint,
            recipient_token_account,
            mint_authority,
            minter_user_perms,
            recipient_perms,
            token_2022_program,
            st_event_authority,
            st_self_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for ApproveMintAccounts<'a> {}
