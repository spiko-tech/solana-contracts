use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

// -------------------------------------------------------------------
// Constants matching the on-chain program (from state.rs / error.rs)
// -------------------------------------------------------------------

const MINTER_CONFIG_SEED: &[u8] = b"minter_config";
const DAILY_LIMIT_SEED: &[u8] = b"daily_limit";
const MINT_OPERATION_SEED: &[u8] = b"mint_op";

const DISCRIMINATOR_MINTER_CONFIG: u8 = 1;
const DISCRIMINATOR_DAILY_LIMIT: u8 = 2;
const DISCRIMINATOR_MINT_OPERATION: u8 = 3;

const STATUS_PENDING: u8 = 1;
const STATUS_DONE: u8 = 2;

const MINTER_CONFIG_LEN: usize = 42;
const DAILY_LIMIT_LEN: usize = 26;
const MINT_OPERATION_LEN: usize = 11;

// Permission manager constants
const PERMISSION_CONFIG_SEED: &[u8] = b"permission_config";
const USER_PERMISSION_SEED: &[u8] = b"user_perm";
const DISCRIMINATOR_PERMISSION_CONFIG: u8 = 1;
const DISCRIMINATOR_USER_PERMISSION: u8 = 2;
const PERMISSION_CONFIG_LEN: usize = 66;
const USER_PERMISSIONS_LEN: usize = 34;

// Role bit constants
const ROLE_MINT_INITIATOR: u8 = 7;
const ROLE_MINT_APPROVER: u8 = 6;

// -------------------------------------------------------------------
// Setup
// -------------------------------------------------------------------

fn setup() -> (Mollusk, Pubkey) {
    let program_id = Pubkey::new_unique();
    std::env::set_var("SBF_OUT_DIR", "../../target/deploy");
    let mollusk = Mollusk::new(&program_id, "minter");
    (mollusk, program_id)
}

// -------------------------------------------------------------------
// PDA helpers
// -------------------------------------------------------------------

fn minter_config_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINTER_CONFIG_SEED], program_id)
}

fn daily_limit_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[DAILY_LIMIT_SEED, mint.as_ref()], program_id)
}

fn mint_operation_pda(operation_id: &[u8; 32], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINT_OPERATION_SEED, operation_id], program_id)
}

fn perm_config_pda(perm_manager_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PERMISSION_CONFIG_SEED], perm_manager_id)
}

// -------------------------------------------------------------------
// Account helpers
// -------------------------------------------------------------------

fn payer_account() -> Account {
    Account::new(10_000_000_000, 0, &Pubkey::default())
}

fn blank_pda_account() -> Account {
    Account::default()
}

/// Build a pre-populated MinterConfig account (for tests that need it already initialized).
fn minter_config_account(
    program_id: &Pubkey,
    bump: u8,
    max_delay: i64,
    permission_manager: &Pubkey,
) -> Account {
    let mut data = vec![0u8; MINTER_CONFIG_LEN];
    data[0] = DISCRIMINATOR_MINTER_CONFIG;
    data[1] = bump;
    data[2..10].copy_from_slice(&max_delay.to_le_bytes());
    data[10..42].copy_from_slice(permission_manager.as_ref());
    Account {
        lamports: 1_000_000,
        data,
        owner: *program_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated PermissionConfig account (owned by permission_manager program).
fn perm_config_account(perm_manager_id: &Pubkey, bump: u8, admin: &Pubkey) -> Account {
    let mut data = vec![0u8; PERMISSION_CONFIG_LEN];
    data[0] = DISCRIMINATOR_PERMISSION_CONFIG;
    data[1] = bump;
    data[2..34].copy_from_slice(admin.as_ref());
    // data[34..66] = pending_admin, zeroed
    Account {
        lamports: 1_000_000,
        data,
        owner: *perm_manager_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

// -------------------------------------------------------------------
// Instruction data builders
// -------------------------------------------------------------------

/// Discriminator 0: initialize
/// Data: [0..8] max_delay (i64 LE) + [8..40] permission_manager (32 bytes)
fn ix_initialize(max_delay: i64, permission_manager: &Pubkey) -> Vec<u8> {
    let mut data = vec![0u8]; // discriminator
    data.extend_from_slice(&max_delay.to_le_bytes());
    data.extend_from_slice(permission_manager.as_ref());
    data
}

// ===================================================================
// TEST: Initialize — happy path
// ===================================================================

#[test]
fn test_initialize() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let (config_pda, _config_bump) = minter_config_pda(&program_id);

    let max_delay: i64 = 86400; // 1 day

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initialize(max_delay, &perm_manager),
        vec![
            AccountMeta::new(admin, true),       // 0: admin (signer, writable)
            AccountMeta::new(config_pda, false), // 1: MinterConfig PDA (writable)
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 2: system program
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_pda, blank_pda_account()),
            keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    // Verify the resulting MinterConfig data
    let config_account = &result.resulting_accounts[1].1;
    let data = &config_account.data;
    assert_eq!(data.len(), MINTER_CONFIG_LEN);
    assert_eq!(data[0], DISCRIMINATOR_MINTER_CONFIG);
    // bump is data[1] — just check it's nonzero
    let stored_max_delay = i64::from_le_bytes(data[2..10].try_into().unwrap());
    assert_eq!(stored_max_delay, max_delay);
    let stored_perm_manager = &data[10..42];
    assert_eq!(stored_perm_manager, perm_manager.as_ref());
}

// ===================================================================
// TEST: Initialize — already initialized (should fail)
// ===================================================================

#[test]
fn test_initialize_already_initialized() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);

    let max_delay: i64 = 86400;

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initialize(max_delay, &perm_manager),
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
        ],
    );

    // Provide an already-initialized config account
    let existing_config = minter_config_account(&program_id, config_bump, 3600, &perm_manager);

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_pda, existing_config),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::Custom(0))], // AlreadyInitialized = 0
    );
}

