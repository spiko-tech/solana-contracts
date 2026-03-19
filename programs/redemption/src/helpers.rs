use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Seed, Signer},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};

use permission_manager::state::{
    has_role, PermissionConfig, UserPermissions, DISCRIMINATOR_PERMISSION_CONFIG,
    DISCRIMINATOR_USER_PERMISSION, PERMISSION_CONFIG_SEED,
};

use crate::{
    error::RedemptionError,
    state::{REDEMPTION_CONFIG_SEED, REDEMPTION_OPERATION_SEED, TOKEN_MINIMUM_SEED, VAULT_SEED},
};

// -----------------------------------------------------------------
// PDA verification helpers
// -----------------------------------------------------------------

/// Verify that an account's address matches the expected PDA and return the bump.
#[inline]
pub fn verify_pda(
    account: &AccountView,
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<u8, ProgramError> {
    let (expected, bump) = Address::find_program_address(seeds, program_id);
    if account.address() != &expected {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(bump)
}

// -----------------------------------------------------------------
// PDA account creation
// -----------------------------------------------------------------

/// Create a PDA account via CPI to the system program. The PDA signs.
pub fn create_pda_account(
    payer: &AccountView,
    pda_account: &AccountView,
    space: usize,
    owner: &Address,
    signer_seeds: &[Signer],
) -> ProgramResult {
    pinocchio_system::create_account_with_minimum_balance_signed(
        pda_account,
        space,
        owner,
        payer,
        None,
        signer_seeds,
    )
}

// -----------------------------------------------------------------
// Admin check: direct read of permission_manager's PermissionConfig
// -----------------------------------------------------------------

/// Verify the caller is the admin by reading the permission_manager's
/// PermissionConfig PDA directly.
pub fn require_admin(
    caller: &AccountView,
    perm_config_account: &AccountView,
    permission_manager_id: &Address,
) -> Result<(), ProgramError> {
    // Verify the config account is owned by the permission_manager program
    if !perm_config_account.owned_by(permission_manager_id) {
        return Err(RedemptionError::Unauthorized.into());
    }

    // Verify it's the actual PermissionConfig PDA
    verify_pda(
        perm_config_account,
        &[PERMISSION_CONFIG_SEED],
        permission_manager_id,
    )?;

    let data = perm_config_account.try_borrow()?;
    if data.len() < PermissionConfig::LEN || data[0] != DISCRIMINATOR_PERMISSION_CONFIG {
        return Err(RedemptionError::Unauthorized.into());
    }
    let config = PermissionConfig::from_bytes(&data)?;

    if caller.address() != &config.admin {
        return Err(RedemptionError::Unauthorized.into());
    }

    Ok(())
}

// -----------------------------------------------------------------
// Permission check: direct role check via PDA read
// -----------------------------------------------------------------

/// Read a UserPermissions PDA from the permission_manager and check
/// that the user has the specified role.
pub fn require_permission(
    user_perms_account: &AccountView,
    permission_manager_id: &Address,
    role_bit: u8,
) -> Result<(), ProgramError> {
    // Verify account is owned by the permission_manager program
    if !user_perms_account.owned_by(permission_manager_id) {
        return Err(RedemptionError::Unauthorized.into());
    }

    let user_data = user_perms_account.try_borrow()?;
    if user_data.len() < UserPermissions::LEN || user_data[0] != DISCRIMINATOR_USER_PERMISSION {
        return Err(RedemptionError::Unauthorized.into());
    }
    let user_perms = UserPermissions::from_bytes(&user_data)?;

    if !has_role(&user_perms.roles, role_bit) {
        return Err(RedemptionError::Unauthorized.into());
    }

    Ok(())
}

// -----------------------------------------------------------------
// Operation ID: SHA256(user || token_mint || amount || salt)
// -----------------------------------------------------------------

/// Build the 80-byte hash input:
/// user(32) || token_mint(32) || amount(8 LE) || salt(8 LE)
#[inline]
fn build_hash_input(user: &[u8; 32], token_mint: &[u8; 32], amount: u64, salt: u64) -> [u8; 80] {
    let mut input = [0u8; 80];
    input[0..32].copy_from_slice(user);
    input[32..64].copy_from_slice(token_mint);
    input[64..72].copy_from_slice(&amount.to_le_bytes());
    input[72..80].copy_from_slice(&salt.to_le_bytes());
    input
}

/// Compute operation_id = SHA256(user || mint || amount_le || salt_le)
///
/// On-chain: uses Solana's sol_sha256 syscall.
#[cfg(target_os = "solana")]
pub fn compute_operation_id(
    user: &[u8; 32],
    token_mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> [u8; 32] {
    let input = build_hash_input(user, token_mint, amount, salt);
    let mut hash_result = [0u8; 32];

    #[repr(C)]
    struct Slice {
        ptr: *const u8,
        len: u64,
    }

    let slices = [Slice {
        ptr: input.as_ptr(),
        len: 80,
    }];

    unsafe {
        pinocchio::syscalls::sol_sha256(slices.as_ptr() as *const u8, 1, hash_result.as_mut_ptr());
    }

    hash_result
}

/// Native fallback for tests — uses the `sha2` crate.
#[cfg(not(target_os = "solana"))]
pub fn compute_operation_id(
    user: &[u8; 32],
    token_mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let input = build_hash_input(user, token_mint, amount, salt);
    let result = Sha256::digest(&input);
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

// -----------------------------------------------------------------
// CPI to spiko_token::burn (discriminator 2)
// -----------------------------------------------------------------

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
    amount: u64,
) -> ProgramResult {
    // Build instruction data: discriminator(2) + amount(8 bytes LE)
    let mut ix_data = [0u8; 9];
    ix_data[0] = 2; // discriminator for burn
    ix_data[1..9].copy_from_slice(&amount.to_le_bytes());

    // Build CPI instruction accounts
    let ix_accounts = [
        InstructionAccount::writable_signer(vault_authority.address()), // caller (PDA signer)
        InstructionAccount::readonly(token_config.address()),           // token config
        InstructionAccount::writable(token_mint.address()),             // token-2022 mint
        InstructionAccount::writable(vault_token_account.address()),    // source token account
        InstructionAccount::readonly(mint_authority.address()),         // mint authority PDA
        InstructionAccount::readonly(vault_authority_perms.address()),  // vault auth's user perms
        InstructionAccount::readonly(token_2022_program.address()),     // token-2022 program
    ];

    let instruction = InstructionView {
        program_id: spiko_token_program.address(),
        accounts: &ix_accounts,
        data: &ix_data,
    };

    // Vault authority PDA signs the CPI
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
            spiko_token_program,
        ],
        &[signer],
    )
}

// -----------------------------------------------------------------
// CPI to Token-2022 Transfer (for cancel refund)
// -----------------------------------------------------------------

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
    hook_program: &AccountView,
    amount: u64,
) -> ProgramResult {
    let bump_bytes = [vault_authority_bump];
    let seeds = vault_authority_seeds(&bump_bytes);
    let signer = Signer::from(&seeds);

    // Token-2022 TransferChecked (opcode 12) data:
    //   [0]    = 12 (instruction discriminator)
    //   [1..9] = amount (u64 LE)
    //   [9]    = decimals (u8)
    let mut ix_data = [0u8; 10];
    ix_data[0] = 12; // TransferChecked opcode
    ix_data[1..9].copy_from_slice(&amount.to_le_bytes());
    ix_data[9] = 5; // TOKEN_DECIMALS

    let ix_accounts = [
        // Standard TransferChecked accounts:
        InstructionAccount::writable(vault_token_account.address()), // source (vault)
        InstructionAccount::readonly(token_mint.address()),          // mint
        InstructionAccount::writable(user_token_account.address()),  // destination (user)
        InstructionAccount::readonly_signer(vault_authority.address()), // authority (vault PDA signs)
        // Transfer Hook extra accounts:
        InstructionAccount::readonly(extra_account_meta_list.address()),
        InstructionAccount::readonly(permission_manager_program.address()),
        InstructionAccount::readonly(spiko_token_program.address()),
        InstructionAccount::readonly(token_config.address()),
        InstructionAccount::readonly(vault_authority_perms.address()), // sender perms (vault authority)
        InstructionAccount::readonly(user_perms.address()),            // recipient perms (user)
        InstructionAccount::readonly(hook_program.address()),          // hook program
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
            hook_program,
            token_2022_program,
        ],
        &[signer],
    )
}

// -----------------------------------------------------------------
// Signer seeds builders
// -----------------------------------------------------------------

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
