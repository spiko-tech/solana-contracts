use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use custodial_gatekeeper::state::{
    GatekeeperConfig, VaultAuthority, WithdrawalDailyLimit, WithdrawalOperation,
};
use custodial_gatekeeper::utils::compute_operation_id;
use litesvm::LiteSVM;
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

fn gatekeeper_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"gatekeeper_config"], &custodial_gatekeeper::ID)
}

fn vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault"], &custodial_gatekeeper::ID)
}

fn withdrawal_daily_limit_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"withdrawal_limit", mint.as_ref()],
        &custodial_gatekeeper::ID,
    )
}

fn withdrawal_operation_pda(operation_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"withdrawal_op", operation_id.as_ref()],
        &custodial_gatekeeper::ID,
    )
}

/// Dummy addresses for transfer hook accounts in tests (no hook on test mint).
fn extra_account_metas_list_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"extra-account-metas", mint.as_ref()],
        &transfer_hook_program_id(),
    )
    .0
}

fn hook_config_pda(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[b"hook_config", mint.as_ref()],
        &transfer_hook_program_id(),
    )
    .0
}

fn transfer_hook_program_id() -> Pubkey {
    custodial_gatekeeper::constants::transfer_hook_program_id()
}

fn find_transfer_hook_event_authority() -> Pubkey {
    Pubkey::find_program_address(&[b"__event_authority"], &transfer_hook_program_id()).0
}

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

const TOKEN_2022_PROGRAM_ID: Pubkey = spl_token_2022_interface::ID;

fn create_token2022_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
) {
    let blockhash = svm.latest_blockhash();
    let space = spl_token_2022_interface::state::Mint::LEN;
    let lamports = svm.minimum_balance_for_rent_exemption(space);
    let ix = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            lamports,
            space as u64,
            &TOKEN_2022_PROGRAM_ID,
        ),
        spl_token_2022_interface::instruction::initialize_mint2(
            &TOKEN_2022_PROGRAM_ID,
            &mint.pubkey(),
            mint_authority,
            None,
            decimals,
        )
        .unwrap(),
    ];
    let tx =
        Transaction::new_signed_with_payer(&ix, Some(&payer.pubkey()), &[payer, mint], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();
}

fn create_ata(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey, owner: &Pubkey) -> Pubkey {
    let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        owner,
        mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    let blockhash = svm.latest_blockhash();
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        owner,
        mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();
    ata
}

fn mint_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    dest: &Pubkey,
    authority: &Keypair,
    amount: u64,
) {
    let blockhash = svm.latest_blockhash();
    let ix = spl_token_2022_interface::instruction::mint_to(
        &TOKEN_2022_PROGRAM_ID,
        mint,
        dest,
        &authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer, authority],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();
}

