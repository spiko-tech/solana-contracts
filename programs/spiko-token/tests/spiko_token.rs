use mollusk_svm::{
    program::{create_program_account_loader_v3, keyed_account_for_system_program},
    result::Check,
    Mollusk,
};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

// -------------------------------------------------------------------
// Constants matching the on-chain programs
// -------------------------------------------------------------------

// spiko-token constants
const TOKEN_CONFIG_SEED: &[u8] = b"token_config";
const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";
const DISCRIMINATOR_TOKEN_CONFIG: u8 = 1;
const TOKEN_CONFIG_LEN: usize = 100; // 1+1+1+1+32+32+32

// permission-manager constants
const USER_PERMISSION_SEED: &[u8] = b"user_perm";
const DISCRIMINATOR_USER_PERMISSION: u8 = 2;
const PERM_ACCOUNT_LEN: usize = 34; // 1+1+32

const PERMISSION_CONFIG_SEED: &[u8] = b"permission_config";
const DISCRIMINATOR_PERMISSION_CONFIG: u8 = 1;
const PERMISSION_CONFIG_LEN: usize = 66; // 1+1+32+32

// Role bit constants (must match permission_manager::state)
const ROLE_PAUSER: u8 = 1;

// Token-2022 program ID (mainnet: TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb)
const TOKEN_2022_PROGRAM_ID: Pubkey = solana_pubkey::Pubkey::new_from_array([
    6, 221, 246, 225, 238, 117, 143, 222, 24, 66, 93, 188, 228, 108, 205, 218, 182, 26, 252, 77,
    131, 185, 13, 39, 254, 189, 249, 40, 216, 161, 139, 252,
]);

// Token-2022 Mint layout sizes
const MINT_BASE_LEN: usize = 82;
// Token-2022 Mint with extensions (TransferHook + PermanentDelegate):
//   base_mint(82) + padding(83) + account_type(1)
//   + TransferHook TLV: ext_type(2) + ext_len(2) + ext_data(64) = 68
//   + PermanentDelegate TLV: ext_type(2) + ext_len(2) + ext_data(32) = 36
//   Total = 82 + 83 + 1 + 68 + 36 = 270
const MINT_EXTENSIONS_LEN: usize = MINT_BASE_LEN + 83 + 1 + 68 + 36; // 270

// -------------------------------------------------------------------
// Setup
// -------------------------------------------------------------------

/// Setup for tests that do NOT need Token-2022 CPI (pause/unpause).
fn setup() -> (Mollusk, Pubkey) {
    let program_id = Pubkey::new_unique();
    std::env::set_var("SBF_OUT_DIR", "../../target/deploy");
    let mollusk = Mollusk::new(&program_id, "spiko_token");
    (mollusk, program_id)
}

/// Setup for tests that DO need Token-2022 CPI (initialize, mint, burn, transfer, approve).
fn setup_with_token_2022() -> (Mollusk, Pubkey) {
    let program_id = Pubkey::new_unique();
    std::env::set_var("SBF_OUT_DIR", "../../target/deploy");
    let mut mollusk = Mollusk::new(&program_id, "spiko_token");
    mollusk.add_program(&TOKEN_2022_PROGRAM_ID, "spl_token_2022");
    (mollusk, program_id)
}

// -------------------------------------------------------------------
// PDA helpers
// -------------------------------------------------------------------

fn token_config_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOKEN_CONFIG_SEED, mint.as_ref()], program_id)
}

fn mint_authority_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINT_AUTHORITY_SEED, mint.as_ref()], program_id)
}

fn user_perm_pda(user: &Pubkey, perm_manager_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USER_PERMISSION_SEED, user.as_ref()], perm_manager_id)
}

// -------------------------------------------------------------------
// Bitmask helpers
// -------------------------------------------------------------------

fn role_bitmask(role: u8) -> [u8; 32] {
    let mut mask = [0u8; 32];
    let byte_index = (role / 8) as usize;
    let bit_index = role % 8;
    mask[byte_index] |= 1 << bit_index;
    mask
}

// -------------------------------------------------------------------
// Account builders
// -------------------------------------------------------------------

fn payer_account() -> Account {
    Account::new(10_000_000_000, 0, &Pubkey::default())
}