// -------------------------------------------------------------------
// Additional account helpers
// -------------------------------------------------------------------

fn user_perm_pda(user: &Pubkey, perm_manager_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USER_PERMISSION_SEED, user.as_ref()], perm_manager_id)
}

/// Build a pre-populated UserPermissions account (owned by permission_manager).
fn user_perms_account(perm_manager_id: &Pubkey, bump: u8, roles: &[u8; 32]) -> Account {
    let mut data = vec![0u8; USER_PERMISSIONS_LEN];
    data[0] = DISCRIMINATOR_USER_PERMISSION;
    data[1] = bump;
    data[2..34].copy_from_slice(roles);
    Account {
        lamports: 1_000_000,
        data,
        owner: *perm_manager_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated DailyLimit account.
fn daily_limit_account(
    program_id: &Pubkey,
    bump: u8,
    limit: u64,
    used_amount: u64,
    last_day: i64,
) -> Account {
    let mut data = vec![0u8; DAILY_LIMIT_LEN];
    data[0] = DISCRIMINATOR_DAILY_LIMIT;
    data[1] = bump;
    data[2..10].copy_from_slice(&limit.to_le_bytes());
    data[10..18].copy_from_slice(&used_amount.to_le_bytes());
    data[18..26].copy_from_slice(&last_day.to_le_bytes());
    Account {
        lamports: 1_000_000,
        data,
        owner: *program_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated MintOperation account.
fn mint_operation_account(program_id: &Pubkey, bump: u8, status: u8, deadline: i64) -> Account {
    let mut data = vec![0u8; MINT_OPERATION_LEN];
    data[0] = DISCRIMINATOR_MINT_OPERATION;
    data[1] = bump;
    data[2] = status;
    data[3..11].copy_from_slice(&deadline.to_le_bytes());
    Account {
        lamports: 1_000_000,
        data,
        owner: *program_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Set a role bit in a 32-byte bitmask.
fn set_role_bit(bitmask: &mut [u8; 32], role: u8) {
    let byte_index = (role / 8) as usize;
    let bit_index = role % 8;
    bitmask[byte_index] |= 1 << bit_index;
}

/// Compute operation_id = SHA256(user || mint || amount_le || salt_le) for tests.
fn compute_operation_id(
    user: &[u8; 32],
    token_mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> [u8; 32] {
    let mut input = [0u8; 80];
    input[0..32].copy_from_slice(user);
    input[32..64].copy_from_slice(token_mint);
    input[64..72].copy_from_slice(&amount.to_le_bytes());
    input[72..80].copy_from_slice(&salt.to_le_bytes());
    let result = Sha256::digest(&input);
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

// -------------------------------------------------------------------
// Instruction data builders
// -------------------------------------------------------------------

/// Discriminator 4: set_daily_limit
/// Data: [0..32] token_mint + [32..40] limit (u64 LE)
fn ix_set_daily_limit(token_mint: &Pubkey, limit: u64) -> Vec<u8> {
    let mut data = vec![4u8]; // discriminator
    data.extend_from_slice(token_mint.as_ref());
    data.extend_from_slice(&limit.to_le_bytes());
    data
}

/// Discriminator 5: set_max_delay
/// Data: [0..8] max_delay (i64 LE)
fn ix_set_max_delay(max_delay: i64) -> Vec<u8> {
    let mut data = vec![5u8]; // discriminator
    data.extend_from_slice(&max_delay.to_le_bytes());
    data
}

/// Discriminator 1: initiate_mint
/// Data: [0..32] user + [32..40] amount (u64 LE) + [40..48] salt (u64 LE)
fn ix_initiate_mint(user: &Pubkey, amount: u64, salt: u64) -> Vec<u8> {
    let mut data = vec![1u8]; // discriminator
    data.extend_from_slice(user.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&salt.to_le_bytes());
    data
}

/// Discriminator 3: cancel_mint
/// Data: [0..32] user + [32..64] token_mint + [64..72] amount + [72..80] salt
fn ix_cancel_mint(user: &Pubkey, token_mint: &Pubkey, amount: u64, salt: u64) -> Vec<u8> {
    let mut data = vec![3u8]; // discriminator
    data.extend_from_slice(user.as_ref());
    data.extend_from_slice(token_mint.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&salt.to_le_bytes());
    data
}

// ===================================================================
// TEST: SetDailyLimit — happy path (create new DailyLimit PDA)
// ===================================================================

#[test]
fn test_set_daily_limit() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);
    let (perm_cfg_pda, perm_cfg_bump) = perm_config_pda(&perm_manager);
    let (dl_pda, _dl_bump) = daily_limit_pda(&token_mint, &program_id);

    let limit: u64 = 1_000_000_00000; // 1M tokens at 5 decimals

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_daily_limit(&token_mint, limit),
        vec![
            AccountMeta::new(admin, true), // 0: caller (signer, writable)
            AccountMeta::new_readonly(config_pda, false), // 1: MinterConfig PDA
            AccountMeta::new_readonly(perm_cfg_pda, false), // 2: PermissionConfig PDA
            AccountMeta::new(dl_pda, false), // 3: DailyLimit PDA (writable)
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 4: system program
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                perm_cfg_pda,
                perm_config_account(&perm_manager, perm_cfg_bump, &admin),
            ),
            (dl_pda, blank_pda_account()),
            keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    // Verify the resulting DailyLimit data
    let dl_account = &result.resulting_accounts[3].1;
    let data = &dl_account.data;
    assert_eq!(data.len(), DAILY_LIMIT_LEN);
    assert_eq!(data[0], DISCRIMINATOR_DAILY_LIMIT);
    let stored_limit = u64::from_le_bytes(data[2..10].try_into().unwrap());
    assert_eq!(stored_limit, limit);
    let stored_used = u64::from_le_bytes(data[10..18].try_into().unwrap());
    assert_eq!(stored_used, 0);
    let stored_last_day = i64::from_le_bytes(data[18..26].try_into().unwrap());
    assert_eq!(stored_last_day, 0);
}

// ===================================================================
// TEST: SetDailyLimit — update existing DailyLimit
// ===================================================================

#[test]
fn test_set_daily_limit_update() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);
    let (perm_cfg_pda, perm_cfg_bump) = perm_config_pda(&perm_manager);
    let (dl_pda, dl_bump) = daily_limit_pda(&token_mint, &program_id);

    let old_limit: u64 = 500_000_00000;
    let new_limit: u64 = 2_000_000_00000;

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_daily_limit(&token_mint, new_limit),
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(perm_cfg_pda, false),
            AccountMeta::new(dl_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
        ],
    );

    // Provide an already-initialized DailyLimit with some usage
    let existing_dl = daily_limit_account(&program_id, dl_bump, old_limit, 100_000_00000, 19800);

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                perm_cfg_pda,
                perm_config_account(&perm_manager, perm_cfg_bump, &admin),
            ),
            (dl_pda, existing_dl),
            keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    // Verify the limit was updated but used_amount and last_day preserved
    let dl_account = &result.resulting_accounts[3].1;
    let data = &dl_account.data;
    let stored_limit = u64::from_le_bytes(data[2..10].try_into().unwrap());
    assert_eq!(stored_limit, new_limit);
    let stored_used = u64::from_le_bytes(data[10..18].try_into().unwrap());
    assert_eq!(stored_used, 100_000_00000); // preserved
    let stored_last_day = i64::from_le_bytes(data[18..26].try_into().unwrap());
    assert_eq!(stored_last_day, 19800); // preserved
}

// ===================================================================
// TEST: SetDailyLimit — unauthorized (non-admin caller)
// ===================================================================

#[test]
fn test_set_daily_limit_unauthorized() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let non_admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);
    let (perm_cfg_pda, perm_cfg_bump) = perm_config_pda(&perm_manager);
    let (dl_pda, _dl_bump) = daily_limit_pda(&token_mint, &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_daily_limit(&token_mint, 1_000_000),
        vec![
            AccountMeta::new(non_admin, true), // non-admin caller
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(perm_cfg_pda, false),
            AccountMeta::new(dl_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (non_admin, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                perm_cfg_pda,
                perm_config_account(&perm_manager, perm_cfg_bump, &admin),
            ), // admin is someone else
            (dl_pda, blank_pda_account()),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::Custom(2))], // Unauthorized = 2
    );
}