fn setup(max_delay: i64) -> (LiteSVM, Keypair, Pubkey, Pubkey, Pubkey) {
    let mut svm = LiteSVM::new().with_default_programs();
    svm.add_program_from_file(
        custodial_gatekeeper::ID,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/custodial_gatekeeper.so"
        ),
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
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    // Init permission-manager
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
    svm.expire_blockhash();

    // Init gatekeeper
    let blockhash = svm.latest_blockhash();
    let (gk_config, _) = gatekeeper_config_pda();
    let (vault_auth, _) = vault_authority_pda();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::Initialize {
            admin: admin.pubkey(),
            gatekeeper_config: gk_config,
            vault_authority: vault_auth,
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::Initialize {
            permission_manager: pm_config,
            max_delay,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    (svm, admin, pm_config, gk_config, vault_auth)
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
    svm.expire_blockhash();
}

fn fund(svm: &mut LiteSVM, payer: &Keypair, to: &Pubkey, lamports: u64) {
    let blockhash = svm.latest_blockhash();
    let ix = system_instruction::transfer(&payer.pubkey(), to, lamports);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();
}

fn get_gatekeeper_config(svm: &LiteSVM, pda: Pubkey) -> GatekeeperConfig {
    let account = svm.get_account(&pda).unwrap();
    GatekeeperConfig::deserialize(&mut &account.data[8..]).unwrap()
}

fn get_withdrawal_operation(svm: &LiteSVM, pda: Pubkey) -> WithdrawalOperation {
    let account = svm.get_account(&pda).unwrap();
    WithdrawalOperation::deserialize(&mut &account.data[8..]).unwrap()
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

#[test]
fn test_initialize() {
    let (svm, _admin, pm_config, gk_config, _vault_auth) = setup(3600);
    let cfg = get_gatekeeper_config(&svm, gk_config);
    assert_eq!(cfg.max_delay, 3600);
    assert_eq!(cfg.permission_manager, pm_config);
}

#[test]
fn test_initialize_unauthorized() {
    let mut svm = LiteSVM::new().with_default_programs();
    svm.add_program_from_file(
        custodial_gatekeeper::ID,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/custodial_gatekeeper.so"
        ),
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
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    let non_admin = Keypair::new();
    svm.airdrop(&non_admin.pubkey(), 1_000_000_000).unwrap();

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
    svm.expire_blockhash();

    let blockhash = svm.latest_blockhash();
    let (gk_config, _) = gatekeeper_config_pda();
    let (vault_auth, _) = vault_authority_pda();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::Initialize {
            admin: non_admin.pubkey(),
            gatekeeper_config: gk_config,
            vault_authority: vault_auth,
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::Initialize {
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

// ---------------------------------------------------------------------------
// Set daily limit
// ---------------------------------------------------------------------------

#[test]
fn test_set_daily_limit() {
    let (mut svm, admin, pm_config, gk_config, _vault_auth) = setup(3600);
    let mint_kp = Keypair::new();
    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    let (daily_limit_pda, _) = withdrawal_daily_limit_pda(&mint_kp.pubkey());
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            mint: mint_kp.pubkey(),
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::SetDailyLimit { limit: 5_000_000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    let account = svm.get_account(&daily_limit_pda).unwrap();
    let dl = WithdrawalDailyLimit::deserialize(&mut &account.data[8..]).unwrap();
    assert_eq!(dl.limit, 5_000_000);
    assert_eq!(dl.used_amount, 0);
}

// ---------------------------------------------------------------------------
// Custodial withdraw — within limit (immediate transfer)
// ---------------------------------------------------------------------------

#[test]
fn test_custodial_withdraw_within_limit() {
    let (mut svm, admin, pm_config, gk_config, vault_auth) = setup(3600);
    let mint_kp = Keypair::new();
    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    // Set daily limit
    let (daily_limit_pda, _) = withdrawal_daily_limit_pda(&mint_kp.pubkey());
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            mint: mint_kp.pubkey(),
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::SetDailyLimit { limit: 10_000_000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Setup sender (WHITELISTED) and recipient (WHITELISTED_EXT)
    let sender = Keypair::new();
    let recipient = Keypair::new();
    fund(&mut svm, &admin, &sender.pubkey(), 2_000_000_000);
    fund(&mut svm, &admin, &recipient.pubkey(), 1_000_000_000);

    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &sender.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &recipient.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED_EXT,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &vault_auth,
        permission_manager::constants::ROLE_WHITELISTED,
    );

    // Create token accounts
    let sender_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &sender.pubkey());
    let vault_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &vault_auth);
    let recipient_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &recipient.pubkey());

    // Mint tokens to sender
    let amount = 1_000_000u64;
    mint_tokens(
        &mut svm,
        &admin,
        &mint_kp.pubkey(),
        &sender_ata,
        &admin,
        amount,
    );

    // Custodial withdraw
    let salt = 1u64;
    let operation_id = compute_operation_id(&sender.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = withdrawal_operation_pda(&operation_id);
    let (sender_perms, _) = user_permissions_pda(&sender.pubkey(), &pm_config);
    let (recipient_perms, _) = user_permissions_pda(&recipient.pubkey(), &pm_config);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::CustodialWithdraw {
            sender: sender.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            withdrawal_operation: op_pda,
            mint: mint_kp.pubkey(),
            sender_token_account: sender_ata,
            vault_token_account: vault_ata,
            recipient_token_account: recipient_ata,
            vault_authority: vault_auth,
            sender_permissions: sender_perms,
            vault_authority_permissions: user_permissions_pda(&vault_auth, &pm_config).0,
            recipient_permissions: recipient_perms,
            permission_manager_config: pm_config,
            permission_manager_program: permission_manager::ID,
            extra_account_metas_list: extra_account_metas_list_pda(&mint_kp.pubkey()),
            hook_config: hook_config_pda(&mint_kp.pubkey()),
            transfer_hook_program: transfer_hook_program_id(),
            transfer_hook_event_authority: find_transfer_hook_event_authority(),
            token_program: TOKEN_2022_PROGRAM_ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::CustodialWithdraw {
            operation_id,
            recipient: recipient.pubkey(),
            amount,
            salt,
        }
        .data(),
    };
    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&sender.pubkey()), &[&sender], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Verify: operation is DONE, recipient got tokens
    let op = get_withdrawal_operation(&svm, op_pda);
    assert_eq!(op.status, 2); // STATUS_DONE
}

// ---------------------------------------------------------------------------
// Custodial withdraw — over limit (blocked)
// ---------------------------------------------------------------------------

#[test]
fn test_custodial_withdraw_over_limit_blocked() {
    let (mut svm, admin, pm_config, gk_config, vault_auth) = setup(3600);
    let mint_kp = Keypair::new();
    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    // Set daily limit to 100
    let (daily_limit_pda, _) = withdrawal_daily_limit_pda(&mint_kp.pubkey());
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            mint: mint_kp.pubkey(),
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::SetDailyLimit { limit: 100 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    let sender = Keypair::new();
    let recipient = Keypair::new();
    fund(&mut svm, &admin, &sender.pubkey(), 2_000_000_000);
    fund(&mut svm, &admin, &recipient.pubkey(), 1_000_000_000);

    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &sender.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &recipient.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED_EXT,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &recipient.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED_EXT,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &vault_auth,
        permission_manager::constants::ROLE_WHITELISTED,
    );

    let sender_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &sender.pubkey());
    let vault_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &vault_auth);
    let recipient_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &recipient.pubkey());

    let amount = 500u64; // > 100 limit
    mint_tokens(
        &mut svm,
        &admin,
        &mint_kp.pubkey(),
        &sender_ata,
        &admin,
        amount,
    );

    let salt = 1u64;
    let operation_id = compute_operation_id(&sender.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = withdrawal_operation_pda(&operation_id);
    let (sender_perms, _) = user_permissions_pda(&sender.pubkey(), &pm_config);
    let (recipient_perms, _) = user_permissions_pda(&recipient.pubkey(), &pm_config);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::CustodialWithdraw {
            sender: sender.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            withdrawal_operation: op_pda,
            mint: mint_kp.pubkey(),
            sender_token_account: sender_ata,
            vault_token_account: vault_ata,
            recipient_token_account: recipient_ata,
            vault_authority: vault_auth,
            sender_permissions: sender_perms,
            vault_authority_permissions: user_permissions_pda(&vault_auth, &pm_config).0,
            recipient_permissions: recipient_perms,
            permission_manager_config: pm_config,
            permission_manager_program: permission_manager::ID,
            extra_account_metas_list: extra_account_metas_list_pda(&mint_kp.pubkey()),
            hook_config: hook_config_pda(&mint_kp.pubkey()),
            transfer_hook_program: transfer_hook_program_id(),
            transfer_hook_event_authority: find_transfer_hook_event_authority(),
            token_program: TOKEN_2022_PROGRAM_ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::CustodialWithdraw {
            operation_id,
            recipient: recipient.pubkey(),
            amount,
            salt,
        }
        .data(),
    };
    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&sender.pubkey()), &[&sender], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Verify: operation is PENDING (blocked)
    let op = get_withdrawal_operation(&svm, op_pda);
    assert_eq!(op.status, 1); // STATUS_PENDING
    assert!(op.deadline > 0);
}

// ---------------------------------------------------------------------------
// Approve withdrawal
// ---------------------------------------------------------------------------

#[test]
fn test_approve_withdrawal() {
    let (mut svm, admin, pm_config, gk_config, vault_auth) = setup(86400);
    let mint_kp = Keypair::new();
    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    // Set daily limit to 100 (so 500 will be blocked)
    let (daily_limit_pda, _) = withdrawal_daily_limit_pda(&mint_kp.pubkey());
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            mint: mint_kp.pubkey(),
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::SetDailyLimit { limit: 100 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    let sender = Keypair::new();
    let recipient = Keypair::new();
    fund(&mut svm, &admin, &sender.pubkey(), 2_000_000_000);
    fund(&mut svm, &admin, &recipient.pubkey(), 1_000_000_000);

    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &sender.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &recipient.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED_EXT,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &vault_auth,
        permission_manager::constants::ROLE_WHITELISTED,
    );

    let sender_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &sender.pubkey());
    let vault_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &vault_auth);
    let recipient_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &recipient.pubkey());

    let amount = 500u64;
    mint_tokens(
        &mut svm,
        &admin,
        &mint_kp.pubkey(),
        &sender_ata,
        &admin,
        amount,
    );

    let salt = 1u64;
    let operation_id = compute_operation_id(&sender.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = withdrawal_operation_pda(&operation_id);
    let (sender_perms, _) = user_permissions_pda(&sender.pubkey(), &pm_config);
    let (recipient_perms, _) = user_permissions_pda(&recipient.pubkey(), &pm_config);

    // Create blocked withdrawal
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::CustodialWithdraw {
            sender: sender.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            withdrawal_operation: op_pda,
            mint: mint_kp.pubkey(),
            sender_token_account: sender_ata,
            vault_token_account: vault_ata,
            recipient_token_account: recipient_ata,
            vault_authority: vault_auth,
            sender_permissions: sender_perms,
            vault_authority_permissions: user_permissions_pda(&vault_auth, &pm_config).0,
            recipient_permissions: recipient_perms,
            permission_manager_config: pm_config,
            permission_manager_program: permission_manager::ID,
            extra_account_metas_list: extra_account_metas_list_pda(&mint_kp.pubkey()),
            hook_config: hook_config_pda(&mint_kp.pubkey()),
            transfer_hook_program: transfer_hook_program_id(),
            transfer_hook_event_authority: find_transfer_hook_event_authority(),
            token_program: TOKEN_2022_PROGRAM_ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::CustodialWithdraw {
            operation_id,
            recipient: recipient.pubkey(),
            amount,
            salt,
        }
        .data(),
    };
    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&sender.pubkey()), &[&sender], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // Grant CUSTODIAL_GATEKEEPER_APPROVER to approver
    let approver = Keypair::new();
    fund(&mut svm, &admin, &approver.pubkey(), 1_000_000_000);
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &approver.pubkey(),
        permission_manager::constants::ROLE_CUSTODIAL_GATEKEEPER_APPROVER,
    );
    let (approver_perms, _) = user_permissions_pda(&approver.pubkey(), &pm_config);

    // Approve
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::ApproveWithdrawal {
            approver: approver.pubkey(),
            withdrawal_operation: op_pda,
            mint: mint_kp.pubkey(),
            vault_token_account: vault_ata,
            recipient_token_account: recipient_ata,
            vault_authority: vault_auth,
            approver_permissions: approver_perms,
            vault_authority_permissions: user_permissions_pda(&vault_auth, &pm_config).0,
            recipient_permissions: recipient_perms,
            permission_manager_config: pm_config,
            permission_manager_program: permission_manager::ID,
            extra_account_metas_list: extra_account_metas_list_pda(&mint_kp.pubkey()),
            hook_config: hook_config_pda(&mint_kp.pubkey()),
            transfer_hook_program: transfer_hook_program_id(),
            transfer_hook_event_authority: find_transfer_hook_event_authority(),
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::ApproveWithdrawal {
            operation_id,
            recipient: recipient.pubkey(),
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&approver.pubkey()),
        &[&approver],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    let op = get_withdrawal_operation(&svm, op_pda);
    assert_eq!(op.status, 2); // STATUS_DONE
}

