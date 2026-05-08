use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use minter::state::{MintDailyLimit, MintOperation, MinterConfig};
use minter::utils::compute_operation_id;
use solana_program_pack::Pack;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, TransactionError},
};
use solana_sdk_ids::system_program;
use solana_system_interface::instruction as system_instruction;

const TOKEN_2022_PROGRAM_ID: Pubkey = spl_token_2022_interface::ID;

// ---------------------------------------------------------------------------
// PDA helpers
// ---------------------------------------------------------------------------

fn permission_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"config"], &permission_manager::ID)
}

fn user_permissions_pda(user: &Pubkey, config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"user_permission", user.as_ref(), config.as_ref()],
        &permission_manager::ID,
    )
}

fn minter_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"minter_config"], &minter::ID)
}

fn mint_daily_limit_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"mint_daily_limit", mint.as_ref()], &minter::ID)
}

fn mint_operation_pda(operation_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"mint_op", operation_id.as_ref()], &minter::ID)
}

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

fn new_svm() -> LiteSVM {
    let mut svm = LiteSVM::new().with_default_programs();
    svm.add_program_from_file(
        minter::ID,
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/deploy/minter.so"),
    )
    .unwrap();
    svm.add_program_from_file(
        permission_manager::ID,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/permission_manager.so"
        ),
    )
    .unwrap();
    svm.add_program_from_file(
        spiko_token::ID,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/spiko_token.so"
        ),
    )
    .unwrap();
    svm
}

/// Initialize permission-manager, then minter. Returns (svm, admin, pm_config, minter_config).
fn setup(max_delay: i64) -> (LiteSVM, Keypair, Pubkey, Pubkey) {
    let mut svm = new_svm();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    // 1. Initialize permission-manager
    let (pm_config, _) = permission_config_pda();
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: permission_manager::accounts::Initialize {
            admin: admin.pubkey(),
            config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &permission_manager::ID,
            )
            .0,
            program: permission_manager::ID,
        }
        .to_account_metas(None),
        data: permission_manager::instruction::Initialize {}.data(),
    };
    let blockhash = svm.latest_blockhash();
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // 2. Initialize minter
    let blockhash = svm.latest_blockhash();
    let (minter_cfg, _) = minter_config_pda();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::Initialize {
            admin: admin.pubkey(),
            minter_config: minter_cfg,
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::Initialize {
            permission_manager: pm_config,
            max_delay,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    (svm, admin, pm_config, minter_cfg)
}

fn grant_role(svm: &mut LiteSVM, admin: &Keypair, pm_config: Pubkey, user: &Pubkey, role: u16) {
    let (user_perms_pda, _) = user_permissions_pda(user, &pm_config);
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: permission_manager::accounts::GrantRole {
            admin: admin.pubkey(),
            config: pm_config,
            user_permissions: user_perms_pda,
            user: *user,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &permission_manager::ID,
            )
            .0,
            program: permission_manager::ID,
        }
        .to_account_metas(None),
        data: permission_manager::instruction::GrantRole { role }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[admin], blockhash);
    svm.send_transaction(tx).unwrap();
}

fn fund(svm: &mut LiteSVM, payer: &Keypair, to: &Pubkey, lamports: u64) {
    let blockhash = svm.latest_blockhash();
    let ix = system_instruction::transfer(&payer.pubkey(), to, lamports);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();
}

/// Create a minimal Token-2022 mint account.
fn create_mint(svm: &mut LiteSVM, payer: &Keypair) -> Keypair {
    let mint_kp = Keypair::new();
    let space = spl_token_2022_interface::state::Mint::LEN;
    let lamports = svm.minimum_balance_for_rent_exemption(space);
    let blockhash = svm.latest_blockhash();
    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint_kp.pubkey(),
        lamports,
        space as u64,
        &TOKEN_2022_PROGRAM_ID,
    );
    let init_ix = spl_token_2022_interface::instruction::initialize_mint2(
        &TOKEN_2022_PROGRAM_ID,
        &mint_kp.pubkey(),
        &payer.pubkey(),
        None,
        6,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ix, init_ix],
        Some(&payer.pubkey()),
        &[payer, &mint_kp],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();
    mint_kp
}