// ===================================================================
// TEST: SetMaxDelay — happy path
// ===================================================================

#[test]
fn test_set_max_delay() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);
    let (perm_cfg_pda, perm_cfg_bump) = perm_config_pda(&perm_manager);

    let old_max_delay: i64 = 86400;
    let new_max_delay: i64 = 172800; // 2 days

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_max_delay(new_max_delay),
        vec![
            AccountMeta::new_readonly(admin, true), // 0: caller (signer)
            AccountMeta::new(config_pda, false),    // 1: MinterConfig PDA (writable)
            AccountMeta::new_readonly(perm_cfg_pda, false), // 2: PermissionConfig PDA
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, old_max_delay, &perm_manager),
            ),
            (
                perm_cfg_pda,
                perm_config_account(&perm_manager, perm_cfg_bump, &admin),
            ),
        ],
        &[Check::success()],
    );

    // Verify max_delay was updated
    let config_account = &result.resulting_accounts[1].1;
    let data = &config_account.data;
    let stored_max_delay = i64::from_le_bytes(data[2..10].try_into().unwrap());
    assert_eq!(stored_max_delay, new_max_delay);
    // Permission manager should be unchanged
    let stored_perm_manager = &data[10..42];
    assert_eq!(stored_perm_manager, perm_manager.as_ref());
}