// ---------------------------------------------------------------------------
// Unauthorized sender (not WHITELISTED)
// ---------------------------------------------------------------------------

#[test]
fn test_custodial_withdraw_unauthorized_sender() {
    let (mut svm, admin, pm_config, gk_config, vault_auth) = setup(3600);
    let mint_kp = Keypair::new();
    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    let (daily_limit_pda, _) = withdrawal_daily_limit_pda(&mint_kp.pubkey());
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::SetDailyLimit {
            admin: admin.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            mint: mint_kp.pubkey(),
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::SetDailyLimit { limit: 10_000_000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();
    svm.expire_blockhash();

    // sender has MINTER role (not WHITELISTED)
    let sender = Keypair::new();
    let recipient = Keypair::new();
    fund(&mut svm, &admin, &sender.pubkey(), 2_000_000_000);

    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &sender.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &recipient.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED_EXT,
    );
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &vault_auth,
        permission_manager::constants::ROLE_WHITELISTED,
    );

    let sender_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &sender.pubkey());
    let vault_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &vault_auth);
    let recipient_ata = create_ata(&mut svm, &admin, &mint_kp.pubkey(), &recipient.pubkey());

    let amount = 100u64;
    mint_tokens(
        &mut svm,
        &admin,
        &mint_kp.pubkey(),
        &sender_ata,
        &admin,
        amount,
    );

    let salt = 1u64;
    let operation_id = compute_operation_id(&sender.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = withdrawal_operation_pda(&operation_id);
    let (sender_perms, _) = user_permissions_pda(&sender.pubkey(), &pm_config);
    let (recipient_perms, _) = user_permissions_pda(&recipient.pubkey(), &pm_config);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: custodial_gatekeeper::ID,
        accounts: custodial_gatekeeper::accounts::CustodialWithdraw {
            sender: sender.pubkey(),
            gatekeeper_config: gk_config,
            withdrawal_daily_limit: daily_limit_pda,
            withdrawal_operation: op_pda,
            mint: mint_kp.pubkey(),
            sender_token_account: sender_ata,
            vault_token_account: vault_ata,
            recipient_token_account: recipient_ata,
            vault_authority: vault_auth,
            sender_permissions: sender_perms,
            vault_authority_permissions: user_permissions_pda(&vault_auth, &pm_config).0,
            recipient_permissions: recipient_perms,
            permission_manager_config: pm_config,
            permission_manager_program: permission_manager::ID,
            extra_account_metas_list: extra_account_metas_list_pda(&mint_kp.pubkey()),
            hook_config: hook_config_pda(&mint_kp.pubkey()),
            transfer_hook_program: transfer_hook_program_id(),
            transfer_hook_event_authority: find_transfer_hook_event_authority(),
            token_program: TOKEN_2022_PROGRAM_ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &custodial_gatekeeper::ID,
            )
            .0,
            program: custodial_gatekeeper::ID,
        }
        .to_account_metas(None),
        data: custodial_gatekeeper::instruction::CustodialWithdraw {
            operation_id,
            recipient: recipient.pubkey(),
            amount,
            salt,
        }
        .data(),
    };
    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&sender.pubkey()), &[&sender], blockhash);
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}
