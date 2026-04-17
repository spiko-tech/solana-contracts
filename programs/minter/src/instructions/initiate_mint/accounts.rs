use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the InitiateMint instruction.
///
///   0.  `[signer]`            Caller (must have ROLE_MINT_INITIATOR)
///   1.  `[]`                  MinterConfig PDA
///   2.  `[writable]`          DailyLimit PDA for this token
///   3.  `[writable]`          MintOperation PDA (created if blocked; unused if immediate)
///   4.  `[]`                  Caller's UserPermissions PDA (from permission_manager)
///   5.  `[]`                  Spiko Token program (for CPI if immediate mint)
///   6.  `[]`                  TokenConfig PDA (on spiko-token, for CPI)
///   7.  `[writable]`          Token-2022 Mint (for CPI)
///   8.  `[writable]`          Recipient's token account (for CPI)
///   9.  `[]`                  Mint authority PDA (spiko-token, for CPI)
///  10.  `[]`                  Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)
///  11.  `[]`                  Recipient's UserPermissions PDA (whitelist check, for CPI)
///  12.  `[]`                  Token-2022 program
///  13.  `[]`                  System program (for PDA creation if blocked)
///  14.  `[]`                  Spiko Token event authority PDA (for CPI)
///  15.  `[]`                  Spiko Token self program (for CPI)
///  16.  `[]`                  Event authority PDA
///  17.  `[]`                  Self program
pub struct InitiateMintAccounts<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub daily_limit: &'a AccountView,
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
    pub system_program: &'a AccountView,
    pub st_event_authority: &'a AccountView,
    pub st_self_program: &'a AccountView,
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for InitiateMintAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [caller, config, daily_limit, mint_operation, caller_perms, spiko_token_program, token_config, token_mint, recipient_token_account, mint_authority, minter_user_perms, recipient_perms, token_2022_program, system_program, st_event_authority, st_self_program, event_authority, self_program, ..] =
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
            daily_limit,
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
            system_program,
            st_event_authority,
            st_self_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for InitiateMintAccounts<'a> {}
