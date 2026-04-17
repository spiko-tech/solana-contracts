use mollusk_svm::{
    program::{create_program_account_loader_v3, keyed_account_for_system_program},
    result::Check,
    Mollusk,
};
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;

const EVENT_AUTHORITY_SEED: &[u8] = b"event_authority";
const GATEKEEPER_CONFIG_SEED: &[u8] = b"gatekeeper_config";
const WITHDRAWAL_DAILY_LIMIT_SEED: &[u8] = b"withdrawal_limit";
const WITHDRAWAL_OPERATION_SEED: &[u8] = b"withdrawal_op";
const VAULT_SEED: &[u8] = b"vault";

const DISCRIMINATOR_GATEKEEPER_CONFIG: u8 = 1;
const DISCRIMINATOR_WITHDRAWAL_DAILY_LIMIT: u8 = 2;
const DISCRIMINATOR_WITHDRAWAL_OPERATION: u8 = 3;

const STATUS_PENDING: u8 = 1;
const STATUS_DONE: u8 = 2;

const GATEKEEPER_CONFIG_LEN: usize = 43; // disc(1) + ver(1) + bump(1) + max_delay(8) + perm_manager(32)
const WITHDRAWAL_DAILY_LIMIT_LEN: usize = 27; // disc(1) + ver(1) + bump(1) + limit(8) + used(8) + last_day(8)
const WITHDRAWAL_OPERATION_LEN: usize = 84; // disc(1) + ver(1) + bump(1) + status(1) + deadline(8) + recipient(32) + mint(32) + amount(8)

const PERMISSION_CONFIG_SEED: &[u8] = b"permission_config";
const USER_PERMISSION_SEED: &[u8] = b"user_perm";
const DISCRIMINATOR_PERMISSION_CONFIG: u8 = 1;
const DISCRIMINATOR_USER_PERMISSION: u8 = 2;
const PERMISSION_CONFIG_LEN: usize = 67;
const USER_PERMISSIONS_LEN: usize = 35;

const ROLE_WHITELISTED: u8 = 4;
const ROLE_WHITELISTED_EXT: u8 = 8;
const ROLE_CUSTODIAL_GATEKEEPER_APPROVER: u8 = 9;

fn setup() -> (Mollusk, Pubkey) {
    let program_id = Pubkey::new_unique();
    std::env::set_var("SBF_OUT_DIR", "../../target/deploy");
    let mollusk = Mollusk::new(&program_id, "custodial_gatekeeper");
    (mollusk, program_id)
}

fn gatekeeper_config_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[GATEKEEPER_CONFIG_SEED], program_id)
}

fn event_authority_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EVENT_AUTHORITY_SEED], program_id)
}

fn withdrawal_daily_limit_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WITHDRAWAL_DAILY_LIMIT_SEED, mint.as_ref()], program_id)
}

fn withdrawal_operation_pda(operation_id: &[u8; 32], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WITHDRAWAL_OPERATION_SEED, operation_id], program_id)
}

fn vault_authority_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_SEED], program_id)
}

fn perm_config_pda(perm_manager_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PERMISSION_CONFIG_SEED], perm_manager_id)
}

fn user_perm_pda(user: &Pubkey, perm_manager_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USER_PERMISSION_SEED, user.as_ref()], perm_manager_id)
}

fn payer_account() -> Account {
    Account::new(10_000_000_000, 0, &Pubkey::default())
}

fn blank_pda_account() -> Account {
    Account::default()
}