// ===================================================================
// TEST: SetMaxDelay — unauthorized (non-admin caller)
// ===================================================================

#[test]
fn test_set_max_delay_unauthorized() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let non_admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);
    let (perm_cfg_pda, perm_cfg_bump) = perm_config_pda(&perm_manager);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_max_delay(172800),
        vec![
            AccountMeta::new_readonly(non_admin, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(perm_cfg_pda, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (non_admin, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                perm_cfg_pda,
                perm_config_account(&perm_manager, perm_cfg_bump, &admin),
            ),
        ],
        &[Check::err(ProgramError::Custom(2))], // Unauthorized = 2
    );
}

// ===================================================================
// TEST: InitiateMint — blocked (over daily limit, creates PENDING op)
// ===================================================================

#[test]
fn test_initiate_mint_blocked() {
    let (mut mollusk, program_id) = setup();
    let caller = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);
    let (dl_pda, dl_bump) = daily_limit_pda(&token_mint, &program_id);

    let max_delay: i64 = 86400;
    let amount: u64 = 2_000_000_00000; // 2M tokens — exceeds 1M limit
    let salt: u64 = 42;

    // Compute operation ID
    let operation_id = compute_operation_id(&user.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, _op_bump) = mint_operation_pda(&operation_id, &program_id);

    // Caller needs ROLE_MINT_INITIATOR
    let (caller_perms_pda, caller_perms_bump) = user_perm_pda(&caller, &perm_manager);
    let mut roles = [0u8; 32];
    set_role_bit(&mut roles, ROLE_MINT_INITIATOR);

    // Set clock timestamp so current_day is deterministic
    let now: i64 = 86400 * 19800; // day 19800
    mollusk.sysvars.clock.unix_timestamp = now;

    // DailyLimit with 1M limit, 0 used
    let dl_account = daily_limit_account(&program_id, dl_bump, 1_000_000_00000, 0, 19800);

    // For the blocked path, we need 14 accounts but many CPI-related ones are unused.
    // The instruction still parses them, so we provide dummy accounts.
    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let recipient_token_account = Pubkey::new_unique();
    let mint_authority = Pubkey::new_unique();
    let minter_user_perms = Pubkey::new_unique();
    let recipient_perms = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();

    let dummy_account = || Account {
        lamports: 0,
        data: vec![],
        owner: Pubkey::default(),
        executable: false,
        rent_epoch: u64::MAX,
    };

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initiate_mint(&user, amount, salt),
        vec![
            AccountMeta::new(caller, true),               // 0: caller (signer)
            AccountMeta::new_readonly(config_pda, false), // 1: MinterConfig
            AccountMeta::new(dl_pda, false),              // 2: DailyLimit (writable)
            AccountMeta::new(op_pda, false),              // 3: MintOperation (writable)
            AccountMeta::new_readonly(caller_perms_pda, false), // 4: caller UserPerms
            AccountMeta::new_readonly(spiko_token_program, false), // 5: spiko-token program
            AccountMeta::new_readonly(token_config, false), // 6: TokenConfig
            AccountMeta::new(token_mint, false),          // 7: token mint (writable for CPI)
            AccountMeta::new(recipient_token_account, false), // 8: recipient token acct
            AccountMeta::new_readonly(mint_authority, false), // 9: mint authority
            AccountMeta::new_readonly(minter_user_perms, false), // 10: minter user perms
            AccountMeta::new_readonly(recipient_perms, false), // 11: recipient perms
            AccountMeta::new_readonly(token_2022_program, false), // 12: token-2022 program
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 13: system program
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, max_delay, &perm_manager),
            ),
            (dl_pda, dl_account),
            (op_pda, blank_pda_account()),
            (
                caller_perms_pda,
                user_perms_account(&perm_manager, caller_perms_bump, &roles),
            ),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (token_mint, dummy_account()),
            (recipient_token_account, dummy_account()),
            (mint_authority, dummy_account()),
            (minter_user_perms, dummy_account()),
            (recipient_perms, dummy_account()),
            (token_2022_program, dummy_account()),
            keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    // Verify MintOperation was created with PENDING status
    let op_account = &result.resulting_accounts[3].1;
    let op_data = &op_account.data;
    assert_eq!(op_data.len(), MINT_OPERATION_LEN);
    assert_eq!(op_data[0], DISCRIMINATOR_MINT_OPERATION);
    assert_eq!(op_data[2], STATUS_PENDING);
    let stored_deadline = i64::from_le_bytes(op_data[3..11].try_into().unwrap());
    assert_eq!(stored_deadline, now + max_delay);

    // Verify daily limit used_amount was NOT updated (over-limit path doesn't increment)
    let dl_result = &result.resulting_accounts[2].1;
    let dl_data = &dl_result.data;
    let stored_used = u64::from_le_bytes(dl_data[10..18].try_into().unwrap());
    assert_eq!(stored_used, 0); // unchanged — mint was blocked
}

