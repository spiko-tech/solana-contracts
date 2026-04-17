use pinocchio::{account::AccountView, error::ProgramError};

use spiko_common::InstructionAccounts;

/// Accounts for the CustodialWithdraw instruction.
///
/// The custodial wallet (sender) initiates a withdrawal to a self-custodial
/// wallet (recipient). Tokens are first transferred from sender to the vault,
/// then either forwarded instantly (within daily limit) or held pending approval.
///
///   0. `[signer, writable]`  Sender (custodial wallet, must have WHITELISTED)
///   1. `[]`                  GatekeeperConfig PDA
///   2. `[writable]`          WithdrawalDailyLimit PDA
///   3. `[writable]`          WithdrawalOperation PDA (created if over limit)
///   4. `[writable]`          Sender's token account (source)
///   5. `[writable]`          Vault token account (intermediate)
///   6. `[writable]`          Recipient's token account (destination if instant)
///   7. `[]`                  Vault authority PDA
///   8. `[]`                  Token-2022 Mint
///   9. `[]`                  Sender's UserPermissions PDA
///  10. `[]`                  Recipient's UserPermissions PDA
///  11. `[]`                  Vault authority's UserPermissions PDA
///  12. `[]`                  Token-2022 program
///  13. `[]`                  System program
///  --- Transfer Hook extra accounts ---
///  14. `[]`                  ExtraAccountMetaList PDA
///  15. `[]`                  PermissionManager program
///  16. `[]`                  SpikoToken program
///  17. `[]`                  TokenConfig PDA (spiko-token)
///  18. `[]`                  Hook event authority PDA
///  19. `[]`                  Hook program (spiko-transfer-hook)
///  --- Self-CPI event emission ---
///  20. `[]`                  Event authority PDA
///  21. `[]`                  Self program
pub struct CustodialWithdrawAccounts<'a> {
    pub sender: &'a AccountView,
    pub config: &'a AccountView,
    pub daily_limit: &'a AccountView,
    pub withdrawal_op: &'a AccountView,
    pub sender_token_account: &'a AccountView,
    pub vault_token_account: &'a AccountView,
    pub recipient_token_account: &'a AccountView,
    pub vault_authority: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub sender_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub vault_authority_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub system_program: &'a AccountView,
    // Transfer Hook extra accounts
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub hook_event_authority: &'a AccountView,
    pub hook_program: &'a AccountView,
    // Self-CPI event
    pub event_authority: &'a AccountView,
    pub self_program: &'a AccountView,
}

impl<'a> TryFrom<&'a [AccountView]> for CustodialWithdrawAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountView]) -> Result<Self, Self::Error> {
        let [sender, config, daily_limit, withdrawal_op, sender_token_account, vault_token_account, recipient_token_account, vault_authority, token_mint, sender_perms, recipient_perms, vault_authority_perms, token_2022_program, system_program, extra_account_meta_list, permission_manager_program, spiko_token_program, token_config, hook_event_authority, hook_program, event_authority, self_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !sender.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            sender,
            config,
            daily_limit,
            withdrawal_op,
            sender_token_account,
            vault_token_account,
            recipient_token_account,
            vault_authority,
            token_mint,
            sender_perms,
            recipient_perms,
            vault_authority_perms,
            token_2022_program,
            system_program,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            token_config,
            hook_event_authority,
            hook_program,
            event_authority,
            self_program,
        })
    }
}

impl<'a> InstructionAccounts<'a> for CustodialWithdrawAccounts<'a> {}
