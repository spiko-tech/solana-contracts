use pinocchio::{account::AccountView, error::ProgramError};

/// Accounts for the RedeemToken instruction.
///
///   0.  `[signer]`   User (token holder)
///   1.  `[]`         TokenConfig PDA (will sign CPI to redemption)
///   2.  `[writable]` User's source token account
///   3.  `[writable]` Vault token account (destination for transfer)
///   4.  `[]`         Token-2022 Mint
///   5.  `[]`         User's UserPermissions PDA (whitelist check)
///   6.  `[]`         Vault authority's UserPermissions PDA (whitelist check)
///   7.  `[]`         Token-2022 program
///   8.  `[]`         Redemption program
///   9.  `[]`         RedemptionConfig PDA (from redemption program)
///  10.  `[writable]` RedemptionOperation PDA (to be created by redemption)
///  11.  `[]`         TokenMinimum PDA (from redemption program)
///  12.  `[]`         System program
///  --- Transfer Hook extra accounts ---
///  13.  `[]`         ExtraAccountMetaList PDA
///  14.  `[]`         PermissionManager program
///  15.  `[]`         SpikoToken program (this program, for hook resolution)
///  16.  `[]`         Transfer Hook event authority PDA
///  17.  `[]`         Transfer Hook program (spiko-transfer-hook)
///  --- Event CPI accounts ---
///  18.  `[]`         Event authority PDA
///  19.  `[]`         Self program (this program)
///  --- Redemption program event CPI accounts ---
///  20.  `[]`         Redemption event authority PDA
///  21.  `[]`         Redemption self program
pub struct RedeemTokenAccounts<'a> {
    pub user: &'a AccountView,
    pub config: &'a AccountView,
    pub user_source: &'a AccountView,
    pub vault: &'a AccountView,
    pub mint: &'a AccountView,
    pub user_perms: &'a AccountView,
    pub vault_authority_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub redemption_program: &'a AccountView,
    pub redemption_config: &'a AccountView,
    pub redemption_op: &'a AccountView,
    pub token_minimum: &'a AccountView,
    pub system_program: &'a AccountView,
    // Transfer Hook extra accounts
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub hook_event_authority: &'a AccountView,
    pub hook_program: &'a AccountView,
    // Event CPI accounts
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
    // Redemption program event CPI accounts
    pub rd_event_authority: &'a AccountView,
    pub rd_self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for RedeemTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [user, config, user_source, vault, mint, user_perms, vault_authority_perms, token_2022_program, redemption_program, redemption_config, redemption_op, token_minimum, system_program, extra_account_meta_list, permission_manager_program, spiko_token_program, hook_event_authority, hook_program, event_authority, self_program, rd_event_authority, rd_self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !user.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            user,
            config,
            user_source,
            vault,
            mint,
            user_perms,
            vault_authority_perms,
            token_2022_program,
            redemption_program,
            redemption_config,
            redemption_op,
            token_minimum,
            system_program,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            hook_event_authority,
            hook_program,
            event_authority,
            self_program,
            rd_event_authority,
            rd_self_program,
        })
    }
}

impl<'a> spiko_common::InstructionAccounts<'a> for RedeemTokenAccounts<'a> {}