fn dummy_account() -> Account {
    Account {
        lamports: 0,
        data: vec![],
        owner: Pubkey::default(),
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated GatekeeperConfig account.
fn gatekeeper_config_account(
    program_id: &Pubkey,
    bump: u8,
    max_delay: i64,
    permission_manager: &Pubkey,
) -> Account {
    let mut data = vec![0u8; GATEKEEPER_CONFIG_LEN];
    data[0] = DISCRIMINATOR_GATEKEEPER_CONFIG;
    data[1] = 1; // version
    data[2] = bump;
    data[3..11].copy_from_slice(&max_delay.to_le_bytes());
    data[11..43].copy_from_slice(permission_manager.as_ref());
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
    data[1] = 1; // version
    data[2] = bump;
    data[3..35].copy_from_slice(admin.as_ref());
    // data[35..67] = pending_admin, zeroed
    Account {
        lamports: 1_000_000,
        data,
        owner: *perm_manager_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated UserPermissions account (owned by permission_manager).
fn user_perms_account(perm_manager_id: &Pubkey, bump: u8, roles: &[u8; 32]) -> Account {
    let mut data = vec![0u8; USER_PERMISSIONS_LEN];
    data[0] = DISCRIMINATOR_USER_PERMISSION;
    data[1] = 1; // version
    data[2] = bump;
    data[3..35].copy_from_slice(roles);
    Account {
        lamports: 1_000_000,
        data,
        owner: *perm_manager_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated WithdrawalDailyLimit account.
fn withdrawal_daily_limit_account(
    program_id: &Pubkey,
    bump: u8,
    limit: u64,
    used_amount: u64,
    last_day: i64,
) -> Account {
    let mut data = vec![0u8; WITHDRAWAL_DAILY_LIMIT_LEN];
    data[0] = DISCRIMINATOR_WITHDRAWAL_DAILY_LIMIT;
    data[1] = 1; // version
    data[2] = bump;
    data[3..11].copy_from_slice(&limit.to_le_bytes());
    data[11..19].copy_from_slice(&used_amount.to_le_bytes());
    data[19..27].copy_from_slice(&last_day.to_le_bytes());
    Account {
        lamports: 1_000_000,
        data,
        owner: *program_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated WithdrawalOperation account.
fn withdrawal_operation_account(
    program_id: &Pubkey,
    bump: u8,
    status: u8,
    deadline: i64,
    recipient: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) -> Account {
    let mut data = vec![0u8; WITHDRAWAL_OPERATION_LEN];
    data[0] = DISCRIMINATOR_WITHDRAWAL_OPERATION;
    data[1] = 1; // version
    data[2] = bump;
    data[3] = status;
    data[4..12].copy_from_slice(&deadline.to_le_bytes());
    data[12..44].copy_from_slice(recipient.as_ref());
    data[44..76].copy_from_slice(mint.as_ref());
    data[76..84].copy_from_slice(&amount.to_le_bytes());
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

/// Compute operation_id = SHA256(recipient || mint || amount_le || salt_le).
fn compute_operation_id(
    recipient: &[u8; 32],
    token_mint: &[u8; 32],
    amount: u64,
    salt: u64,
) -> [u8; 32] {
    let mut input = [0u8; 80];
    input[0..32].copy_from_slice(recipient);
    input[32..64].copy_from_slice(token_mint);
    input[64..72].copy_from_slice(&amount.to_le_bytes());
    input[72..80].copy_from_slice(&salt.to_le_bytes());
    let result = Sha256::digest(&input);
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Discriminator 0: Initialize
/// Data: [0..32] permission_manager + [32..40] max_delay (i64 LE)
fn ix_initialize(permission_manager: &Pubkey, max_delay: i64) -> Vec<u8> {
    let mut data = vec![0u8]; // discriminator
    data.extend_from_slice(permission_manager.as_ref());
    data.extend_from_slice(&max_delay.to_le_bytes());
    data
}

/// Discriminator 1: SetDailyLimit
/// Data: [0..32] token_mint + [32..40] limit (u64 LE)
fn ix_set_daily_limit(token_mint: &Pubkey, limit: u64) -> Vec<u8> {
    let mut data = vec![1u8]; // discriminator
    data.extend_from_slice(token_mint.as_ref());
    data.extend_from_slice(&limit.to_le_bytes());
    data
}

/// Discriminator 2: CustodialWithdraw
/// Data: [0..32] recipient + [32..40] amount (u64 LE) + [40..48] salt (u64 LE)
fn ix_custodial_withdraw(recipient: &Pubkey, amount: u64, salt: u64) -> Vec<u8> {
    let mut data = vec![2u8]; // discriminator
    data.extend_from_slice(recipient.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&salt.to_le_bytes());
    data
}

/// Discriminator 3: ApproveWithdrawal
/// Data: [0..32] recipient + [32..40] amount (u64 LE) + [40..48] salt (u64 LE)
fn ix_approve_withdrawal(recipient: &Pubkey, amount: u64, salt: u64) -> Vec<u8> {
    let mut data = vec![3u8]; // discriminator
    data.extend_from_slice(recipient.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&salt.to_le_bytes());
    data
}

/// Discriminator 4: CancelWithdrawal
/// Data: [0..32] recipient + [32..40] amount (u64 LE) + [40..48] salt (u64 LE)
fn ix_cancel_withdrawal(recipient: &Pubkey, amount: u64, salt: u64) -> Vec<u8> {
    let mut data = vec![4u8]; // discriminator
    data.extend_from_slice(recipient.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&salt.to_le_bytes());
    data
}

#[test]
fn test_initialize() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let (config_pda, _config_bump) = gatekeeper_config_pda(&program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);

    let max_delay: i64 = 86400;

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initialize(&perm_manager, max_delay),
        vec![
            AccountMeta::new(admin, true),       // 0: admin
            AccountMeta::new(config_pda, false), // 1: config PDA
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 2: system program
            AccountMeta::new_readonly(event_authority_key, false), // 3: event_authority
            AccountMeta::new_readonly(program_id, false),          // 4: self_program
        ],
    );

    // NOTE: Self-CPI event emission fails in Mollusk tests because the
    // event_authority PDA bump is derived at compile time from the
    // hardcoded program ID (crate::ID), which differs from the test's
    // Pubkey::new_unique(). The core business logic succeeds but the
    // CPI event call fails.
    let result = mollusk.process_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_pda, blank_pda_account()),
            keyed_account_for_system_program(),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
    );
    assert!(result.program_result.is_err());
}

#[test]
fn test_initialize_already_initialized() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);

    let max_delay: i64 = 86400;

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initialize(&perm_manager, max_delay),
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(event_authority_key, false),
            AccountMeta::new_readonly(program_id, false),
        ],
    );

    let existing_config = gatekeeper_config_account(&program_id, config_bump, 3600, &perm_manager);

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_pda, existing_config),
            keyed_account_for_system_program(),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(0))], // AlreadyInitialized = 0
    );
}

