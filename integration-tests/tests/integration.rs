//! Cross-program integration tests using Mollusk SVM.
//!
//! Strategy: run instruction on program A → extract resulting account states →
//! feed those as input accounts to instruction on program B.
//! This verifies real data layout compatibility between programs.

#![allow(dead_code)]

use std::str::FromStr;

use mollusk_svm::{program::keyed_account_for_system_program, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_pubkey::Pubkey;

const PERMISSION_CONFIG_SEED: &[u8] = b"permission_config";
const USER_PERMISSION_SEED: &[u8] = b"user_perm";
const DISCRIMINATOR_PERMISSION_CONFIG: u8 = 1;
const DISCRIMINATOR_USER_PERMISSION: u8 = 2;
const PERMISSION_CONFIG_LEN: usize = 67;
const USER_PERMISSIONS_LEN: usize = 35;

const EVENT_AUTHORITY_SEED: &[u8] = b"event_authority";

const ROLE_PAUSER: u8 = 1;
const ROLE_MINT_APPROVER: u8 = 6;

const TOKEN_CONFIG_SEED: &[u8] = b"token_config";
const DISCRIMINATOR_TOKEN_CONFIG: u8 = 1;
const TOKEN_CONFIG_LEN: usize = 101;

const MINTER_CONFIG_SEED: &[u8] = b"minter_config";
const DAILY_LIMIT_SEED: &[u8] = b"daily_limit";
const MINT_OPERATION_SEED: &[u8] = b"mint_op";
const DISCRIMINATOR_MINTER_CONFIG: u8 = 1;
const DISCRIMINATOR_DAILY_LIMIT: u8 = 2;
const DISCRIMINATOR_MINT_OPERATION: u8 = 3;
const MINTER_CONFIG_LEN: usize = 43;
const DAILY_LIMIT_LEN: usize = 27;
const MINT_OPERATION_LEN: usize = 12;
const STATUS_PENDING: u8 = 1;

const REDEMPTION_CONFIG_SEED: &[u8] = b"redemption_config";
const TOKEN_MINIMUM_SEED: &[u8] = b"minimum";
const DISCRIMINATOR_REDEMPTION_CONFIG: u8 = 1;
const DISCRIMINATOR_TOKEN_MINIMUM: u8 = 2;
const REDEMPTION_CONFIG_LEN: usize = 35;
const TOKEN_MINIMUM_LEN: usize = 11;

struct TestEnv {
    perm_mollusk: Mollusk,
    perm_id: Pubkey,
    token_mollusk: Mollusk,
    token_id: Pubkey,
    minter_mollusk: Mollusk,
    minter_id: Pubkey,
    redemption_mollusk: Mollusk,
    redemption_id: Pubkey,
}

fn setup() -> TestEnv {
    std::env::set_var("SBF_OUT_DIR", "../target/deploy");

    let perm_id = Pubkey::from_str("2Qhjh6NXiyQEPBP9tVCkzNtLWERHbggUjbbwje1Mpqsc").unwrap();
    let token_id = Pubkey::from_str("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd").unwrap();
    let minter_id = Pubkey::from_str("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2").unwrap();
    let redemption_id = Pubkey::from_str("8opABJP3fzXuCVUnbzDZqYpnfxmCmeiXUQ49txf6BFWX").unwrap();

    let perm_mollusk = Mollusk::new(&perm_id, "permission_manager");
    let token_mollusk = Mollusk::new(&token_id, "spiko_token");
    let minter_mollusk = Mollusk::new(&minter_id, "minter");
    let redemption_mollusk = Mollusk::new(&redemption_id, "redemption");

    TestEnv {
        perm_mollusk,
        perm_id,
        token_mollusk,
        token_id,
        minter_mollusk,
        minter_id,
        redemption_mollusk,
        redemption_id,
    }
}

fn perm_config_pda(perm_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PERMISSION_CONFIG_SEED], perm_id)
}

fn user_perm_pda(user: &Pubkey, perm_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[USER_PERMISSION_SEED, user.as_ref()], perm_id)
}

fn token_config_pda(mint: &Pubkey, token_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOKEN_CONFIG_SEED, mint.as_ref()], token_id)
}

fn minter_config_pda(minter_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINTER_CONFIG_SEED], minter_id)
}

fn daily_limit_pda(mint: &Pubkey, minter_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[DAILY_LIMIT_SEED, mint.as_ref()], minter_id)
}

fn mint_op_pda(op_id: &[u8; 32], minter_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINT_OPERATION_SEED, op_id], minter_id)
}

fn redemption_config_pda(redemption_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REDEMPTION_CONFIG_SEED], redemption_id)
}

fn token_minimum_pda(mint: &Pubkey, redemption_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOKEN_MINIMUM_SEED, mint.as_ref()], redemption_id)
}

fn event_authority_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EVENT_AUTHORITY_SEED], program_id)
}

fn payer_account() -> Account {
    Account::new(10_000_000_000, 0, &Pubkey::default())
}

fn blank_pda_account() -> Account {
    Account::default()
}

