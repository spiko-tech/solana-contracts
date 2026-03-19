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
    error::MinterError,
    state::{DAILY_LIMIT_SEED, MINTER_CONFIG_SEED, MINT_OPERATION_SEED},
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
// Permission check: direct role check via PDA read
// -----------------------------------------------------------------

/// Read a UserPermissions PDA from the permission_manager and check
/// that the user has the specified role.
///
/// - `user_perms_account`: UserPermissions PDA (owned by permission_manager_id)
/// - `permission_manager_id`: the permission_manager program's address
/// - `role_bit`: the role bit to check (e.g. ROLE_MINT_INITIATOR)
///
/// Returns Ok(()) if authorized, Err(Unauthorized) if not.
pub fn require_permission(
    user_perms_account: &AccountView,
    permission_manager_id: &Address,
    role_bit: u8,
) -> Result<(), ProgramError> {
    // Verify account is owned by the permission_manager program
    if !user_perms_account.owned_by(permission_manager_id) {
        return Err(MinterError::Unauthorized.into());
    }

    let user_data = user_perms_account.try_borrow()?;
    if user_data.len() < UserPermissions::LEN || user_data[0] != DISCRIMINATOR_USER_PERMISSION {
        return Err(MinterError::Unauthorized.into());
    }
    let user_perms = UserPermissions::from_bytes(&user_data)?;

    if !has_role(&user_perms.roles, role_bit) {
        return Err(MinterError::Unauthorized.into());
    }

    Ok(())
}

// -----------------------------------------------------------------
// Admin check: direct read of permission_manager's PermissionConfig
// -----------------------------------------------------------------

/// Verify the caller is the admin by reading the permission_manager's
/// PermissionConfig PDA directly.
///
/// - `caller`: the signer account
/// - `perm_config_account`: the PermissionConfig PDA (owned by permission_manager_id)
/// - `permission_manager_id`: the permission_manager program's address
///
/// Returns Ok(()) if caller == config.admin, Err(Unauthorized) otherwise.
pub fn require_admin(
    caller: &AccountView,
    perm_config_account: &AccountView,
    permission_manager_id: &Address,
) -> Result<(), ProgramError> {
    // Verify the config account is owned by the permission_manager program
    if !perm_config_account.owned_by(permission_manager_id) {
        return Err(MinterError::Unauthorized.into());
    }

    // Verify it's the actual PermissionConfig PDA (not some random account)
    verify_pda(
        perm_config_account,
        &[PERMISSION_CONFIG_SEED],
        permission_manager_id,
    )?;

    let data = perm_config_account.try_borrow()?;
    if data.len() < PermissionConfig::LEN || data[0] != DISCRIMINATOR_PERMISSION_CONFIG {
        return Err(MinterError::Unauthorized.into());
    }
    let config = PermissionConfig::from_bytes(&data)?;

    if caller.address() != &config.admin {
        return Err(MinterError::Unauthorized.into());
    }

    Ok(())
}

// -----------------------------------------------------------------
// Operation ID: SHA256(user || token_mint || amount || salt)
// -----------------------------------------------------------------

/// Build the 80-byte hash input: user(32) || mint(32) || amount(8 LE) || salt(8 LE)
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
/// Native (tests): uses the `sha2` crate (added as dev-dependency).
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
// CPI to spiko_token::mint (discriminator 1)
// -----------------------------------------------------------------