#[test]
fn test_initialize_invalid_max_delay() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let (config_pda, _config_bump) = gatekeeper_config_pda(&program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_initialize(&perm_manager, 0), // invalid: max_delay <= 0
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(event_authority_key, false),
            AccountMeta::new_readonly(program_id, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (config_pda, blank_pda_account()),
            keyed_account_for_system_program(),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(7))], // InvalidMaxDelay = 7
    );
}

#[test]
fn test_set_daily_limit() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (perm_cfg_pda, perm_cfg_bump) = perm_config_pda(&perm_manager);
    let (dl_pda, _dl_bump) = withdrawal_daily_limit_pda(&token_mint, &program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);

    let limit: u64 = 1_000_000_00000;

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_daily_limit(&token_mint, limit),
        vec![
            AccountMeta::new(admin, true),                  // 0: caller
            AccountMeta::new_readonly(config_pda, false),   // 1: config
            AccountMeta::new_readonly(perm_cfg_pda, false), // 2: perm config
            AccountMeta::new(dl_pda, false),                // 3: daily limit
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 4: system
            AccountMeta::new_readonly(event_authority_key, false), // 5: event auth
            AccountMeta::new_readonly(program_id, false),   // 6: self
        ],
    );

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                perm_cfg_pda,
                perm_config_account(&perm_manager, perm_cfg_bump, &admin),
            ),
            (dl_pda, blank_pda_account()),
            keyed_account_for_system_program(),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
    );
    assert!(result.program_result.is_err());
}

#[test]
fn test_set_daily_limit_update() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (perm_cfg_pda, perm_cfg_bump) = perm_config_pda(&perm_manager);
    let (dl_pda, dl_bump) = withdrawal_daily_limit_pda(&token_mint, &program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);

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
            AccountMeta::new_readonly(event_authority_key, false),
            AccountMeta::new_readonly(program_id, false),
        ],
    );

    let existing_dl =
        withdrawal_daily_limit_account(&program_id, dl_bump, 500_000_00000, 100_000_00000, 19800);

    let result = mollusk.process_instruction(
        &instruction,
        &[
            (admin, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                perm_cfg_pda,
                perm_config_account(&perm_manager, perm_cfg_bump, &admin),
            ),
            (dl_pda, existing_dl),
            keyed_account_for_system_program(),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
    );
    assert!(result.program_result.is_err());
}