/// Build an executable account entry for the self_program in self-CPI event pattern.
fn executable_account(_program_id: &Pubkey) -> Account {
    Account {
        lamports: 1_000_000,
        data: vec![],
        owner: solana_sdk_ids::bpf_loader_upgradeable::ID,
        executable: true,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated TokenConfig account (owned by spiko_token program).
fn token_config_account(
    token_program_id: &Pubkey,
    bump: u8,
    paused: u8,
    perm_manager: &Pubkey,
    spl_mint: &Pubkey,
) -> Account {
    let mut data = vec![0u8; TOKEN_CONFIG_LEN];
    data[0] = DISCRIMINATOR_TOKEN_CONFIG;
    data[1] = 1; // version
    data[2] = bump;
    data[3] = paused;
    data[4] = 0; // mint_authority_bump (unused for pause/unpause)
    data[5..37].copy_from_slice(perm_manager.as_ref());
    data[37..69].copy_from_slice(spl_mint.as_ref());
    // data[69..101] = redemption_contract, zeroed
    Account {
        lamports: 1_000_000,
        data,
        owner: *token_program_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated MinterConfig account.
fn minter_config_account(
    minter_program_id: &Pubkey,
    bump: u8,
    max_delay: i64,
    perm_manager: &Pubkey,
) -> Account {
    let mut data = vec![0u8; MINTER_CONFIG_LEN];
    data[0] = DISCRIMINATOR_MINTER_CONFIG;
    data[1] = 1; // version
    data[2] = bump;
    data[3..11].copy_from_slice(&max_delay.to_le_bytes());
    data[11..43].copy_from_slice(perm_manager.as_ref());
    Account {
        lamports: 1_000_000,
        data,
        owner: *minter_program_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated RedemptionConfig account.
fn redemption_config_account(
    redemption_program_id: &Pubkey,
    bump: u8,
    perm_manager: &Pubkey,
) -> Account {
    let mut data = vec![0u8; REDEMPTION_CONFIG_LEN];
    data[0] = DISCRIMINATOR_REDEMPTION_CONFIG;
    data[1] = 1; // version
    data[2] = bump;
    data[3..35].copy_from_slice(perm_manager.as_ref());
    Account {
        lamports: 1_000_000,
        data,
        owner: *redemption_program_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

/// Build a pre-populated MintOperation PDA (PENDING status).
fn mint_operation_account(minter_program_id: &Pubkey, bump: u8, deadline: i64) -> Account {
    let mut data = vec![0u8; MINT_OPERATION_LEN];
    data[0] = DISCRIMINATOR_MINT_OPERATION;
    data[1] = 1; // version
    data[2] = bump;
    data[3] = STATUS_PENDING;
    data[4..12].copy_from_slice(&deadline.to_le_bytes());
    Account {
        lamports: 1_000_000,
        data,
        owner: *minter_program_id,
        executable: false,
        rent_epoch: u64::MAX,
    }
}

fn ix_perm_initialize() -> Vec<u8> {
    vec![0]
}

fn ix_perm_grant_role(role_id: u8) -> Vec<u8> {
    vec![1, role_id]
}

fn ix_token_pause() -> Vec<u8> {
    vec![4]
}

fn ix_token_unpause() -> Vec<u8> {
    vec![5]
}

fn ix_token_set_redemption_contract(addr: &Pubkey) -> Vec<u8> {
    let mut data = vec![7];
    data.extend_from_slice(addr.as_ref());
    data
}

fn ix_minter_set_daily_limit(token_mint: &Pubkey, limit: u64) -> Vec<u8> {
    let mut data = vec![4]; // discriminator
    data.extend_from_slice(token_mint.as_ref());
    data.extend_from_slice(&limit.to_le_bytes());
    data
}

fn ix_minter_set_max_delay(max_delay: i64) -> Vec<u8> {
    let mut data = vec![5]; // discriminator
    data.extend_from_slice(&max_delay.to_le_bytes());
    data
}

fn ix_minter_cancel_mint(user: &Pubkey, token_mint: &Pubkey, amount: u64, salt: u64) -> Vec<u8> {
    let mut data = vec![3]; // discriminator
    data.extend_from_slice(user.as_ref());
    data.extend_from_slice(token_mint.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&salt.to_le_bytes());
    data
}

fn ix_redemption_set_minimum(token_mint: &Pubkey, minimum: u64) -> Vec<u8> {
    let mut data = vec![3]; // discriminator
    data.extend_from_slice(token_mint.as_ref());
    data.extend_from_slice(&minimum.to_le_bytes());
    data
}

/// Runs permission_manager::initialize and returns the resulting
/// (PermissionConfig account, admin UserPermissions account).
fn run_perm_initialize(mollusk: &Mollusk, perm_id: &Pubkey, admin: &Pubkey) -> (Account, Account) {
    let (config_key, _) = perm_config_pda(perm_id);
    let (admin_perms_key, _) = user_perm_pda(admin, perm_id);
    let (ea_key, _) = event_authority_pda(perm_id);

    let ix = Instruction::new_with_bytes(
        *perm_id,
        &ix_perm_initialize(),
        vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(config_key, false),
            AccountMeta::new(admin_perms_key, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(*perm_id, false),
        ],
    );

    let result = mollusk.process_instruction(
        &ix,
        &[
            (*admin, payer_account()),
            (config_key, blank_pda_account()),
            (admin_perms_key, blank_pda_account()),
            keyed_account_for_system_program(),
            (ea_key, Account::default()),
            (*perm_id, executable_account(perm_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "perm_initialize failed: {:?}",
        result.program_result
    );

    // Extract resulting accounts (index 1 = config, index 2 = admin_perms)
    let config_account = result.resulting_accounts[1].1.clone();
    let admin_perms_account = result.resulting_accounts[2].1.clone();

    (config_account, admin_perms_account)
}

/// Runs permission_manager::grant_role for a target user and returns
/// the resulting UserPermissions account.
fn run_perm_grant_role(
    mollusk: &Mollusk,
    perm_id: &Pubkey,
    admin: &Pubkey,
    config_account: &Account,
    admin_perms_account: &Account,
    target_user: &Pubkey,
    role_id: u8,
) -> Account {
    let (config_key, _) = perm_config_pda(perm_id);
    let (target_perms_key, _) = user_perm_pda(target_user, perm_id);
    let (admin_perms_key, _) = user_perm_pda(admin, perm_id);
    let (ea_key, _) = event_authority_pda(perm_id);

    let ix = Instruction::new_with_bytes(
        *perm_id,
        &ix_perm_grant_role(role_id),
        vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(target_perms_key, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(*target_user, false),
            AccountMeta::new_readonly(admin_perms_key, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(*perm_id, false),
        ],
    );

    let result = mollusk.process_instruction(
        &ix,
        &[
            (*admin, payer_account()),
            (config_key, config_account.clone()),
            (target_perms_key, blank_pda_account()),
            keyed_account_for_system_program(),
            (*target_user, Account::default()),
            (admin_perms_key, admin_perms_account.clone()),
            (ea_key, Account::default()),
            (*perm_id, executable_account(perm_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "perm_grant_role({}) failed: {:?}",
        role_id,
        result.program_result
    );

    // Return the resulting UserPermissions account
    result.resulting_accounts[2].1.clone()
}

fn compute_operation_id(user: &[u8; 32], mint: &[u8; 32], amount: u64, salt: u64) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut input = [0u8; 80];
    input[0..32].copy_from_slice(user);
    input[32..64].copy_from_slice(mint);
    input[64..72].copy_from_slice(&amount.to_le_bytes());
    input[72..80].copy_from_slice(&salt.to_le_bytes());
    let result = Sha256::digest(&input);
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

#[test]
fn test_perm_to_token_pause() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let pauser = Pubkey::new_unique();
    let spl_mint = Pubkey::new_unique();

    let (config_account, admin_perms) =
        run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    let pauser_perms = run_perm_grant_role(
        &env.perm_mollusk,
        &env.perm_id,
        &admin,
        &config_account,
        &admin_perms,
        &pauser,
        ROLE_PAUSER,
    );

    // Sanity: verify the UserPermissions account has correct layout
    assert_eq!(pauser_perms.owner, env.perm_id);
    assert_eq!(pauser_perms.data[0], DISCRIMINATOR_USER_PERMISSION);
    assert_ne!(pauser_perms.data[3] & (1 << ROLE_PAUSER), 0);

    let (token_config_key, token_config_bump) = token_config_pda(&spl_mint, &env.token_id);
    let token_config = token_config_account(
        &env.token_id,
        token_config_bump,
        0, // not paused
        &env.perm_id,
        &spl_mint,
    );

    // The pauser's UserPermissions PDA address (in the permission_manager)
    let (pauser_perms_key, _) = user_perm_pda(&pauser, &env.perm_id);

    let (ea_key, _) = event_authority_pda(&env.token_id);
    let ix = Instruction::new_with_bytes(
        env.token_id,
        &ix_token_pause(),
        vec![
            AccountMeta::new_readonly(pauser, true), // 0: caller (signer)
            AccountMeta::new(token_config_key, false), // 1: TokenConfig PDA (writable)
            AccountMeta::new_readonly(pauser_perms_key, false), // 2: caller's UserPerms
            AccountMeta::new_readonly(ea_key, false), // 3: event_authority
            AccountMeta::new_readonly(env.token_id, false), // 4: self_program
        ],
    );

    let result = env.token_mollusk.process_instruction(
        &ix,
        &[
            (pauser, payer_account()),
            (token_config_key, token_config.clone()),
            (pauser_perms_key, pauser_perms.clone()),
            (ea_key, Account::default()),
            (env.token_id, executable_account(&env.token_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "spiko_token::pause failed: {:?}",
        result.program_result
    );

    let resulting_config = &result.resulting_accounts[1].1;
    assert_eq!(resulting_config.data[3], 1, "Token should be paused");
}

#[test]
fn test_perm_to_token_unpause() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let pauser = Pubkey::new_unique();
    let spl_mint = Pubkey::new_unique();

    let (config_account, admin_perms) =
        run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);
    let pauser_perms = run_perm_grant_role(
        &env.perm_mollusk,
        &env.perm_id,
        &admin,
        &config_account,
        &admin_perms,
        &pauser,
        ROLE_PAUSER,
    );

    // Build TokenConfig that is ALREADY PAUSED
    let (token_config_key, token_config_bump) = token_config_pda(&spl_mint, &env.token_id);
    let token_config = token_config_account(
        &env.token_id,
        token_config_bump,
        1, // paused
        &env.perm_id,
        &spl_mint,
    );

    let (pauser_perms_key, _) = user_perm_pda(&pauser, &env.perm_id);

    // Call spiko_token::unpause
    let (ea_key, _) = event_authority_pda(&env.token_id);
    let ix = Instruction::new_with_bytes(
        env.token_id,
        &ix_token_unpause(),
        vec![
            AccountMeta::new_readonly(pauser, true),
            AccountMeta::new(token_config_key, false),
            AccountMeta::new_readonly(pauser_perms_key, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(env.token_id, false),
        ],
    );

    let result = env.token_mollusk.process_instruction(
        &ix,
        &[
            (pauser, payer_account()),
            (token_config_key, token_config.clone()),
            (pauser_perms_key, pauser_perms.clone()),
            (ea_key, Account::default()),
            (env.token_id, executable_account(&env.token_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "spiko_token::unpause failed: {:?}",
        result.program_result
    );

    let resulting_config = &result.resulting_accounts[1].1;
    assert_eq!(resulting_config.data[3], 0, "Token should be unpaused");
}

#[test]
fn test_perm_to_token_set_redemption_contract() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let spl_mint = Pubkey::new_unique();
    let redemption_contract = Pubkey::new_unique();

    let (perm_config_account, _) = run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    let (token_config_key, token_config_bump) = token_config_pda(&spl_mint, &env.token_id);
    let token_config =
        token_config_account(&env.token_id, token_config_bump, 0, &env.perm_id, &spl_mint);

    let (perm_config_key, _) = perm_config_pda(&env.perm_id);

    let (ea_key, _) = event_authority_pda(&env.token_id);
    let ix = Instruction::new_with_bytes(
        env.token_id,
        &ix_token_set_redemption_contract(&redemption_contract),
        vec![
            AccountMeta::new_readonly(admin, true),    // 0: admin (signer)
            AccountMeta::new(token_config_key, false), // 1: TokenConfig (writable)
            AccountMeta::new_readonly(perm_config_key, false), // 2: PermissionConfig
            AccountMeta::new_readonly(ea_key, false),  // 3: event_authority
            AccountMeta::new_readonly(env.token_id, false), // 4: self_program
        ],
    );

    let result = env.token_mollusk.process_instruction(
        &ix,
        &[
            (admin, payer_account()),
            (token_config_key, token_config.clone()),
            (perm_config_key, perm_config_account.clone()),
            (ea_key, Account::default()),
            (env.token_id, executable_account(&env.token_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "set_redemption_contract failed: {:?}",
        result.program_result
    );

    let resulting_config = &result.resulting_accounts[1].1;
    assert_eq!(
        &resulting_config.data[69..101],
        redemption_contract.as_ref(),
        "Redemption contract should be set"
    );
}

#[test]
fn test_set_redemption_contract_wrong_admin() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let not_admin = Pubkey::new_unique();
    let spl_mint = Pubkey::new_unique();
    let redemption_contract = Pubkey::new_unique();

    let (perm_config_account, _) = run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    let (token_config_key, token_config_bump) = token_config_pda(&spl_mint, &env.token_id);
    let token_config =
        token_config_account(&env.token_id, token_config_bump, 0, &env.perm_id, &spl_mint);

    let (perm_config_key, _) = perm_config_pda(&env.perm_id);

    // Call with not_admin as signer — should fail
    let (ea_key, _) = event_authority_pda(&env.token_id);
    let ix = Instruction::new_with_bytes(
        env.token_id,
        &ix_token_set_redemption_contract(&redemption_contract),
        vec![
            AccountMeta::new_readonly(not_admin, true),
            AccountMeta::new(token_config_key, false),
            AccountMeta::new_readonly(perm_config_key, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(env.token_id, false),
        ],
    );

    let result = env.token_mollusk.process_instruction(
        &ix,
        &[
            (not_admin, payer_account()),
            (token_config_key, token_config),
            (perm_config_key, perm_config_account),
            (ea_key, Account::default()),
            (env.token_id, executable_account(&env.token_id)),
        ],
    );

    // TokenError::Unauthorized = Custom(3)
    assert!(
        result.program_result.is_err(),
        "set_redemption_contract should fail with wrong admin"
    );
}

#[test]
fn test_perm_to_minter_set_daily_limit() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();

    let (perm_config_account, _) = run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    let (minter_config_key, minter_config_bump) = minter_config_pda(&env.minter_id);
    let minter_cfg = minter_config_account(
        &env.minter_id,
        minter_config_bump,
        86400, // max_delay = 1 day
        &env.perm_id,
    );

    let (perm_config_key, _) = perm_config_pda(&env.perm_id);
    let (daily_limit_key, _) = daily_limit_pda(&token_mint, &env.minter_id);

    let limit: u64 = 1_000_000_00000; // 1M tokens (5 decimals)

    let (ea_key, _) = event_authority_pda(&env.minter_id);
    let ix = Instruction::new_with_bytes(
        env.minter_id,
        &ix_minter_set_daily_limit(&token_mint, limit),
        vec![
            AccountMeta::new(admin, true), // 0: admin (signer, writable)
            AccountMeta::new_readonly(minter_config_key, false), // 1: MinterConfig
            AccountMeta::new_readonly(perm_config_key, false), // 2: PermissionConfig
            AccountMeta::new(daily_limit_key, false), // 3: DailyLimit (writable)
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 4: system
            AccountMeta::new_readonly(ea_key, false), // 5: event_authority
            AccountMeta::new_readonly(env.minter_id, false), // 6: self_program
        ],
    );

    let result = env.minter_mollusk.process_instruction(
        &ix,
        &[
            (admin, payer_account()),
            (minter_config_key, minter_cfg),
            (perm_config_key, perm_config_account),
            (daily_limit_key, blank_pda_account()),
            keyed_account_for_system_program(),
            (ea_key, Account::default()),
            (env.minter_id, executable_account(&env.minter_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "minter::set_daily_limit failed: {:?}",
        result.program_result
    );

    // Verify DailyLimit was created with the right limit value
    let dl_account = &result.resulting_accounts[3].1;
    assert_eq!(dl_account.owner, env.minter_id);
    assert_eq!(dl_account.data[0], DISCRIMINATOR_DAILY_LIMIT);
    let stored_limit = u64::from_le_bytes(dl_account.data[3..11].try_into().unwrap());
    assert_eq!(stored_limit, limit, "Daily limit should match");
}

#[test]
fn test_perm_to_minter_set_max_delay() {
    let env = setup();
    let admin = Pubkey::new_unique();

    let (perm_config_account, _) = run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    let (minter_config_key, minter_config_bump) = minter_config_pda(&env.minter_id);
    let minter_cfg = minter_config_account(
        &env.minter_id,
        minter_config_bump,
        86400, // current max_delay
        &env.perm_id,
    );

    let (perm_config_key, _) = perm_config_pda(&env.perm_id);
    let new_max_delay: i64 = 172800; // 2 days

    let (ea_key, _) = event_authority_pda(&env.minter_id);
    let ix = Instruction::new_with_bytes(
        env.minter_id,
        &ix_minter_set_max_delay(new_max_delay),
        vec![
            AccountMeta::new_readonly(admin, true), // 0: admin (signer)
            AccountMeta::new(minter_config_key, false), // 1: MinterConfig (writable)
            AccountMeta::new_readonly(perm_config_key, false), // 2: PermissionConfig
            AccountMeta::new_readonly(ea_key, false), // 3: event_authority
            AccountMeta::new_readonly(env.minter_id, false), // 4: self_program
        ],
    );

    let result = env.minter_mollusk.process_instruction(
        &ix,
        &[
            (admin, payer_account()),
            (minter_config_key, minter_cfg),
            (perm_config_key, perm_config_account),
            (ea_key, Account::default()),
            (env.minter_id, executable_account(&env.minter_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "minter::set_max_delay failed: {:?}",
        result.program_result
    );

    let cfg_account = &result.resulting_accounts[1].1;
    let stored_delay = i64::from_le_bytes(cfg_account.data[3..11].try_into().unwrap());
    assert_eq!(stored_delay, new_max_delay, "Max delay should be updated");
}

#[test]
fn test_minter_set_max_delay_wrong_admin() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let not_admin = Pubkey::new_unique();

    let (perm_config_account, _) = run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    let (minter_config_key, minter_config_bump) = minter_config_pda(&env.minter_id);
    let minter_cfg = minter_config_account(&env.minter_id, minter_config_bump, 86400, &env.perm_id);

    let (perm_config_key, _) = perm_config_pda(&env.perm_id);

    let (ea_key, _) = event_authority_pda(&env.minter_id);
    let ix = Instruction::new_with_bytes(
        env.minter_id,
        &ix_minter_set_max_delay(172800),
        vec![
            AccountMeta::new_readonly(not_admin, true),
            AccountMeta::new(minter_config_key, false),
            AccountMeta::new_readonly(perm_config_key, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(env.minter_id, false),
        ],
    );

    let result = env.minter_mollusk.process_instruction(
        &ix,
        &[
            (not_admin, payer_account()),
            (minter_config_key, minter_cfg),
            (perm_config_key, perm_config_account),
            (ea_key, Account::default()),
            (env.minter_id, executable_account(&env.minter_id)),
        ],
    );

    assert!(
        result.program_result.is_err(),
        "set_max_delay should fail with wrong admin"
    );
}

#[test]
fn test_perm_to_minter_cancel_mint() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let approver = Pubkey::new_unique();
    let user = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let amount: u64 = 500_000_00000; // 500k tokens
    let salt: u64 = 42;

    let (perm_config_account, admin_perms) =
        run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);
    let approver_perms = run_perm_grant_role(
        &env.perm_mollusk,
        &env.perm_id,
        &admin,
        &perm_config_account,
        &admin_perms,
        &approver,
        ROLE_MINT_APPROVER,
    );

    // Sanity check: bit 6 should be set
    assert_ne!(approver_perms.data[3] & (1 << ROLE_MINT_APPROVER), 0);

    let (minter_config_key, minter_config_bump) = minter_config_pda(&env.minter_id);
    let minter_cfg = minter_config_account(&env.minter_id, minter_config_bump, 86400, &env.perm_id);

    let op_id = compute_operation_id(&user.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (mint_op_key, mint_op_bump) = mint_op_pda(&op_id, &env.minter_id);
    let mint_op = mint_operation_account(
        &env.minter_id,
        mint_op_bump,
        1_700_000_000 + 86400, // deadline in future
    );

    let (approver_perms_key, _) = user_perm_pda(&approver, &env.perm_id);

    let (ea_key, _) = event_authority_pda(&env.minter_id);
    let ix = Instruction::new_with_bytes(
        env.minter_id,
        &ix_minter_cancel_mint(&user, &token_mint, amount, salt),
        vec![
            AccountMeta::new_readonly(approver, true), // 0: caller (signer)
            AccountMeta::new_readonly(minter_config_key, false), // 1: MinterConfig
            AccountMeta::new(mint_op_key, false),      // 2: MintOperation (writable)
            AccountMeta::new_readonly(approver_perms_key, false), // 3: caller's UserPerms
            AccountMeta::new_readonly(ea_key, false),  // 4: event_authority
            AccountMeta::new_readonly(env.minter_id, false), // 5: self_program
        ],
    );

    let result = env.minter_mollusk.process_instruction(
        &ix,
        &[
            (approver, payer_account()),
            (minter_config_key, minter_cfg),
            (mint_op_key, mint_op),
            (approver_perms_key, approver_perms.clone()),
            (ea_key, Account::default()),
            (env.minter_id, executable_account(&env.minter_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "minter::cancel_mint failed: {:?}",
        result.program_result
    );

    let op_account = &result.resulting_accounts[2].1;
    assert_eq!(
        op_account.data[3], 2,
        "MintOperation should be DONE after cancel"
    );
}

#[test]
fn test_minter_cancel_mint_wrong_role() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let pauser = Pubkey::new_unique(); // has PAUSER, not MINT_APPROVER
    let user = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();
    let amount: u64 = 500_000_00000;
    let salt: u64 = 42;

    // Initialize + grant PAUSER (not MINT_APPROVER)
    let (perm_config_account, admin_perms) =
        run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);
    let pauser_perms = run_perm_grant_role(
        &env.perm_mollusk,
        &env.perm_id,
        &admin,
        &perm_config_account,
        &admin_perms,
        &pauser,
        ROLE_PAUSER,
    );

    let (minter_config_key, minter_config_bump) = minter_config_pda(&env.minter_id);
    let minter_cfg = minter_config_account(&env.minter_id, minter_config_bump, 86400, &env.perm_id);

    let op_id = compute_operation_id(&user.to_bytes(), &token_mint.to_bytes(), amount, salt);
    let (mint_op_key, mint_op_bump) = mint_op_pda(&op_id, &env.minter_id);
    let mint_op = mint_operation_account(&env.minter_id, mint_op_bump, 1_700_000_000 + 86400);

    let (pauser_perms_key, _) = user_perm_pda(&pauser, &env.perm_id);

    let (ea_key, _) = event_authority_pda(&env.minter_id);
    let ix = Instruction::new_with_bytes(
        env.minter_id,
        &ix_minter_cancel_mint(&user, &token_mint, amount, salt),
        vec![
            AccountMeta::new_readonly(pauser, true),
            AccountMeta::new_readonly(minter_config_key, false),
            AccountMeta::new(mint_op_key, false),
            AccountMeta::new_readonly(pauser_perms_key, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(env.minter_id, false),
        ],
    );

    let result = env.minter_mollusk.process_instruction(
        &ix,
        &[
            (pauser, payer_account()),
            (minter_config_key, minter_cfg),
            (mint_op_key, mint_op),
            (pauser_perms_key, pauser_perms),
            (ea_key, Account::default()),
            (env.minter_id, executable_account(&env.minter_id)),
        ],
    );

    // MinterError::Unauthorized = Custom(2)
    assert!(
        result.program_result.is_err(),
        "cancel_mint should fail without ROLE_MINT_APPROVER"
    );
}

#[test]
fn test_perm_to_redemption_set_minimum() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();

    let (perm_config_account, _) = run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    let (redemption_config_key, redemption_config_bump) = redemption_config_pda(&env.redemption_id);
    let redemption_cfg =
        redemption_config_account(&env.redemption_id, redemption_config_bump, &env.perm_id);

    let (perm_config_key, _) = perm_config_pda(&env.perm_id);
    let (token_min_key, _) = token_minimum_pda(&token_mint, &env.redemption_id);

    let minimum: u64 = 100_00000; // 100 tokens (5 decimals)

    let (ea_key, _) = event_authority_pda(&env.redemption_id);
    let ix = Instruction::new_with_bytes(
        env.redemption_id,
        &ix_redemption_set_minimum(&token_mint, minimum),
        vec![
            AccountMeta::new(admin, true), // 0: admin (signer, writable)
            AccountMeta::new_readonly(redemption_config_key, false), // 1: RedemptionConfig
            AccountMeta::new_readonly(perm_config_key, false), // 2: PermissionConfig
            AccountMeta::new(token_min_key, false), // 3: TokenMinimum (writable)
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false), // 4: system
            AccountMeta::new_readonly(ea_key, false), // 5: event_authority
            AccountMeta::new_readonly(env.redemption_id, false), // 6: self_program
        ],
    );

    let result = env.redemption_mollusk.process_instruction(
        &ix,
        &[
            (admin, payer_account()),
            (redemption_config_key, redemption_cfg),
            (perm_config_key, perm_config_account.clone()),
            (token_min_key, blank_pda_account()),
            keyed_account_for_system_program(),
            (ea_key, Account::default()),
            (env.redemption_id, executable_account(&env.redemption_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "redemption::set_minimum failed: {:?}",
        result.program_result
    );

    let tm_account = &result.resulting_accounts[3].1;
    assert_eq!(tm_account.owner, env.redemption_id);
    assert_eq!(tm_account.data[0], DISCRIMINATOR_TOKEN_MINIMUM);
    let stored_min = u64::from_le_bytes(tm_account.data[3..11].try_into().unwrap());
    assert_eq!(stored_min, minimum, "TokenMinimum should match");
}

#[test]
fn test_redemption_set_minimum_wrong_admin() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let not_admin = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();

    let (perm_config_account, _) = run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    let (redemption_config_key, redemption_config_bump) = redemption_config_pda(&env.redemption_id);
    let redemption_cfg =
        redemption_config_account(&env.redemption_id, redemption_config_bump, &env.perm_id);

    let (perm_config_key, _) = perm_config_pda(&env.perm_id);
    let (token_min_key, _) = token_minimum_pda(&token_mint, &env.redemption_id);

    let (ea_key, _) = event_authority_pda(&env.redemption_id);
    let ix = Instruction::new_with_bytes(
        env.redemption_id,
        &ix_redemption_set_minimum(&token_mint, 100_00000),
        vec![
            AccountMeta::new(not_admin, true),
            AccountMeta::new_readonly(redemption_config_key, false),
            AccountMeta::new_readonly(perm_config_key, false),
            AccountMeta::new(token_min_key, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(env.redemption_id, false),
        ],
    );

    let result = env.redemption_mollusk.process_instruction(
        &ix,
        &[
            (not_admin, payer_account()),
            (redemption_config_key, redemption_cfg),
            (perm_config_key, perm_config_account),
            (token_min_key, blank_pda_account()),
            keyed_account_for_system_program(),
            (ea_key, Account::default()),
            (env.redemption_id, executable_account(&env.redemption_id)),
        ],
    );

    // RedemptionError::Unauthorized = Custom(2)
    assert!(
        result.program_result.is_err(),
        "set_minimum should fail with wrong admin"
    );
}

#[test]
fn test_pause_fails_wrong_perm_owner() {
    let env = setup();
    let pauser = Pubkey::new_unique();
    let spl_mint = Pubkey::new_unique();
    let fake_perm_id = Pubkey::new_unique(); // different from what TokenConfig references

    // Build TokenConfig pointing to env.perm_id
    let (token_config_key, token_config_bump) = token_config_pda(&spl_mint, &env.token_id);
    let token_config = token_config_account(
        &env.token_id,
        token_config_bump,
        0,
        &env.perm_id, // expects accounts owned by env.perm_id
        &spl_mint,
    );

    // Build a fake UserPerms account owned by a DIFFERENT program
    let (pauser_perms_key, _) = user_perm_pda(&pauser, &env.perm_id);
    let mut fake_perms_data = vec![0u8; USER_PERMISSIONS_LEN];
    fake_perms_data[0] = DISCRIMINATOR_USER_PERMISSION;
    fake_perms_data[1] = 1; // version
    fake_perms_data[2] = 0; // bump
    fake_perms_data[3] = 1 << ROLE_PAUSER; // PAUSER bit set
    let fake_perms = Account {
        lamports: 1_000_000,
        data: fake_perms_data,
        owner: fake_perm_id, // WRONG owner
        executable: false,
        rent_epoch: u64::MAX,
    };

    let (ea_key, _) = event_authority_pda(&env.token_id);
    let ix = Instruction::new_with_bytes(
        env.token_id,
        &ix_token_pause(),
        vec![
            AccountMeta::new_readonly(pauser, true),
            AccountMeta::new(token_config_key, false),
            AccountMeta::new_readonly(pauser_perms_key, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(env.token_id, false),
        ],
    );

    let result = env.token_mollusk.process_instruction(
        &ix,
        &[
            (pauser, payer_account()),
            (token_config_key, token_config),
            (pauser_perms_key, fake_perms),
            (ea_key, Account::default()),
            (env.token_id, executable_account(&env.token_id)),
        ],
    );

    // TokenError::Unauthorized = Custom(3)
    assert!(
        result.program_result.is_err(),
        "pause should fail when UserPerms owned by wrong program"
    );
}

#[test]
fn test_pause_fails_without_pauser_role() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let minter_user = Pubkey::new_unique();
    let spl_mint = Pubkey::new_unique();

    // Initialize + grant MINTER (bit 0) — NOT PAUSER
    let (perm_config_account, admin_perms) =
        run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);
    let minter_perms = run_perm_grant_role(
        &env.perm_mollusk,
        &env.perm_id,
        &admin,
        &perm_config_account,
        &admin_perms,
        &minter_user,
        0, // ROLE_MINTER
    );

    // Sanity: MINTER bit set, PAUSER bit not set
    assert_ne!(minter_perms.data[3] & 1, 0); // MINTER
    assert_eq!(minter_perms.data[3] & (1 << ROLE_PAUSER), 0); // NOT PAUSER

    let (token_config_key, token_config_bump) = token_config_pda(&spl_mint, &env.token_id);
    let token_config =
        token_config_account(&env.token_id, token_config_bump, 0, &env.perm_id, &spl_mint);

    let (minter_perms_key, _) = user_perm_pda(&minter_user, &env.perm_id);

    let (ea_key, _) = event_authority_pda(&env.token_id);
    let ix = Instruction::new_with_bytes(
        env.token_id,
        &ix_token_pause(),
        vec![
            AccountMeta::new_readonly(minter_user, true),
            AccountMeta::new(token_config_key, false),
            AccountMeta::new_readonly(minter_perms_key, false),
            AccountMeta::new_readonly(ea_key, false),
            AccountMeta::new_readonly(env.token_id, false),
        ],
    );

    let result = env.token_mollusk.process_instruction(
        &ix,
        &[
            (minter_user, payer_account()),
            (token_config_key, token_config),
            (minter_perms_key, minter_perms),
            (ea_key, Account::default()),
            (env.token_id, executable_account(&env.token_id)),
        ],
    );

    assert!(
        result.program_result.is_err(),
        "pause should fail without PAUSER role"
    );
}

#[test]
fn test_multi_role_grant_and_use() {
    let env = setup();
    let admin = Pubkey::new_unique();
    let user_a = Pubkey::new_unique();
    let spl_mint = Pubkey::new_unique();

    // Initialize
    let (perm_config, admin_perms) = run_perm_initialize(&env.perm_mollusk, &env.perm_id, &admin);

    // Grant MINTER to user_a first (creates UserPerms with bit 0)
    let user_a_perms_v1 = run_perm_grant_role(
        &env.perm_mollusk,
        &env.perm_id,
        &admin,
        &perm_config,
        &admin_perms,
        &user_a,
        0, // ROLE_MINTER
    );

    // Now grant PAUSER to same user_a (should add bit 1 to existing account)
    let (user_a_perms_key, _) = user_perm_pda(&user_a, &env.perm_id);
    let (config_key, _) = perm_config_pda(&env.perm_id);
    let (admin_perms_key, _) = user_perm_pda(&admin, &env.perm_id);
    let (ea_perm_key, _) = event_authority_pda(&env.perm_id);

    let ix = Instruction::new_with_bytes(
        env.perm_id,
        &ix_perm_grant_role(ROLE_PAUSER),
        vec![
            AccountMeta::new(admin, true),
            AccountMeta::new_readonly(config_key, false),
            AccountMeta::new(user_a_perms_key, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new_readonly(user_a, false),
            AccountMeta::new_readonly(admin_perms_key, false),
            AccountMeta::new_readonly(ea_perm_key, false),
            AccountMeta::new_readonly(env.perm_id, false),
        ],
    );

    let result = env.perm_mollusk.process_instruction(
        &ix,
        &[
            (admin, payer_account()),
            (config_key, perm_config.clone()),
            (user_a_perms_key, user_a_perms_v1), // pass the existing account
            keyed_account_for_system_program(),
            (user_a, Account::default()),
            (admin_perms_key, admin_perms.clone()),
            (ea_perm_key, Account::default()),
            (env.perm_id, executable_account(&env.perm_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "grant second role failed: {:?}",
        result.program_result
    );

    let user_a_perms_v2 = result.resulting_accounts[2].1.clone();

    // Verify both bits are set
    assert_ne!(
        user_a_perms_v2.data[3] & 1,
        0,
        "MINTER bit should still be set"
    );
    assert_ne!(
        user_a_perms_v2.data[3] & (1 << ROLE_PAUSER),
        0,
        "PAUSER bit should now be set"
    );

    // Now use user_a to pause the token
    let (token_config_key, token_config_bump) = token_config_pda(&spl_mint, &env.token_id);
    let token_config =
        token_config_account(&env.token_id, token_config_bump, 0, &env.perm_id, &spl_mint);

    let (ea_token_key, _) = event_authority_pda(&env.token_id);
    let ix = Instruction::new_with_bytes(
        env.token_id,
        &ix_token_pause(),
        vec![
            AccountMeta::new_readonly(user_a, true),
            AccountMeta::new(token_config_key, false),
            AccountMeta::new_readonly(user_a_perms_key, false),
            AccountMeta::new_readonly(ea_token_key, false),
            AccountMeta::new_readonly(env.token_id, false),
        ],
    );

    let result = env.token_mollusk.process_instruction(
        &ix,
        &[
            (user_a, payer_account()),
            (token_config_key, token_config),
            (user_a_perms_key, user_a_perms_v2),
            (ea_token_key, Account::default()),
            (env.token_id, executable_account(&env.token_id)),
        ],
    );

    assert!(
        !result.program_result.is_err(),
        "pause with multi-role user failed: {:?}",
        result.program_result
    );

    let resulting_config = &result.resulting_accounts[1].1;
    assert_eq!(resulting_config.data[3], 1, "Token should be paused");
}