// ===================================================================
// TEST: InitiateMint — unauthorized (missing ROLE_MINT_INITIATOR)
// ===================================================================

#[test]
fn test_initiate_mint_unauthorized() {
    let (mut mollusk, program_id) = setup();
    let caller = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);
    let (dl_pda, dl_bump) = daily_limit_pda(&token_mint, &program_id);

    let amount: u64 = 100_00000;
    let salt: u64 = 1;

    let operation_id = compute_operation_id(&user.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, _) = mint_operation_pda(&operation_id, &program_id);

    // Caller has NO roles (empty bitmask)
    let (caller_perms_pda, caller_perms_bump) = user_perm_pda(&caller, &perm_manager);
    let roles = [0u8; 32]; // no roles

    mollusk.sysvars.clock.unix_timestamp = 86400 * 19800;

    let dummy_account = || Account {
        lamports: 0,
        data: vec![],
        owner: Pubkey::default(),
        executable: false,
        rent_epoch: u64::MAX,
    };

    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let recipient_token_account = Pubkey::new_unique();
    let mint_authority = Pubkey::new_unique();
    let minter_user_perms = Pubkey::new_unique();
    let recipient_perms = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initiate_mint(&user, amount, salt),
        vec![
            AccountMeta::new(caller, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(dl_pda, false),
            AccountMeta::new(op_pda, false),
            AccountMeta::new_readonly(caller_perms_pda, false),
            AccountMeta::new_readonly(spiko_token_program, false),
            AccountMeta::new_readonly(token_config, false),
            AccountMeta::new(token_mint, false),
            AccountMeta::new(recipient_token_account, false),
            AccountMeta::new_readonly(mint_authority, false),
            AccountMeta::new_readonly(minter_user_perms, false),
            AccountMeta::new_readonly(recipient_perms, false),
            AccountMeta::new_readonly(token_2022_program, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                dl_pda,
                daily_limit_account(&program_id, dl_bump, 1_000_000_00000, 0, 19800),
            ),
            (op_pda, blank_pda_account()),
            (
                caller_perms_pda,
                user_perms_account(&perm_manager, caller_perms_bump, &roles),
            ),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (token_mint, dummy_account()),
            (recipient_token_account, dummy_account()),
            (mint_authority, dummy_account()),
            (minter_user_perms, dummy_account()),
            (recipient_perms, dummy_account()),
            (token_2022_program, dummy_account()),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::Custom(2))], // Unauthorized = 2
    );
}