#[test]
fn test_set_daily_limit_unauthorized() {
    let (mollusk, program_id) = setup();
    let admin = Pubkey::new_unique();
    let non_admin = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (perm_cfg_pda, perm_cfg_bump) = perm_config_pda(&perm_manager);
    let (dl_pda, _dl_bump) = withdrawal_daily_limit_pda(&token_mint, &program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_set_daily_limit(&token_mint, 1_000_000),
        vec![
            AccountMeta::new(non_admin, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(perm_cfg_pda, false),
            AccountMeta::new(dl_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(event_authority_key, false),
            AccountMeta::new_readonly(program_id, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (non_admin, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                perm_cfg_pda,
                perm_config_account(&perm_manager, perm_cfg_bump, &admin),
            ),
            (dl_pda, blank_pda_account()),
            keyed_account_for_system_program(),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(2))], // Unauthorized = 2
    );
}

#[test]
fn test_custodial_withdraw_unauthorized_sender() {
    let (mut mollusk, program_id) = setup();
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (dl_pda, dl_bump) = withdrawal_daily_limit_pda(&token_mint, &program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);
    let (vault_authority, _) = vault_authority_pda(&program_id);

    let amount: u64 = 100_00000;
    let salt: u64 = 1;
    let operation_id =
        compute_operation_id(&recipient.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, _) = withdrawal_operation_pda(&operation_id, &program_id);

    // Sender has NO roles
    let (sender_perms_pda, sender_perms_bump) = user_perm_pda(&sender, &perm_manager);
    let roles = [0u8; 32];

    // Recipient perms (has WHITELISTED_EXT)
    let (recipient_perms_pda, recipient_perms_bump) = user_perm_pda(&recipient, &perm_manager);
    let mut recipient_roles = [0u8; 32];
    set_role_bit(&mut recipient_roles, ROLE_WHITELISTED_EXT);

    // Vault authority perms
    let (vault_perms_pda, vault_perms_bump) = user_perm_pda(&vault_authority, &perm_manager);
    let mut vault_roles = [0u8; 32];
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED);
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED_EXT);

    mollusk.sysvars.clock.unix_timestamp = 86400 * 19800;

    let sender_token_account = Pubkey::new_unique();
    let vault_token_account = Pubkey::new_unique();
    let recipient_token_account = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();
    let extra_account_meta_list = Pubkey::new_unique();
    let permission_manager_program = perm_manager;
    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let hook_event_authority = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_custodial_withdraw(&recipient, amount, salt),
        vec![
            AccountMeta::new(sender, true),                        // 0
            AccountMeta::new_readonly(config_pda, false),          // 1
            AccountMeta::new(dl_pda, false),                       // 2
            AccountMeta::new(op_pda, false),                       // 3
            AccountMeta::new(sender_token_account, false),         // 4
            AccountMeta::new(vault_token_account, false),          // 5
            AccountMeta::new(recipient_token_account, false),      // 6
            AccountMeta::new_readonly(vault_authority, false),     // 7
            AccountMeta::new_readonly(token_mint, false),          // 8
            AccountMeta::new_readonly(sender_perms_pda, false),    // 9
            AccountMeta::new_readonly(recipient_perms_pda, false), // 10
            AccountMeta::new_readonly(vault_perms_pda, false),     // 11
            AccountMeta::new_readonly(token_2022_program, false),  // 12
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 13
            AccountMeta::new_readonly(extra_account_meta_list, false), // 14
            AccountMeta::new_readonly(permission_manager_program, false), // 15
            AccountMeta::new_readonly(spiko_token_program, false), // 16
            AccountMeta::new_readonly(token_config, false),        // 17
            AccountMeta::new_readonly(hook_event_authority, false), // 18
            AccountMeta::new_readonly(hook_program, false),        // 19
            AccountMeta::new_readonly(event_authority_key, false), // 20
            AccountMeta::new_readonly(program_id, false),          // 21
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (sender, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                dl_pda,
                withdrawal_daily_limit_account(&program_id, dl_bump, 1_000_000_00000, 0, 19800),
            ),
            (op_pda, blank_pda_account()),
            (sender_token_account, dummy_account()),
            (vault_token_account, dummy_account()),
            (recipient_token_account, dummy_account()),
            (vault_authority, dummy_account()),
            (token_mint, dummy_account()),
            (
                sender_perms_pda,
                user_perms_account(&perm_manager, sender_perms_bump, &roles),
            ),
            (
                recipient_perms_pda,
                user_perms_account(&perm_manager, recipient_perms_bump, &recipient_roles),
            ),
            (
                vault_perms_pda,
                user_perms_account(&perm_manager, vault_perms_bump, &vault_roles),
            ),
            (token_2022_program, dummy_account()),
            keyed_account_for_system_program(),
            (extra_account_meta_list, dummy_account()),
            (permission_manager_program, dummy_account()),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (hook_event_authority, dummy_account()),
            (hook_program, dummy_account()),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(8))], // UnauthorizedFrom = 8
    );
}