/// Build a pre-initialized TokenConfig account (owned by spiko_token program).
///
/// Layout (100 bytes):
///   [0]       discriminator = 1
///   [1]       bump
///   [2]       paused (0 or 1)
///   [3]       mint_authority_bump
///   [4..36]   permission_manager address
///   [36..68]  spl_mint address
///   [68..100] redemption_contract address (all zeros = not set)
fn token_config_account(
    owner: &Pubkey,
    bump: u8,
    paused: u8,
    mint_auth_bump: u8,
    permission_manager: &Pubkey,
    mint: &Pubkey,
) -> Account {
    let mut data = vec![0u8; TOKEN_CONFIG_LEN];
    data[0] = DISCRIMINATOR_TOKEN_CONFIG;
    data[1] = bump;
    data[2] = paused;
    data[3] = mint_auth_bump;
    data[4..36].copy_from_slice(permission_manager.as_ref());
    data[36..68].copy_from_slice(mint.as_ref());
    // data[68..100] = redemption_contract, zeroed (not set)
    Account {
        lamports: 1_000_000,
        data,
        owner: *owner,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-initialized TokenConfig account with a redemption_contract set.
fn token_config_account_with_redemption(
    owner: &Pubkey,
    bump: u8,
    paused: u8,
    mint_auth_bump: u8,
    permission_manager: &Pubkey,
    mint: &Pubkey,
    redemption_contract: &Pubkey,
) -> Account {
    let mut data = vec![0u8; TOKEN_CONFIG_LEN];
    data[0] = DISCRIMINATOR_TOKEN_CONFIG;
    data[1] = bump;
    data[2] = paused;
    data[3] = mint_auth_bump;
    data[4..36].copy_from_slice(permission_manager.as_ref());
    data[36..68].copy_from_slice(mint.as_ref());
    data[68..100].copy_from_slice(redemption_contract.as_ref());
    Account {
        lamports: 1_000_000,
        data,
        owner: *owner,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a UserPermissions account (owned by permission_manager program).
///
/// Layout (34 bytes):
///   [0]       discriminator = 2
///   [1]       bump
///   [2..34]   roles bitmask
fn user_perms_account(perm_manager_id: &Pubkey, bump: u8, roles: &[u8; 32]) -> Account {
    let mut data = vec![0u8; PERM_ACCOUNT_LEN];
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

// -------------------------------------------------------------------
// Instruction data builders
// -------------------------------------------------------------------

fn ix_pause() -> Vec<u8> {
    vec![5] // discriminator only
}

fn ix_unpause() -> Vec<u8> {
    vec![6] // discriminator only
}

fn ix_initialize() -> Vec<u8> {
    // discriminator(0) only
    vec![0u8]
}

/// Pre-allocated Token-2022 mint account (uninitialized, owned by Token-2022 program).
/// Must have enough space for base mint + TransferHook + PermanentDelegate extensions.
fn uninitialized_mint_account() -> Account {
    let space = MINT_EXTENSIONS_LEN;
    Account {
        lamports: 10_000_000, // enough for rent-exempt even after realloc
        data: vec![0u8; space],
        owner: TOKEN_2022_PROGRAM_ID,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

fn blank_pda_account() -> Account {
    Account::default()
}

// ===================================================================
// TESTS: Pause
// ===================================================================

#[test]
fn test_pause_success() {
    let (mollusk, program_id) = setup();

    let perm_manager_id = Pubkey::new_unique();
    let caller = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    // Derive PDAs
    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (caller_perms_key, caller_perms_bump) = user_perm_pda(&caller, &perm_manager_id);

    // Caller has PAUSER role (bit 1)
    let caller_roles = role_bitmask(ROLE_PAUSER);
    let caller_perms = user_perms_account(&perm_manager_id, caller_perms_bump, &caller_roles);

    // TokenConfig: not paused
    let config = token_config_account(&program_id, config_bump, 0, 0, &perm_manager_id, &mint);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_pause(),
        vec![
            AccountMeta::new(caller, true),      // 0: caller [signer]
            AccountMeta::new(config_key, false), // 1: config [writable]
            AccountMeta::new_readonly(caller_perms_key, false), // 2: caller perms
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (config_key, config),
            (caller_perms_key, caller_perms),
        ],
        &[Check::success()],
    );

    // Verify config.paused is now 1
    let config_data = &result.resulting_accounts[1].1.data;
    assert_eq!(config_data[2], 1, "paused should be 1 after pause");
}

#[test]
fn test_pause_unauthorized_fails() {
    let (mollusk, program_id) = setup();

    let perm_manager_id = Pubkey::new_unique();
    let caller = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (caller_perms_key, caller_perms_bump) = user_perm_pda(&caller, &perm_manager_id);

    // Caller has WHITELISTED role (bit 4), NOT PAUSER (bit 1)
    let caller_roles = role_bitmask(4); // ROLE_WHITELISTED
    let caller_perms = user_perms_account(&perm_manager_id, caller_perms_bump, &caller_roles);

    let config = token_config_account(&program_id, config_bump, 0, 0, &perm_manager_id, &mint);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_pause(),
        vec![
            AccountMeta::new(caller, true),
            AccountMeta::new(config_key, false),
            AccountMeta::new_readonly(caller_perms_key, false),
        ],
    );

    // Unauthorized = custom error 3 (TokenError::Unauthorized)
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (config_key, config),
            (caller_perms_key, caller_perms),
        ],
        &[Check::err(ProgramError::Custom(3))],
    );
}

#[test]
fn test_pause_not_initialized_fails() {
    let (mollusk, program_id) = setup();

    let perm_manager_id = Pubkey::new_unique();
    let caller = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let (config_key, _) = token_config_pda(&mint, &program_id);
    let (caller_perms_key, caller_perms_bump) = user_perm_pda(&caller, &perm_manager_id);

    let caller_roles = role_bitmask(ROLE_PAUSER);
    let caller_perms = user_perms_account(&perm_manager_id, caller_perms_bump, &caller_roles);

    // Config not initialized (system-owned = not owned by program)
    let config = Account::default();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_pause(),
        vec![
            AccountMeta::new(caller, true),
            AccountMeta::new(config_key, false),
            AccountMeta::new_readonly(caller_perms_key, false),
        ],
    );

    // NotInitialized = custom error 5
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (config_key, config),
            (caller_perms_key, caller_perms),
        ],
        &[Check::err(ProgramError::Custom(5))],
    );
}

