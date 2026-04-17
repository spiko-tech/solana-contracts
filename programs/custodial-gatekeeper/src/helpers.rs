use pinocchio::{
    account::AccountView,
    cpi::{Seed, Signer},
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};

pub use spiko_common::{compute_operation_id, create_pda_account, verify_pda};
pub use spiko_permission::{require_admin, require_permission};

use crate::state::{
    GATEKEEPER_CONFIG_SEED, VAULT_SEED, WITHDRAWAL_DAILY_LIMIT_SEED, WITHDRAWAL_OPERATION_SEED,
};

/// Transfer tokens from vault to recipient via Token-2022 TransferChecked.
/// The vault authority PDA signs.
///
/// Includes Transfer Hook extra accounts required by mints with the
/// Transfer Hook extension enabled.
pub fn cpi_token_2022_transfer<'a>(
    vault_token_account: &AccountView,
    token_mint: &AccountView,
    recipient_token_account: &AccountView,
    vault_authority: &AccountView,
    vault_authority_bump: u8,
    token_2022_program: &AccountView,
    // Transfer Hook extra accounts:
    extra_account_meta_list: &AccountView,
    permission_manager_program: &AccountView,
    spiko_token_program: &AccountView,
    token_config: &AccountView,
    vault_authority_perms: &AccountView,
    recipient_perms: &AccountView,
    hook_event_authority: &AccountView,
    hook_program: &AccountView,
    amount: u64,
) -> ProgramResult {
    let bump_bytes = [vault_authority_bump];
    let seeds = vault_authority_seeds(&bump_bytes);
    let signer = Signer::from(&seeds);

    // Read decimals from the on-chain mint account (byte offset 44)
    let decimals = {
        let mint_data = token_mint.try_borrow()?;
        if mint_data.len() < 45 {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        mint_data[44]
    };

    let mut ix_data = [0u8; 10];
    ix_data[0] = 12; // TransferChecked opcode
    ix_data[1..9].copy_from_slice(&amount.to_le_bytes());
    ix_data[9] = decimals;

    let ix_accounts = [
        // Standard TransferChecked accounts:
        InstructionAccount::writable(vault_token_account.address()),
        InstructionAccount::readonly(token_mint.address()),
        InstructionAccount::writable(recipient_token_account.address()),
        InstructionAccount::readonly_signer(vault_authority.address()),
        // Transfer Hook extra accounts:
        InstructionAccount::readonly(extra_account_meta_list.address()),
        InstructionAccount::readonly(permission_manager_program.address()),
        InstructionAccount::readonly(spiko_token_program.address()),
        InstructionAccount::readonly(token_config.address()),
        InstructionAccount::readonly(vault_authority_perms.address()),
        InstructionAccount::readonly(recipient_perms.address()),
        InstructionAccount::readonly(hook_event_authority.address()),
        InstructionAccount::readonly(hook_program.address()),
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
            recipient_token_account,
            vault_authority,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            token_config,
            vault_authority_perms,
            recipient_perms,
            hook_event_authority,
            hook_program,
            token_2022_program,
        ],
        &[signer],
    )
}

// -----------------------------------------------------------------
// Signer seeds builders
// -----------------------------------------------------------------

/// Seeds: ["gatekeeper_config", bump]
#[inline]
pub fn gatekeeper_config_seeds<'a>(bump: &'a [u8; 1]) -> [Seed<'a>; 2] {
    [
        Seed::from(GATEKEEPER_CONFIG_SEED),
        Seed::from(bump.as_ref()),
    ]
}

/// Seeds: ["withdrawal_limit", mint_pubkey, bump]
#[inline]
pub fn withdrawal_daily_limit_seeds<'a>(mint_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(WITHDRAWAL_DAILY_LIMIT_SEED),
        Seed::from(mint_key),
        Seed::from(bump.as_ref()),
    ]
}

/// Seeds: ["withdrawal_op", operation_id, bump]
#[inline]
pub fn withdrawal_operation_seeds<'a>(operation_id: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(WITHDRAWAL_OPERATION_SEED),
        Seed::from(operation_id),
        Seed::from(bump.as_ref()),
    ]
}

/// Seeds: ["vault", bump]
#[inline]
pub fn vault_authority_seeds<'a>(bump: &'a [u8; 1]) -> [Seed<'a>; 2] {
    [Seed::from(VAULT_SEED), Seed::from(bump.as_ref())]
}