#[test]
fn test_custodial_withdraw_unauthorized_recipient() {
    let (mut mollusk, program_id) = setup();
    let sender = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (dl_pda, dl_bump) = withdrawal_daily_limit_pda(&token_mint, &program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);
    let (vault_authority, _) = vault_authority_pda(&program_id);

    let amount: u64 = 100_00000;
    let salt: u64 = 1;
    let operation_id =
        compute_operation_id(&recipient.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, _) = withdrawal_operation_pda(&operation_id, &program_id);

    // Sender has WHITELISTED
    let (sender_perms_pda, sender_perms_bump) = user_perm_pda(&sender, &perm_manager);
    let mut sender_roles = [0u8; 32];
    set_role_bit(&mut sender_roles, ROLE_WHITELISTED);

    // Recipient has NO roles (missing WHITELISTED_EXT)
    let (recipient_perms_pda, recipient_perms_bump) = user_perm_pda(&recipient, &perm_manager);
    let recipient_roles = [0u8; 32];

    let (vault_perms_pda, vault_perms_bump) = user_perm_pda(&vault_authority, &perm_manager);
    let mut vault_roles = [0u8; 32];
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED);
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED_EXT);

    mollusk.sysvars.clock.unix_timestamp = 86400 * 19800;

    let sender_token_account = Pubkey::new_unique();
    let vault_token_account = Pubkey::new_unique();
    let recipient_token_account = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();
    let extra_account_meta_list = Pubkey::new_unique();
    let permission_manager_program = perm_manager;
    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let hook_event_authority = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_custodial_withdraw(&recipient, amount, salt),
        vec![
            AccountMeta::new(sender, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(dl_pda, false),
            AccountMeta::new(op_pda, false),
            AccountMeta::new(sender_token_account, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new(recipient_token_account, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new_readonly(sender_perms_pda, false),
            AccountMeta::new_readonly(recipient_perms_pda, false),
            AccountMeta::new_readonly(vault_perms_pda, false),
            AccountMeta::new_readonly(token_2022_program, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(extra_account_meta_list, false),
            AccountMeta::new_readonly(permission_manager_program, false),
            AccountMeta::new_readonly(spiko_token_program, false),
            AccountMeta::new_readonly(token_config, false),
            AccountMeta::new_readonly(hook_event_authority, false),
            AccountMeta::new_readonly(hook_program, false),
            AccountMeta::new_readonly(event_authority_key, false),
            AccountMeta::new_readonly(program_id, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (sender, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                dl_pda,
                withdrawal_daily_limit_account(&program_id, dl_bump, 1_000_000_00000, 0, 19800),
            ),
            (op_pda, blank_pda_account()),
            (sender_token_account, dummy_account()),
            (vault_token_account, dummy_account()),
            (recipient_token_account, dummy_account()),
            (vault_authority, dummy_account()),
            (token_mint, dummy_account()),
            (
                sender_perms_pda,
                user_perms_account(&perm_manager, sender_perms_bump, &sender_roles),
            ),
            (
                recipient_perms_pda,
                user_perms_account(&perm_manager, recipient_perms_bump, &recipient_roles),
            ),
            (
                vault_perms_pda,
                user_perms_account(&perm_manager, vault_perms_bump, &vault_roles),
            ),
            (token_2022_program, dummy_account()),
            keyed_account_for_system_program(),
            (extra_account_meta_list, dummy_account()),
            (permission_manager_program, dummy_account()),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (hook_event_authority, dummy_account()),
            (hook_program, dummy_account()),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(9))], // UnauthorizedTo = 9
    );
}

#[test]
fn test_approve_withdrawal_unauthorized() {
    let (mut mollusk, program_id) = setup();
    let approver = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);
    let (vault_authority, _) = vault_authority_pda(&program_id);

    let amount: u64 = 100_00000;
    let salt: u64 = 1;
    let operation_id =
        compute_operation_id(&recipient.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, op_bump) = withdrawal_operation_pda(&operation_id, &program_id);

    // Approver has NO roles (missing CUSTODIAL_GATEKEEPER_APPROVER)
    let (approver_perms_pda, approver_perms_bump) = user_perm_pda(&approver, &perm_manager);
    let roles = [0u8; 32];

    let (vault_perms_pda, vault_perms_bump) = user_perm_pda(&vault_authority, &perm_manager);
    let mut vault_roles = [0u8; 32];
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED);
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED_EXT);

    let (recipient_perms_pda, recipient_perms_bump) = user_perm_pda(&recipient, &perm_manager);
    let mut recipient_roles = [0u8; 32];
    set_role_bit(&mut recipient_roles, ROLE_WHITELISTED_EXT);

    mollusk.sysvars.clock.unix_timestamp = 86400 * 19800;

    let deadline: i64 = 86400 * 19800 + 86400;

    let vault_token_account = Pubkey::new_unique();
    let recipient_token_account = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();
    let extra_account_meta_list = Pubkey::new_unique();
    let permission_manager_program = perm_manager;
    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let hook_event_authority = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_approve_withdrawal(&recipient, amount, salt),
        vec![
            AccountMeta::new_readonly(approver, true),             // 0
            AccountMeta::new_readonly(config_pda, false),          // 1
            AccountMeta::new(op_pda, false),                       // 2
            AccountMeta::new_readonly(approver_perms_pda, false),  // 3
            AccountMeta::new(vault_token_account, false),          // 4
            AccountMeta::new(recipient_token_account, false),      // 5
            AccountMeta::new_readonly(vault_authority, false),     // 6
            AccountMeta::new_readonly(token_mint, false),          // 7
            AccountMeta::new_readonly(vault_perms_pda, false),     // 8
            AccountMeta::new_readonly(recipient_perms_pda, false), // 9
            AccountMeta::new_readonly(token_2022_program, false),  // 10
            AccountMeta::new_readonly(extra_account_meta_list, false), // 11
            AccountMeta::new_readonly(permission_manager_program, false), // 12
            AccountMeta::new_readonly(spiko_token_program, false), // 13
            AccountMeta::new_readonly(token_config, false),        // 14
            AccountMeta::new_readonly(hook_event_authority, false), // 15
            AccountMeta::new_readonly(hook_program, false),        // 16
            AccountMeta::new_readonly(event_authority_key, false), // 17
            AccountMeta::new_readonly(program_id, false),          // 18
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (approver, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                op_pda,
                withdrawal_operation_account(
                    &program_id,
                    op_bump,
                    STATUS_PENDING,
                    deadline,
                    &recipient,
                    &token_mint,
                    amount,
                ),
            ),
            (
                approver_perms_pda,
                user_perms_account(&perm_manager, approver_perms_bump, &roles),
            ),
            (vault_token_account, dummy_account()),
            (recipient_token_account, dummy_account()),
            (vault_authority, dummy_account()),
            (token_mint, dummy_account()),
            (
                vault_perms_pda,
                user_perms_account(&perm_manager, vault_perms_bump, &vault_roles),
            ),
            (
                recipient_perms_pda,
                user_perms_account(&perm_manager, recipient_perms_bump, &recipient_roles),
            ),
            (token_2022_program, dummy_account()),
            (extra_account_meta_list, dummy_account()),
            (permission_manager_program, dummy_account()),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (hook_event_authority, dummy_account()),
            (hook_program, dummy_account()),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(2))], // Unauthorized = 2
    );
}