#[test]
fn test_pause_missing_signer_fails() {
    let (mollusk, program_id) = setup();

    let perm_manager_id = Pubkey::new_unique();
    let caller = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (caller_perms_key, caller_perms_bump) = user_perm_pda(&caller, &perm_manager_id);

    let caller_roles = role_bitmask(ROLE_PAUSER);
    let caller_perms = user_perms_account(&perm_manager_id, caller_perms_bump, &caller_roles);
    let config = token_config_account(&program_id, config_bump, 0, 0, &perm_manager_id, &mint);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_pause(),
        vec![
            AccountMeta::new(caller, false), // NOT a signer
            AccountMeta::new(config_key, false),
            AccountMeta::new_readonly(caller_perms_key, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (config_key, config),
            (caller_perms_key, caller_perms),
        ],
        &[Check::err(ProgramError::MissingRequiredSignature)],
    );
}

// ===================================================================
// TESTS: Unpause
// ===================================================================

#[test]
fn test_unpause_success() {
    let (mollusk, program_id) = setup();

    let perm_manager_id = Pubkey::new_unique();
    let caller = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (caller_perms_key, caller_perms_bump) = user_perm_pda(&caller, &perm_manager_id);

    // Caller has PAUSER role (bit 1)
    let caller_roles = role_bitmask(ROLE_PAUSER);
    let caller_perms = user_perms_account(&perm_manager_id, caller_perms_bump, &caller_roles);

    // TokenConfig: currently paused
    let config = token_config_account(&program_id, config_bump, 1, 0, &perm_manager_id, &mint);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_unpause(),
        vec![
            AccountMeta::new(caller, true),
            AccountMeta::new(config_key, false),
            AccountMeta::new_readonly(caller_perms_key, false),
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (config_key, config),
            (caller_perms_key, caller_perms),
        ],
        &[Check::success()],
    );

    // Verify config.paused is now 0
    let config_data = &result.resulting_accounts[1].1.data;
    assert_eq!(config_data[2], 0, "paused should be 0 after unpause");
}

#[test]
fn test_unpause_unauthorized_fails() {
    let (mollusk, program_id) = setup();

    let perm_manager_id = Pubkey::new_unique();
    let caller = Pubkey::new_unique();
    let mint = Pubkey::new_unique();

    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (caller_perms_key, caller_perms_bump) = user_perm_pda(&caller, &perm_manager_id);

    // Caller has MINTER role (bit 0), NOT PAUSER (bit 1)
    let caller_roles = role_bitmask(0); // ROLE_MINTER
    let caller_perms = user_perms_account(&perm_manager_id, caller_perms_bump, &caller_roles);

    let config = token_config_account(&program_id, config_bump, 1, 0, &perm_manager_id, &mint);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_unpause(),
        vec![
            AccountMeta::new(caller, true),
            AccountMeta::new(config_key, false),
            AccountMeta::new_readonly(caller_perms_key, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (config_key, config),
            (caller_perms_key, caller_perms),
        ],
        &[Check::err(ProgramError::Custom(3))],
    );
}