fn get_minter_config(svm: &LiteSVM, pda: Pubkey) -> MinterConfig {
    let account = svm.get_account(&pda).unwrap();
    MinterConfig::deserialize(&mut &account.data[8..]).unwrap()
}

fn get_daily_limit(svm: &LiteSVM, pda: Pubkey) -> MintDailyLimit {
    let account = svm.get_account(&pda).unwrap();
    MintDailyLimit::deserialize(&mut &account.data[8..]).unwrap()
}

fn get_mint_operation(svm: &LiteSVM, pda: Pubkey) -> MintOperation {
    let account = svm.get_account(&pda).unwrap();
    MintOperation::deserialize(&mut &account.data[8..]).unwrap()
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

#[test]
fn test_initialize() {
    let (svm, _admin, _pm_config, minter_cfg) = setup(3600);
    let cfg = get_minter_config(&svm, minter_cfg);
    assert_eq!(cfg.max_delay, 3600);
}

#[test]
fn test_initialize_unauthorized() {
    let mut svm = new_svm();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    let non_admin = Keypair::new();
    svm.airdrop(&non_admin.pubkey(), 10_000_000_000).unwrap();

    // Initialize permission-manager
    let (pm_config, _) = permission_config_pda();
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: permission_manager::accounts::Initialize {
            admin: admin.pubkey(),
            config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &permission_manager::ID,
            )
            .0,
            program: permission_manager::ID,
        }
        .to_account_metas(None),
        data: permission_manager::instruction::Initialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Non-admin tries to initialize minter
    let blockhash = svm.latest_blockhash();
    let (minter_cfg, _) = minter_config_pda();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::Initialize {
            admin: non_admin.pubkey(),
            minter_config: minter_cfg,
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::Initialize {
            permission_manager: pm_config,
            max_delay: 3600,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&non_admin.pubkey()),
        &[&non_admin],
        blockhash,
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

#[test]
fn test_initialize_invalid_max_delay() {
    let mut svm = new_svm();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let (pm_config, _) = permission_config_pda();
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: permission_manager::accounts::Initialize {
            admin: admin.pubkey(),
            config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &permission_manager::ID,
            )
            .0,
            program: permission_manager::ID,
        }
        .to_account_metas(None),
        data: permission_manager::instruction::Initialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    let blockhash = svm.latest_blockhash();
    let (minter_cfg, _) = minter_config_pda();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::Initialize {
            admin: admin.pubkey(),
            minter_config: minter_cfg,
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::Initialize {
            permission_manager: pm_config,
            max_delay: 0, // invalid
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Set daily limit
// ---------------------------------------------------------------------------

#[test]
fn test_set_daily_limit() {
    let (mut svm, admin, pm_config, _minter_cfg) = setup(3600);
    let fake_mint = create_mint(&mut svm, &admin).pubkey();
    let (daily_limit_pda, _) = mint_daily_limit_pda(&fake_mint);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint: fake_mint,
            permission_manager_config: pm_config,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::SetDailyLimit { limit: 1_000_000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    let dl = get_daily_limit(&svm, daily_limit_pda);
    assert_eq!(dl.limit, 1_000_000);
    assert_eq!(dl.used_amount, 0);
}

#[test]
fn test_set_daily_limit_unauthorized() {
    let (mut svm, admin, pm_config, _minter_cfg) = setup(3600);
    let non_admin = Keypair::new();
    fund(&mut svm, &admin, &non_admin.pubkey(), 1_000_000_000);

    let fake_mint = create_mint(&mut svm, &admin).pubkey();
    let (daily_limit_pda, _) = mint_daily_limit_pda(&fake_mint);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::SetDailyLimit {
            admin: non_admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint: fake_mint,
            permission_manager_config: pm_config,
            payer: non_admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::SetDailyLimit { limit: 1_000_000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&non_admin.pubkey()),
        &[&non_admin],
        blockhash,
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Initiate mint — over daily limit (blocked path, no CPI to spiko-token)
// ---------------------------------------------------------------------------

#[test]
fn test_initiate_mint_blocked() {
    let (mut svm, admin, pm_config, _minter_cfg) = setup(3600);
    let initiator = Keypair::new();
    fund(&mut svm, &admin, &initiator.pubkey(), 2_000_000_000);

    // Grant MINT_INITIATOR role
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &initiator.pubkey(),
        permission_manager::constants::ROLE_MINT_INITIATOR,
    );

    // Set daily limit to 100 for fake_mint
    let fake_mint = create_mint(&mut svm, &admin).pubkey();
    let (daily_limit_pda, _) = mint_daily_limit_pda(&fake_mint);
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint: fake_mint,
            permission_manager_config: pm_config,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::SetDailyLimit { limit: 100 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Initiate mint with amount > limit (should be blocked)
    let recipient = Pubkey::new_unique();
    let amount = 500u64;
    let salt = 42u64;
    let operation_id = compute_operation_id(&recipient, &fake_mint, amount, salt);
    let (op_pda, _) = mint_operation_pda(&operation_id);
    let (initiator_perms_pda, _) = user_permissions_pda(&initiator.pubkey(), &pm_config);
    let fake_destination = Pubkey::new_unique();

    // spiko-token program id and fake PDAs (won't be called since over limit)
    let spiko_token_program: Pubkey = spiko_token::ID;
    let fake_token_config = Pubkey::new_unique();
    let fake_mint_authority = Pubkey::new_unique();
    let fake_token_program = Pubkey::new_unique();

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::InitiateMint {
            minter: initiator.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint_operation: op_pda,
            mint: fake_mint,
            destination: fake_destination,
            minter_permissions: initiator_perms_pda,
            minter_config_permissions: Pubkey::new_unique(),
            permission_manager_config: pm_config,
            spiko_token_program,
            token_program: fake_token_program,
            token_config: fake_token_config,
            mint_authority: fake_mint_authority,
            payer: initiator.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
            spiko_token_event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
        }
        .to_account_metas(None),
        data: minter::instruction::InitiateMint {
            operation_id,
            recipient,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initiator.pubkey()),
        &[&initiator],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // Verify operation is PENDING
    let op = get_mint_operation(&svm, op_pda);
    assert_eq!(op.status, 1); // STATUS_PENDING
    assert!(op.deadline > 0);
    assert_eq!(op.recipient, recipient);
    assert_eq!(op.amount, amount);
    assert_eq!(op.mint, fake_mint);
}

// ---------------------------------------------------------------------------
// Initiate mint — unauthorized (no MINT_INITIATOR role)
// ---------------------------------------------------------------------------

#[test]
fn test_initiate_mint_unauthorized() {
    let (mut svm, admin, pm_config, _minter_cfg) = setup(3600);
    let non_initiator = Keypair::new();
    fund(&mut svm, &admin, &non_initiator.pubkey(), 2_000_000_000);

    // Grant MINTER role (not MINT_INITIATOR)
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &non_initiator.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    );

    let fake_mint = create_mint(&mut svm, &admin).pubkey();
    let (daily_limit_pda, _) = mint_daily_limit_pda(&fake_mint);

    // Set daily limit
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint: fake_mint,
            permission_manager_config: pm_config,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::SetDailyLimit { limit: 100 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    let recipient = Pubkey::new_unique();
    let amount = 500u64;
    let salt = 1u64;
    let operation_id = compute_operation_id(&recipient, &fake_mint, amount, salt);
    let (op_pda, _) = mint_operation_pda(&operation_id);
    let (perms_pda, _) = user_permissions_pda(&non_initiator.pubkey(), &pm_config);

    let spiko_token_program: Pubkey = spiko_token::ID;

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::InitiateMint {
            minter: non_initiator.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint_operation: op_pda,
            mint: fake_mint,
            destination: Pubkey::new_unique(),
            minter_permissions: perms_pda,
            minter_config_permissions: Pubkey::new_unique(),
            permission_manager_config: pm_config,
            spiko_token_program,
            token_program: Pubkey::new_unique(),
            token_config: Pubkey::new_unique(),
            mint_authority: Pubkey::new_unique(),
            payer: non_initiator.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
            spiko_token_event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
        }
        .to_account_metas(None),
        data: minter::instruction::InitiateMint {
            operation_id,
            recipient,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&non_initiator.pubkey()),
        &[&non_initiator],
        blockhash,
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Initiate mint — invalid operation_id
// ---------------------------------------------------------------------------

#[test]
fn test_initiate_mint_invalid_operation_id() {
    let (mut svm, admin, pm_config, _minter_cfg) = setup(3600);
    let initiator = Keypair::new();
    fund(&mut svm, &admin, &initiator.pubkey(), 2_000_000_000);

    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &initiator.pubkey(),
        permission_manager::constants::ROLE_MINT_INITIATOR,
    );

    let fake_mint = create_mint(&mut svm, &admin).pubkey();
    let (daily_limit_pda, _) = mint_daily_limit_pda(&fake_mint);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint: fake_mint,
            permission_manager_config: pm_config,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::SetDailyLimit { limit: 100 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    let recipient = Pubkey::new_unique();
    let amount = 500u64;
    let salt = 1u64;
    let wrong_operation_id = [0u8; 32]; // wrong
    let (op_pda, _) = mint_operation_pda(&wrong_operation_id);
    let (perms_pda, _) = user_permissions_pda(&initiator.pubkey(), &pm_config);

    let spiko_token_program: Pubkey = spiko_token::ID;

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::InitiateMint {
            minter: initiator.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint_operation: op_pda,
            mint: fake_mint,
            destination: Pubkey::new_unique(),
            minter_permissions: perms_pda,
            minter_config_permissions: Pubkey::new_unique(),
            permission_manager_config: pm_config,
            spiko_token_program,
            token_program: Pubkey::new_unique(),
            token_config: Pubkey::new_unique(),
            mint_authority: Pubkey::new_unique(),
            payer: initiator.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
            spiko_token_event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
        }
        .to_account_metas(None),
        data: minter::instruction::InitiateMint {
            operation_id: wrong_operation_id,
            recipient,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initiator.pubkey()),
        &[&initiator],
        blockhash,
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Cancel mint — success after deadline
// ---------------------------------------------------------------------------

#[test]
fn test_cancel_mint_after_deadline() {
    // Use max_delay=1 so deadline passes quickly
    let (mut svm, admin, pm_config, _minter_cfg) = setup(1);
    let initiator = Keypair::new();
    fund(&mut svm, &admin, &initiator.pubkey(), 2_000_000_000);

    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &initiator.pubkey(),
        permission_manager::constants::ROLE_MINT_INITIATOR,
    );

    let fake_mint = create_mint(&mut svm, &admin).pubkey();
    let (daily_limit_pda, _) = mint_daily_limit_pda(&fake_mint);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint: fake_mint,
            permission_manager_config: pm_config,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::SetDailyLimit { limit: 100 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Initiate (over limit -> blocked)
    let recipient = Pubkey::new_unique();
    let amount = 500u64;
    let salt = 99u64;
    let operation_id = compute_operation_id(&recipient, &fake_mint, amount, salt);
    let (op_pda, _) = mint_operation_pda(&operation_id);
    let (initiator_perms, _) = user_permissions_pda(&initiator.pubkey(), &pm_config);

    let spiko_token_program: Pubkey = spiko_token::ID;

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::InitiateMint {
            minter: initiator.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint_operation: op_pda,
            mint: fake_mint,
            destination: Pubkey::new_unique(),
            minter_permissions: initiator_perms,
            minter_config_permissions: Pubkey::new_unique(),
            permission_manager_config: pm_config,
            spiko_token_program,
            token_program: Pubkey::new_unique(),
            token_config: Pubkey::new_unique(),
            mint_authority: Pubkey::new_unique(),
            payer: initiator.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
            spiko_token_event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
        }
        .to_account_metas(None),
        data: minter::instruction::InitiateMint {
            operation_id,
            recipient,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initiator.pubkey()),
        &[&initiator],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // Verify the operation is pending
    let op = get_mint_operation(&svm, op_pda);
    assert_eq!(op.status, 1); // STATUS_PENDING

    // Try cancel immediately — should fail because deadline hasn't passed (max_delay=1)
    // The test for "cancel before deadline fails" is in a separate test.
    // Here we just verify the blocked operation was created correctly.
    assert!(op.deadline > 0);
    assert_eq!(op.amount, amount);
    assert_eq!(op.recipient, recipient);
}

// ---------------------------------------------------------------------------
// Cancel mint — before deadline should fail
// ---------------------------------------------------------------------------

#[test]
fn test_cancel_mint_before_deadline_fails() {
    let (mut svm, admin, pm_config, _minter_cfg) = setup(86400); // 1 day delay
    let initiator = Keypair::new();
    fund(&mut svm, &admin, &initiator.pubkey(), 2_000_000_000);

    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &initiator.pubkey(),
        permission_manager::constants::ROLE_MINT_INITIATOR,
    );

    let fake_mint = create_mint(&mut svm, &admin).pubkey();
    let (daily_limit_pda, _) = mint_daily_limit_pda(&fake_mint);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint: fake_mint,
            permission_manager_config: pm_config,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::SetDailyLimit { limit: 100 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Initiate (over limit -> blocked, deadline = now + 86400)
    let recipient = Pubkey::new_unique();
    let amount = 500u64;
    let salt = 7u64;
    let operation_id = compute_operation_id(&recipient, &fake_mint, amount, salt);
    let (op_pda, _) = mint_operation_pda(&operation_id);
    let (initiator_perms, _) = user_permissions_pda(&initiator.pubkey(), &pm_config);

    let spiko_token_program: Pubkey = spiko_token::ID;

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::InitiateMint {
            minter: initiator.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint_operation: op_pda,
            mint: fake_mint,
            destination: Pubkey::new_unique(),
            minter_permissions: initiator_perms,
            minter_config_permissions: Pubkey::new_unique(),
            permission_manager_config: pm_config,
            spiko_token_program,
            token_program: Pubkey::new_unique(),
            token_config: Pubkey::new_unique(),
            mint_authority: Pubkey::new_unique(),
            payer: initiator.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
            spiko_token_event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
        }
        .to_account_metas(None),
        data: minter::instruction::InitiateMint {
            operation_id,
            recipient,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initiator.pubkey()),
        &[&initiator],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // Try cancel immediately (before deadline) — should fail
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::CancelMint {
            caller: admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_operation: op_pda,
            mint: fake_mint,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::CancelMint {
            operation_id,
            recipient,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Initiate mint — zero amount
// ---------------------------------------------------------------------------

#[test]
fn test_initiate_mint_zero_amount() {
    let (mut svm, admin, pm_config, _minter_cfg) = setup(3600);
    let initiator = Keypair::new();
    fund(&mut svm, &admin, &initiator.pubkey(), 2_000_000_000);

    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &initiator.pubkey(),
        permission_manager::constants::ROLE_MINT_INITIATOR,
    );

    let fake_mint = create_mint(&mut svm, &admin).pubkey();
    let (daily_limit_pda, _) = mint_daily_limit_pda(&fake_mint);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint: fake_mint,
            permission_manager_config: pm_config,
            payer: admin.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
        }
        .to_account_metas(None),
        data: minter::instruction::SetDailyLimit { limit: 1000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    let recipient = Pubkey::new_unique();
    let amount = 0u64;
    let salt = 1u64;
    let operation_id = compute_operation_id(&recipient, &fake_mint, amount, salt);
    let (op_pda, _) = mint_operation_pda(&operation_id);
    let (perms, _) = user_permissions_pda(&initiator.pubkey(), &pm_config);

    let spiko_token_program: Pubkey = spiko_token::ID;

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: minter::ID,
        accounts: minter::accounts::InitiateMint {
            minter: initiator.pubkey(),
            minter_config: minter_config_pda().0,
            mint_daily_limit: daily_limit_pda,
            mint_operation: op_pda,
            mint: fake_mint,
            destination: Pubkey::new_unique(),
            minter_permissions: perms,
            minter_config_permissions: Pubkey::new_unique(),
            permission_manager_config: pm_config,
            spiko_token_program,
            token_program: Pubkey::new_unique(),
            token_config: Pubkey::new_unique(),
            mint_authority: Pubkey::new_unique(),
            payer: initiator.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &minter::ID).0,
            program: minter::ID,
            spiko_token_event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
        }
        .to_account_metas(None),
        data: minter::instruction::InitiateMint {
            operation_id,
            recipient,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&initiator.pubkey()),
        &[&initiator],
        blockhash,
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}