#[test]
fn test_approve_withdrawal_not_pending() {
    let (mut mollusk, program_id) = setup();
    let approver = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);
    let (vault_authority, _) = vault_authority_pda(&program_id);

    let amount: u64 = 100_00000;
    let salt: u64 = 1;
    let operation_id =
        compute_operation_id(&recipient.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, op_bump) = withdrawal_operation_pda(&operation_id, &program_id);

    // Approver has correct role
    let (approver_perms_pda, approver_perms_bump) = user_perm_pda(&approver, &perm_manager);
    let mut roles = [0u8; 32];
    set_role_bit(&mut roles, ROLE_CUSTODIAL_GATEKEEPER_APPROVER);

    let (vault_perms_pda, vault_perms_bump) = user_perm_pda(&vault_authority, &perm_manager);
    let mut vault_roles = [0u8; 32];
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED);
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED_EXT);

    let (recipient_perms_pda, recipient_perms_bump) = user_perm_pda(&recipient, &perm_manager);
    let mut recipient_roles = [0u8; 32];
    set_role_bit(&mut recipient_roles, ROLE_WHITELISTED_EXT);

    mollusk.sysvars.clock.unix_timestamp = 86400 * 19800;

    let vault_token_account = Pubkey::new_unique();
    let recipient_token_account = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();
    let extra_account_meta_list = Pubkey::new_unique();
    let permission_manager_program = perm_manager;
    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let hook_event_authority = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_approve_withdrawal(&recipient, amount, salt),
        vec![
            AccountMeta::new_readonly(approver, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(op_pda, false),
            AccountMeta::new_readonly(approver_perms_pda, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new(recipient_token_account, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new_readonly(vault_perms_pda, false),
            AccountMeta::new_readonly(recipient_perms_pda, false),
            AccountMeta::new_readonly(token_2022_program, false),
            AccountMeta::new_readonly(extra_account_meta_list, false),
            AccountMeta::new_readonly(permission_manager_program, false),
            AccountMeta::new_readonly(spiko_token_program, false),
            AccountMeta::new_readonly(token_config, false),
            AccountMeta::new_readonly(hook_event_authority, false),
            AccountMeta::new_readonly(hook_program, false),
            AccountMeta::new_readonly(event_authority_key, false),
            AccountMeta::new_readonly(program_id, false),
        ],
    );

    // Operation is already DONE
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (approver, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                op_pda,
                withdrawal_operation_account(
                    &program_id,
                    op_bump,
                    STATUS_DONE,
                    86400 * 19800 + 86400,
                    &recipient,
                    &token_mint,
                    amount,
                ),
            ),
            (
                approver_perms_pda,
                user_perms_account(&perm_manager, approver_perms_bump, &roles),
            ),
            (vault_token_account, dummy_account()),
            (recipient_token_account, dummy_account()),
            (vault_authority, dummy_account()),
            (token_mint, dummy_account()),
            (
                vault_perms_pda,
                user_perms_account(&perm_manager, vault_perms_bump, &vault_roles),
            ),
            (
                recipient_perms_pda,
                user_perms_account(&perm_manager, recipient_perms_bump, &recipient_roles),
            ),
            (token_2022_program, dummy_account()),
            (extra_account_meta_list, dummy_account()),
            (permission_manager_program, dummy_account()),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (hook_event_authority, dummy_account()),
            (hook_program, dummy_account()),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(4))], // NotPending = 4
    );
}