// ===================================================================
// TESTS: Initialize
// ===================================================================

#[test]
fn test_initialize_success() {
    let (mollusk, program_id) = setup_with_token_2022();

    let admin = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let perm_manager_id = Pubkey::new_unique();
    let transfer_hook_program = Pubkey::new_unique();

    // Derive PDAs
    let (config_key, _) = token_config_pda(&mint, &program_id);
    let (mint_authority_key, _) = mint_authority_pda(&mint, &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initialize(),
        vec![
            AccountMeta::new(admin, true),                           // 0
            AccountMeta::new(config_key, false),                     // 1
            AccountMeta::new(mint, false),                           // 2
            AccountMeta::new_readonly(mint_authority_key, false),    // 3
            AccountMeta::new_readonly(perm_manager_id, false),       // 4
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false), // 5
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 6
            AccountMeta::new_readonly(transfer_hook_program, false), // 7: Transfer Hook program
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_key, blank_pda_account()),
            (mint, uninitialized_mint_account()),
            (mint_authority_key, blank_pda_account()),
            (perm_manager_id, Account::default()),
            (
                TOKEN_2022_PROGRAM_ID,
                create_program_account_loader_v3(&TOKEN_2022_PROGRAM_ID),
            ),
            keyed_account_for_system_program(),
            (transfer_hook_program, Account::default()),
        ],
        &[Check::success()],
    );

    // Verify TokenConfig PDA was created
    let config_data = &result.resulting_accounts[1].1.data;
    assert_eq!(config_data.len(), TOKEN_CONFIG_LEN);
    assert_eq!(config_data[0], DISCRIMINATOR_TOKEN_CONFIG, "discriminator");
    assert_eq!(config_data[2], 0, "paused should be 0");
    // permission_manager at offset [4..36]
    assert_eq!(&config_data[4..36], perm_manager_id.as_ref());
    // spl_mint at offset [36..68]
    assert_eq!(&config_data[36..68], mint.as_ref());
    // redemption_contract at offset [68..100] should be all zeros (not set)
    assert_eq!(&config_data[68..100], &[0u8; 32]);
}

#[test]
fn test_initialize_double_init_fails() {
    let (mollusk, program_id) = setup_with_token_2022();

    let admin = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let perm_manager_id = Pubkey::new_unique();
    let transfer_hook_program = Pubkey::new_unique();

    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (mint_authority_key, _) = mint_authority_pda(&mint, &program_id);

    // Config already exists (owned by program)
    let existing_config =
        token_config_account(&program_id, config_bump, 0, 0, &perm_manager_id, &mint);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initialize(),
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new(config_key, false),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(mint_authority_key, false),
            AccountMeta::new_readonly(perm_manager_id, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(transfer_hook_program, false), // 7: Transfer Hook program
        ],
    );

    // AlreadyInitialized = custom error 4
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_key, existing_config),
            (mint, uninitialized_mint_account()),
            (mint_authority_key, blank_pda_account()),
            (perm_manager_id, Account::default()),
            (
                TOKEN_2022_PROGRAM_ID,
                create_program_account_loader_v3(&TOKEN_2022_PROGRAM_ID),
            ),
            keyed_account_for_system_program(),
            (transfer_hook_program, Account::default()),
        ],
        &[Check::err(ProgramError::Custom(4))],
    );
}

#[test]
fn test_initialize_missing_signer_fails() {
    let (mollusk, program_id) = setup_with_token_2022();

    let admin = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let perm_manager_id = Pubkey::new_unique();
    let transfer_hook_program = Pubkey::new_unique();

    let (config_key, _) = token_config_pda(&mint, &program_id);
    let (mint_authority_key, _) = mint_authority_pda(&mint, &program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initialize(),
        vec![
            AccountMeta::new(admin, false), // NOT a signer
            AccountMeta::new(config_key, false),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(mint_authority_key, false),
            AccountMeta::new_readonly(perm_manager_id, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(transfer_hook_program, false), // 7: Transfer Hook program
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_key, blank_pda_account()),
            (mint, uninitialized_mint_account()),
            (mint_authority_key, blank_pda_account()),
            (perm_manager_id, Account::default()),
            (
                TOKEN_2022_PROGRAM_ID,
                create_program_account_loader_v3(&TOKEN_2022_PROGRAM_ID),
            ),
            keyed_account_for_system_program(),
            (transfer_hook_program, Account::default()),
        ],
        &[Check::err(ProgramError::MissingRequiredSignature)],
    );
}

