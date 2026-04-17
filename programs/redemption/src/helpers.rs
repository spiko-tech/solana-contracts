use pinocchio::{
    account::AccountView,
    cpi::{Seed, Signer},
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};

pub use spiko_common::{compute_operation_id, create_pda_account, verify_pda};
pub use spiko_permission::{require_admin, require_permission};

use crate::state::{
    REDEMPTION_CONFIG_SEED, REDEMPTION_OPERATION_SEED, TOKEN_MINIMUM_SEED, VAULT_SEED,
};

/// Build and invoke CPI to spiko_token program's burn instruction.
///
/// spiko_token burn (discriminator 2) expects accounts:
///   0. `[signer]`   Caller (must have BURNER role)
///   1. `[]`         TokenConfig PDA (spiko-token)
///   2. `[writable]` Token-2022 Mint
///   3. `[writable]` Source token account (tokens to burn)
///   4. `[]`         Mint authority PDA (spiko-token)
///   5. `[]`         Caller's UserPermissions PDA (with ROLE_BURNER)
///   6. `[]`         Token-2022 program
///
/// Data: discriminator(2) + amount(u64 LE, 8 bytes) = 9 bytes
///
/// The redemption program's vault authority PDA must have ROLE_BURNER
/// in the permission_manager to call burn.
pub fn cpi_spiko_token_burn<'a>(
    vault_authority: &AccountView,
    vault_authority_bump: u8,
    token_config: &AccountView,
    token_mint: &AccountView,
    vault_token_account: &AccountView,
    mint_authority: &AccountView,
    vault_authority_perms: &AccountView,
    token_2022_program: &AccountView,
    spiko_token_program: &AccountView,
    st_event_authority: &AccountView,
    st_self_program: &AccountView,
    amount: u64,
) -> ProgramResult {
    let mut ix_data = [0u8; 9];
    ix_data[0] = 2; // discriminator for burn
    ix_data[1..9].copy_from_slice(&amount.to_le_bytes());

    let ix_accounts = [
        InstructionAccount::writable_signer(vault_authority.address()), // caller (PDA signer)
        InstructionAccount::readonly(token_config.address()),           // token config
        InstructionAccount::writable(token_mint.address()),             // token-2022 mint
        InstructionAccount::writable(vault_token_account.address()),    // source token account
        InstructionAccount::readonly(mint_authority.address()),         // mint authority PDA
        InstructionAccount::readonly(vault_authority_perms.address()),  // vault auth's user perms
        InstructionAccount::readonly(token_2022_program.address()),     // token-2022 program
        InstructionAccount::readonly(st_event_authority.address()), // spiko-token event authority PDA
        InstructionAccount::readonly(st_self_program.address()), // spiko-token program (self_program)
    ];

    let instruction = InstructionView {
        program_id: spiko_token_program.address(),
        accounts: &ix_accounts,
        data: &ix_data,
    };

    let bump_bytes = [vault_authority_bump];
    let seeds = vault_authority_seeds(&bump_bytes);
    let signer = Signer::from(&seeds);

    pinocchio::cpi::invoke_signed(
        &instruction,
        &[
            vault_authority,
            token_config,
            token_mint,
            vault_token_account,
            mint_authority,
            vault_authority_perms,
            token_2022_program,
            st_event_authority,
            st_self_program,
            spiko_token_program,
        ],
        &[signer],
    )
}

/// Transfer tokens from vault back to user via Token-2022 transfer.
/// The vault authority PDA signs.
///
/// Includes Transfer Hook extra accounts required by mints with the
/// Transfer Hook extension enabled.
pub fn cpi_token_2022_transfer<'a>(
    vault_token_account: &AccountView,
    token_mint: &AccountView,
    user_token_account: &AccountView,
    vault_authority: &AccountView,
    vault_authority_bump: u8,
    token_2022_program: &AccountView,
    // Transfer Hook extra accounts:
    extra_account_meta_list: &AccountView,
    permission_manager_program: &AccountView,
    spiko_token_program: &AccountView,
    token_config: &AccountView,
    vault_authority_perms: &AccountView,
    user_perms: &AccountView,
    hook_event_authority: &AccountView,
    hook_program: &AccountView,
    amount: u64,
) -> ProgramResult {
    let bump_bytes = [vault_authority_bump];
    let seeds = vault_authority_seeds(&bump_bytes);
    let signer = Signer::from(&seeds);

    let decimals = {
        let mint_data = token_mint.try_borrow()?;
        if mint_data.len() < 45 {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        mint_data[44]
    };

    // Token-2022 TransferChecked (opcode 12):
    let mut ix_data = [0u8; 10];
    ix_data[0] = 12; // TransferChecked opcode
    ix_data[1..9].copy_from_slice(&amount.to_le_bytes());
    ix_data[9] = decimals;

    let ix_accounts = [
        InstructionAccount::writable(vault_token_account.address()), // source (vault)
        InstructionAccount::readonly(token_mint.address()),          // mint
        InstructionAccount::writable(user_token_account.address()),  // destination (user)
        InstructionAccount::readonly_signer(vault_authority.address()), // authority (vault PDA signs)
        // Transfer Hook extra accounts (must match ExtraAccountMetaList order):
        InstructionAccount::readonly(extra_account_meta_list.address()), // [meta] ExtraAccountMetaList
        InstructionAccount::readonly(permission_manager_program.address()), // [0] PermissionManager program
        InstructionAccount::readonly(spiko_token_program.address()), // [1] SpikoToken program
        InstructionAccount::readonly(token_config.address()),        // [2] TokenConfig
        InstructionAccount::readonly(vault_authority_perms.address()), // [3] Sender perms (vault authority)
        InstructionAccount::readonly(user_perms.address()),            // [4] Recipient perms (user)
        InstructionAccount::readonly(hook_event_authority.address()),  // [5] Hook event authority
        InstructionAccount::readonly(hook_program.address()),          // [6] Hook program
    ];

    let instruction = InstructionView {
        program_id: token_2022_program.address(),
        accounts: &ix_accounts,
        data: &ix_data,
    };

    pinocchio::cpi::invoke_signed(
        &instruction,
        &[
            vault_token_account,
            token_mint,
            user_token_account,
            vault_authority,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            token_config,
            vault_authority_perms,
            user_perms,
            hook_event_authority,
            hook_program,
            token_2022_program,
        ],
        &[signer],
    )
}

/// Seeds: ["redemption_config", bump]
#[inline]
pub fn redemption_config_seeds<'a>(bump: &'a [u8; 1]) -> [Seed<'a>; 2] {
    [
        Seed::from(REDEMPTION_CONFIG_SEED),
        Seed::from(bump.as_ref()),
    ]
}

/// Seeds: ["minimum", mint_pubkey, bump]
#[inline]
pub fn token_minimum_seeds<'a>(mint_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(TOKEN_MINIMUM_SEED),
        Seed::from(mint_key),
        Seed::from(bump.as_ref()),
    ]
}

/// Seeds: ["redemption_op", operation_id, bump]
#[inline]
pub fn redemption_operation_seeds<'a>(operation_id: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(REDEMPTION_OPERATION_SEED),
        Seed::from(operation_id),
        Seed::from(bump.as_ref()),
    ]
}

/// Seeds: ["vault", bump]
#[inline]
pub fn vault_authority_seeds<'a>(bump: &'a [u8; 1]) -> [Seed<'a>; 2] {
    [Seed::from(VAULT_SEED), Seed::from(bump.as_ref())]
}