// ===================================================================
// TEST: CancelMint — happy path (cancel a PENDING operation)
// ===================================================================

#[test]
fn test_cancel_mint() {
    let (mollusk, program_id) = setup();
    let caller = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);

    let amount: u64 = 2_000_000_00000;
    let salt: u64 = 42;

    let operation_id = compute_operation_id(&user.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, op_bump) = mint_operation_pda(&operation_id, &program_id);

    // Caller needs ROLE_MINT_APPROVER
    let (caller_perms_pda, caller_perms_bump) = user_perm_pda(&caller, &perm_manager);
    let mut roles = [0u8; 32];
    set_role_bit(&mut roles, ROLE_MINT_APPROVER);

    let deadline: i64 = 86400 * 19800 + 86400; // some future deadline

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_cancel_mint(&user, &token_mint, amount, salt),
        vec![
            AccountMeta::new_readonly(caller, true), // 0: caller (signer)
            AccountMeta::new_readonly(config_pda, false), // 1: MinterConfig
            AccountMeta::new(op_pda, false),         // 2: MintOperation (writable)
            AccountMeta::new_readonly(caller_perms_pda, false), // 3: caller UserPerms
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                op_pda,
                mint_operation_account(&program_id, op_bump, STATUS_PENDING, deadline),
            ),
            (
                caller_perms_pda,
                user_perms_account(&perm_manager, caller_perms_bump, &roles),
            ),
        ],
        &[Check::success()],
    );

    // Verify status changed to DONE
    let op_account = &result.resulting_accounts[2].1;
    let op_data = &op_account.data;
    assert_eq!(op_data[2], STATUS_DONE);
}

// ===================================================================
// TEST: CancelMint — not pending (already DONE, should fail)
// ===================================================================

#[test]
fn test_cancel_mint_not_pending() {
    let (mollusk, program_id) = setup();
    let caller = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let (config_pda, config_bump) = minter_config_pda(&program_id);

    let amount: u64 = 2_000_000_00000;
    let salt: u64 = 42;

    let operation_id = compute_operation_id(&user.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, op_bump) = mint_operation_pda(&operation_id, &program_id);

    let (caller_perms_pda, caller_perms_bump) = user_perm_pda(&caller, &perm_manager);
    let mut roles = [0u8; 32];
    set_role_bit(&mut roles, ROLE_MINT_APPROVER);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_cancel_mint(&user, &token_mint, amount, salt),
        vec![
            AccountMeta::new_readonly(caller, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(op_pda, false),
            AccountMeta::new_readonly(caller_perms_pda, false),
        ],
    );

    // Provide a MintOperation that is already DONE
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (
                config_pda,
                minter_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                op_pda,
                mint_operation_account(&program_id, op_bump, STATUS_DONE, 86400 * 19800),
            ),
            (
                caller_perms_pda,
                user_perms_account(&perm_manager, caller_perms_bump, &roles),
            ),
        ],
        &[Check::err(ProgramError::Custom(3))], // NotPending = 3
    );
}