// ===================================================================
// Helper: PermissionConfig PDA + account builder
// ===================================================================

fn perm_config_pda(perm_manager_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PERMISSION_CONFIG_SEED], perm_manager_id)
}

/// Build a pre-populated PermissionConfig account (owned by permission_manager).
/// Layout (66 bytes): disc(1) + bump(1) + admin(32) + pending_admin(32)
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

// ===================================================================
// Instruction data builder: set_redemption_contract
// ===================================================================

/// Discriminator 8: set_redemption_contract
/// Data: [0..32] redemption_contract address
fn ix_set_redemption_contract(redemption_contract: &Pubkey) -> Vec<u8> {
    let mut data = vec![8u8]; // discriminator
    data.extend_from_slice(redemption_contract.as_ref());
    data
}

// ===================================================================
// TESTS: SetRedemptionContract
// ===================================================================

#[test]
fn test_set_redemption_contract_success() {
    let (mollusk, program_id) = setup();

    let admin = Pubkey::new_unique();
    let perm_manager_id = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let redemption_program = Pubkey::new_unique();

    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (perm_cfg_key, perm_cfg_bump) = perm_config_pda(&perm_manager_id);

    let config = token_config_account(&program_id, config_bump, 0, 0, &perm_manager_id, &mint);
    let perm_cfg = perm_config_account(&perm_manager_id, perm_cfg_bump, &admin);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_redemption_contract(&redemption_program),
        vec![
            AccountMeta::new(admin, true),       // 0: caller (admin, signer)
            AccountMeta::new(config_key, false), // 1: TokenConfig (writable)
            AccountMeta::new_readonly(perm_cfg_key, false), // 2: PermissionConfig
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_key, config),
            (perm_cfg_key, perm_cfg),
        ],
        &[Check::success()],
    );

    // Verify redemption_contract was set at offset [68..100]
    let config_data = &result.resulting_accounts[1].1.data;
    assert_eq!(&config_data[68..100], redemption_program.as_ref());
}

#[test]
fn test_set_redemption_contract_unauthorized() {
    let (mollusk, program_id) = setup();

    let admin = Pubkey::new_unique();
    let non_admin = Pubkey::new_unique();
    let perm_manager_id = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let redemption_program = Pubkey::new_unique();

    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (perm_cfg_key, perm_cfg_bump) = perm_config_pda(&perm_manager_id);

    let config = token_config_account(&program_id, config_bump, 0, 0, &perm_manager_id, &mint);
    let perm_cfg = perm_config_account(&perm_manager_id, perm_cfg_bump, &admin); // admin is someone else

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_redemption_contract(&redemption_program),
        vec![
            AccountMeta::new(non_admin, true), // non-admin caller
            AccountMeta::new(config_key, false),
            AccountMeta::new_readonly(perm_cfg_key, false),
        ],
    );

    // Unauthorized = custom error 3 (TokenError::Unauthorized)
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (non_admin, payer_account()),
            (config_key, config),
            (perm_cfg_key, perm_cfg),
        ],
        &[Check::err(ProgramError::Custom(3))],
    );
}

#[test]
fn test_set_redemption_contract_clear() {
    let (mollusk, program_id) = setup();

    let admin = Pubkey::new_unique();
    let perm_manager_id = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let redemption_program = Pubkey::new_unique();
    let zero_addr = Pubkey::new_from_array([0u8; 32]);

    let (config_key, config_bump) = token_config_pda(&mint, &program_id);
    let (perm_cfg_key, perm_cfg_bump) = perm_config_pda(&perm_manager_id);

    // Start with redemption_contract already set
    let config = token_config_account_with_redemption(
        &program_id,
        config_bump,
        0,
        0,
        &perm_manager_id,
        &mint,
        &redemption_program,
    );
    let perm_cfg = perm_config_account(&perm_manager_id, perm_cfg_bump, &admin);

    // Clear it by setting to zeros
    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_redemption_contract(&zero_addr),
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new(config_key, false),
            AccountMeta::new_readonly(perm_cfg_key, false),
        ],
    );

    let result = mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_key, config),
            (perm_cfg_key, perm_cfg),
        ],
        &[Check::success()],
    );

    // Verify redemption_contract was cleared
    let config_data = &result.resulting_accounts[1].1.data;
    assert_eq!(&config_data[68..100], &[0u8; 32]);
}