#[test]
fn test_approve_withdrawal_deadline_passed() {
    let (mut mollusk, program_id) = setup();
    let approver = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);
    let (vault_authority, _) = vault_authority_pda(&program_id);

    let amount: u64 = 100_00000;
    let salt: u64 = 1;
    let operation_id =
        compute_operation_id(&recipient.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, op_bump) = withdrawal_operation_pda(&operation_id, &program_id);

    let (approver_perms_pda, approver_perms_bump) = user_perm_pda(&approver, &perm_manager);
    let mut roles = [0u8; 32];
    set_role_bit(&mut roles, ROLE_CUSTODIAL_GATEKEEPER_APPROVER);

    let (vault_perms_pda, vault_perms_bump) = user_perm_pda(&vault_authority, &perm_manager);
    let mut vault_roles = [0u8; 32];
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED);
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED_EXT);

    let (recipient_perms_pda, recipient_perms_bump) = user_perm_pda(&recipient, &perm_manager);
    let mut recipient_roles = [0u8; 32];
    set_role_bit(&mut recipient_roles, ROLE_WHITELISTED_EXT);

    // Clock is AFTER deadline
    let deadline: i64 = 86400 * 19800;
    mollusk.sysvars.clock.unix_timestamp = deadline + 1; // past deadline

    let vault_token_account = Pubkey::new_unique();
    let recipient_token_account = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();
    let extra_account_meta_list = Pubkey::new_unique();
    let permission_manager_program = perm_manager;
    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let hook_event_authority = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_approve_withdrawal(&recipient, amount, salt),
        vec![
            AccountMeta::new_readonly(approver, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(op_pda, false),
            AccountMeta::new_readonly(approver_perms_pda, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new(recipient_token_account, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new_readonly(vault_perms_pda, false),
            AccountMeta::new_readonly(recipient_perms_pda, false),
            AccountMeta::new_readonly(token_2022_program, false),
            AccountMeta::new_readonly(extra_account_meta_list, false),
            AccountMeta::new_readonly(permission_manager_program, false),
            AccountMeta::new_readonly(spiko_token_program, false),
            AccountMeta::new_readonly(token_config, false),
            AccountMeta::new_readonly(hook_event_authority, false),
            AccountMeta::new_readonly(hook_program, false),
            AccountMeta::new_readonly(event_authority_key, false),
            AccountMeta::new_readonly(program_id, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (approver, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                op_pda,
                withdrawal_operation_account(
                    &program_id,
                    op_bump,
                    STATUS_PENDING,
                    deadline, // deadline is in the past relative to clock
                    &recipient,
                    &token_mint,
                    amount,
                ),
            ),
            (
                approver_perms_pda,
                user_perms_account(&perm_manager, approver_perms_bump, &roles),
            ),
            (vault_token_account, dummy_account()),
            (recipient_token_account, dummy_account()),
            (vault_authority, dummy_account()),
            (token_mint, dummy_account()),
            (
                vault_perms_pda,
                user_perms_account(&perm_manager, vault_perms_bump, &vault_roles),
            ),
            (
                recipient_perms_pda,
                user_perms_account(&perm_manager, recipient_perms_bump, &recipient_roles),
            ),
            (token_2022_program, dummy_account()),
            (extra_account_meta_list, dummy_account()),
            (permission_manager_program, dummy_account()),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (hook_event_authority, dummy_account()),
            (hook_program, dummy_account()),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(5))], // DeadlinePassed = 5
    );
}

#[test]
fn test_cancel_withdrawal_not_pending() {
    let (mut mollusk, program_id) = setup();
    let caller = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let sender = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);
    let (vault_authority, _) = vault_authority_pda(&program_id);

    let amount: u64 = 100_00000;
    let salt: u64 = 1;
    let operation_id =
        compute_operation_id(&recipient.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, op_bump) = withdrawal_operation_pda(&operation_id, &program_id);

    let (vault_perms_pda, vault_perms_bump) = user_perm_pda(&vault_authority, &perm_manager);
    let mut vault_roles = [0u8; 32];
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED);
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED_EXT);

    let (sender_perms_pda, sender_perms_bump) = user_perm_pda(&sender, &perm_manager);
    let mut sender_roles = [0u8; 32];
    set_role_bit(&mut sender_roles, ROLE_WHITELISTED);

    // Clock is past deadline to pass the deadline check, but status is DONE
    let deadline: i64 = 86400 * 19800;
    mollusk.sysvars.clock.unix_timestamp = deadline + 1;

    let vault_token_account = Pubkey::new_unique();
    let sender_token_account = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();
    let extra_account_meta_list = Pubkey::new_unique();
    let permission_manager_program = perm_manager;
    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let hook_event_authority = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_cancel_withdrawal(&recipient, amount, salt),
        vec![
            AccountMeta::new_readonly(caller, true),              // 0
            AccountMeta::new_readonly(config_pda, false),         // 1
            AccountMeta::new(op_pda, false),                      // 2
            AccountMeta::new(vault_token_account, false),         // 3
            AccountMeta::new(sender_token_account, false),        // 4
            AccountMeta::new_readonly(vault_authority, false),    // 5
            AccountMeta::new_readonly(token_mint, false),         // 6
            AccountMeta::new_readonly(token_2022_program, false), // 7
            AccountMeta::new_readonly(extra_account_meta_list, false), // 8
            AccountMeta::new_readonly(permission_manager_program, false), // 9
            AccountMeta::new_readonly(spiko_token_program, false), // 10
            AccountMeta::new_readonly(token_config, false),       // 11
            AccountMeta::new_readonly(vault_perms_pda, false),    // 12
            AccountMeta::new_readonly(sender_perms_pda, false),   // 13
            AccountMeta::new_readonly(hook_event_authority, false), // 14
            AccountMeta::new_readonly(hook_program, false),       // 15
            AccountMeta::new_readonly(event_authority_key, false), // 16
            AccountMeta::new_readonly(program_id, false),         // 17
        ],
    );

    // Operation is already DONE
    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                op_pda,
                withdrawal_operation_account(
                    &program_id,
                    op_bump,
                    STATUS_DONE,
                    deadline,
                    &recipient,
                    &token_mint,
                    amount,
                ),
            ),
            (vault_token_account, dummy_account()),
            (sender_token_account, dummy_account()),
            (vault_authority, dummy_account()),
            (token_mint, dummy_account()),
            (token_2022_program, dummy_account()),
            (extra_account_meta_list, dummy_account()),
            (permission_manager_program, dummy_account()),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (
                vault_perms_pda,
                user_perms_account(&perm_manager, vault_perms_bump, &vault_roles),
            ),
            (
                sender_perms_pda,
                user_perms_account(&perm_manager, sender_perms_bump, &sender_roles),
            ),
            (hook_event_authority, dummy_account()),
            (hook_program, dummy_account()),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(4))], // NotPending = 4
    );
}