/// Build and invoke CPI to spiko_token program's mint instruction.
///
/// spiko_token mint (discriminator 1) expects accounts:
///   0. `[signer]`   Caller (the minter program — but since CPI, this is the calling account)
///   1. `[]`         TokenConfig PDA (spiko-token)
///   2. `[writable]` Token-2022 Mint
///   3. `[writable]` Recipient's token account
///   4. `[]`         Mint authority PDA (spiko-token)
///   5. `[]`         Caller's UserPermissions PDA (the minter's user-perm with ROLE_MINTER)
///   6. `[]`         Recipient's UserPermissions PDA (whitelist check)
///   7. `[]`         Token-2022 program
///
/// Data: discriminator(1) + amount(u64 LE, 8 bytes) = 9 bytes
///
/// The caller of this CPI must be a signer that has ROLE_MINTER in the permission_manager.
/// In practice, the "caller" account for the CPI is the actual human caller who initiated
/// the minter instruction. But the minter doesn't sign as a PDA for this CPI — the original
/// signer's signature propagates through.
///
/// Wait — actually, the spiko_token mint instruction checks `caller.is_signer()` and verifies
/// ROLE_MINTER for that caller. For the minter program to CPI into spiko_token.mint(), we need
/// a signer that has ROLE_MINTER. This means we need a PDA of the minter program that has
/// been granted ROLE_MINTER in the permission_manager. The MinterConfig PDA could serve this
/// purpose. The minter program would sign as its MinterConfig PDA.
///
/// Account ordering for the CPI:
///   0. MinterConfig PDA [signer via CPI] — must have ROLE_MINTER
///   1. TokenConfig PDA (spiko-token)
///   2. Token-2022 Mint [writable]
///   3. Recipient's token account [writable]
///   4. Mint authority PDA (spiko-token)
///   5. MinterConfig's UserPermissions PDA (proving ROLE_MINTER)
///   6. Recipient's UserPermissions PDA (whitelist)
///   7. Token-2022 program
pub fn cpi_spiko_token_mint<'a>(
    minter_config: &AccountView,
    minter_config_bump: u8,
    token_config: &AccountView,
    token_mint: &AccountView,
    recipient_token_account: &AccountView,
    mint_authority: &AccountView,
    minter_user_perms: &AccountView,
    recipient_perms: &AccountView,
    token_2022_program: &AccountView,
    spiko_token_program: &AccountView,
    amount: u64,
) -> ProgramResult {
    // Build instruction data: discriminator(1) + amount(8 bytes LE)
    let mut ix_data = [0u8; 9];
    ix_data[0] = 1; // discriminator for mint
    ix_data[1..9].copy_from_slice(&amount.to_le_bytes());

    // Build CPI instruction accounts
    let ix_accounts = [
        InstructionAccount::writable_signer(minter_config.address()), // caller (PDA signer)
        InstructionAccount::readonly(token_config.address()),         // token config
        InstructionAccount::writable(token_mint.address()),           // token-2022 mint
        InstructionAccount::writable(recipient_token_account.address()), // recipient ATA
        InstructionAccount::readonly(mint_authority.address()),       // mint authority PDA
        InstructionAccount::readonly(minter_user_perms.address()), // minter's user perms (ROLE_MINTER)
        InstructionAccount::readonly(recipient_perms.address()), // recipient's user perms (whitelist)
        InstructionAccount::readonly(token_2022_program.address()), // token-2022 program
    ];

    let instruction = InstructionView {
        program_id: spiko_token_program.address(),
        accounts: &ix_accounts,
        data: &ix_data,
    };

    // MinterConfig PDA signs the CPI
    let bump_bytes = [minter_config_bump];
    let seeds = minter_config_seeds(&bump_bytes);
    let signer = Signer::from(&seeds);

    pinocchio::cpi::invoke_signed(
        &instruction,
        &[
            minter_config,
            token_config,
            token_mint,
            recipient_token_account,
            mint_authority,
            minter_user_perms,
            recipient_perms,
            token_2022_program,
            spiko_token_program,
        ],
        &[signer],
    )
}

// -----------------------------------------------------------------
// Signer seeds builders
// -----------------------------------------------------------------

/// Build Seed array for the MinterConfig PDA.
/// Seeds: ["minter_config", bump]
#[inline]
pub fn minter_config_seeds<'a>(bump: &'a [u8; 1]) -> [Seed<'a>; 2] {
    [Seed::from(MINTER_CONFIG_SEED), Seed::from(bump.as_ref())]
}

/// Build Seed array for a DailyLimit PDA.
/// Seeds: ["daily_limit", mint_pubkey, bump]
#[inline]
pub fn daily_limit_seeds<'a>(mint_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(DAILY_LIMIT_SEED),
        Seed::from(mint_key),
        Seed::from(bump.as_ref()),
    ]
}

/// Build Seed array for a MintOperation PDA.
/// Seeds: ["mint_op", operation_id, bump]
#[inline]
pub fn mint_operation_seeds<'a>(operation_id: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(MINT_OPERATION_SEED),
        Seed::from(operation_id),
        Seed::from(bump.as_ref()),
    ]
}