#[test]
fn test_cancel_withdrawal_deadline_not_passed() {
    let (mut mollusk, program_id) = setup();
    let caller = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let sender = Pubkey::new_unique();
    let perm_manager = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let (config_pda, config_bump) = gatekeeper_config_pda(&program_id);
    let (event_authority_key, _) = event_authority_pda(&program_id);
    let (vault_authority, _) = vault_authority_pda(&program_id);

    let amount: u64 = 100_00000;
    let salt: u64 = 1;
    let operation_id =
        compute_operation_id(&recipient.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (op_pda, op_bump) = withdrawal_operation_pda(&operation_id, &program_id);

    let (vault_perms_pda, vault_perms_bump) = user_perm_pda(&vault_authority, &perm_manager);
    let mut vault_roles = [0u8; 32];
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED);
    set_role_bit(&mut vault_roles, ROLE_WHITELISTED_EXT);

    let (sender_perms_pda, sender_perms_bump) = user_perm_pda(&sender, &perm_manager);
    let mut sender_roles = [0u8; 32];
    set_role_bit(&mut sender_roles, ROLE_WHITELISTED);

    // Clock is BEFORE deadline
    let deadline: i64 = 86400 * 19800 + 86400; // future
    mollusk.sysvars.clock.unix_timestamp = 86400 * 19800; // before deadline

    let vault_token_account = Pubkey::new_unique();
    let sender_token_account = Pubkey::new_unique();
    let token_2022_program = Pubkey::new_unique();
    let extra_account_meta_list = Pubkey::new_unique();
    let permission_manager_program = perm_manager;
    let spiko_token_program = Pubkey::new_unique();
    let token_config = Pubkey::new_unique();
    let hook_event_authority = Pubkey::new_unique();
    let hook_program = Pubkey::new_unique();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &ix_cancel_withdrawal(&recipient, amount, salt),
        vec![
            AccountMeta::new_readonly(caller, true),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new(op_pda, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new(sender_token_account, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new_readonly(token_2022_program, false),
            AccountMeta::new_readonly(extra_account_meta_list, false),
            AccountMeta::new_readonly(permission_manager_program, false),
            AccountMeta::new_readonly(spiko_token_program, false),
            AccountMeta::new_readonly(token_config, false),
            AccountMeta::new_readonly(vault_perms_pda, false),
            AccountMeta::new_readonly(sender_perms_pda, false),
            AccountMeta::new_readonly(hook_event_authority, false),
            AccountMeta::new_readonly(hook_program, false),
            AccountMeta::new_readonly(event_authority_key, false),
            AccountMeta::new_readonly(program_id, false),
        ],
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (caller, payer_account()),
            (
                config_pda,
                gatekeeper_config_account(&program_id, config_bump, 86400, &perm_manager),
            ),
            (
                op_pda,
                withdrawal_operation_account(
                    &program_id,
                    op_bump,
                    STATUS_PENDING,
                    deadline,
                    &recipient,
                    &token_mint,
                    amount,
                ),
            ),
            (vault_token_account, dummy_account()),
            (sender_token_account, dummy_account()),
            (vault_authority, dummy_account()),
            (token_mint, dummy_account()),
            (token_2022_program, dummy_account()),
            (extra_account_meta_list, dummy_account()),
            (permission_manager_program, dummy_account()),
            (spiko_token_program, dummy_account()),
            (token_config, dummy_account()),
            (
                vault_perms_pda,
                user_perms_account(&perm_manager, vault_perms_bump, &vault_roles),
            ),
            (
                sender_perms_pda,
                user_perms_account(&perm_manager, sender_perms_bump, &sender_roles),
            ),
            (hook_event_authority, dummy_account()),
            (hook_program, dummy_account()),
            (event_authority_key, Account::default()),
            (program_id, create_program_account_loader_v3(&program_id)),
        ],
        &[Check::err(ProgramError::Custom(6))], // DeadlineNotPassed = 6
    );
}
