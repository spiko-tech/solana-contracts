use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use redemption::state::{RedemptionConfig, RedemptionOperation, VaultAuthority};
use redemption::utils::compute_operation_id;
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

fn redemption_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"redemption_config"], &redemption::ID)
}

fn vault_authority_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"vault_authority", mint.as_ref()], &redemption::ID)
}

fn redemption_operation_pda(operation_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"redemption_op", operation_id.as_ref()], &redemption::ID)
}

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

const TOKEN_2022_PROGRAM_ID: Pubkey = spl_token_2022_interface::ID;

fn new_svm() -> LiteSVM {
    let mut svm = LiteSVM::new().with_default_programs();
    svm.add_program_from_file(
        redemption::ID,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/redemption.so"
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
    svm
}

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
    ata
}

/// Setup: init permission-manager + redemption program. Returns everything needed.
fn setup(deadline_delay: i64) -> (LiteSVM, Keypair, Pubkey, Pubkey) {
    let mut svm = new_svm();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let blockhash = svm.latest_blockhash();

    // Init permission-manager
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
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Init redemption
    let blockhash = svm.latest_blockhash();
    let (redemption_cfg, _) = redemption_config_pda();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::Initialize {
            admin: admin.pubkey(),
            redemption_config: redemption_cfg,
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::Initialize { deadline_delay }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    (svm, admin, pm_config, redemption_cfg)
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
}

fn fund(svm: &mut LiteSVM, payer: &Keypair, to: &Pubkey, lamports: u64) {
    let blockhash = svm.latest_blockhash();
    let ix = system_instruction::transfer(&payer.pubkey(), to, lamports);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);
    svm.send_transaction(tx).unwrap();
}

fn get_redemption_config(svm: &LiteSVM, pda: Pubkey) -> RedemptionConfig {
    let account = svm.get_account(&pda).unwrap();
    RedemptionConfig::deserialize(&mut &account.data[8..]).unwrap()
}

fn get_redemption_operation(svm: &LiteSVM, pda: Pubkey) -> RedemptionOperation {
    let account = svm.get_account(&pda).unwrap();
    RedemptionOperation::deserialize(&mut &account.data[8..]).unwrap()
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

#[test]
fn test_initialize() {
    let (svm, _admin, pm_config, redemption_cfg) = setup(7200);
    let cfg = get_redemption_config(&svm, redemption_cfg);
    assert_eq!(cfg.deadline_delay, 7200);
    assert_eq!(cfg.permission_manager, pm_config);
}

#[test]
fn test_initialize_unauthorized() {
    let mut svm = new_svm();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    let non_admin = Keypair::new();
    svm.airdrop(&non_admin.pubkey(), 10_000_000_000).unwrap();

    let blockhash = svm.latest_blockhash();

    // Init permission-manager
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
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Non-admin tries to initialize redemption
    let blockhash = svm.latest_blockhash();
    let (redemption_cfg, _) = redemption_config_pda();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::Initialize {
            admin: non_admin.pubkey(),
            redemption_config: redemption_cfg,
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::Initialize {
            deadline_delay: 3600,
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
// Create vault
// ---------------------------------------------------------------------------

#[test]
fn test_create_vault() {
    let (mut svm, admin, pm_config, _redemption_cfg) = setup(3600);
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    // Create Token-2022 mint
    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    // Create vault ATA for vault_authority
    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::CreateVault {
            admin: admin.pubkey(),
            redemption_config: redemption_config_pda().0,
            permission_manager_config: pm_config,
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            vault: vault_ata,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Verify vault authority exists
    let account = svm.get_account(&vault_auth).unwrap();
    let va = VaultAuthority::deserialize(&mut &account.data[8..]).unwrap();
    assert!(va.bump > 0);
}

// ---------------------------------------------------------------------------
// on_redeem
// ---------------------------------------------------------------------------

#[test]
fn test_on_redeem() {
    let (mut svm, admin, pm_config, _redemption_cfg) = setup(3600);
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    // Create mint + vault
    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::CreateVault {
            admin: admin.pubkey(),
            redemption_config: redemption_config_pda().0,
            permission_manager_config: pm_config,
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            vault: vault_ata,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // on_redeem
    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut svm, &admin, &user.pubkey(), 2_000_000_000);
    let amount = 1_000_000u64;
    let salt = 42u64;
    let operation_id = compute_operation_id(&user.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = redemption_operation_pda(&operation_id);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::OnRedeem {
            user: user.pubkey(),
            mint_authority: mint_auth_kp.pubkey(),
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            redemption_config: redemption_config_pda().0,
            redemption_operation: op_pda,
            payer: user.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::OnRedeem {
            operation_id,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &mint_auth_kp],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // Verify operation
    let op = get_redemption_operation(&svm, op_pda);
    assert_eq!(op.status, 1); // STATUS_PENDING
    assert_eq!(op.amount, amount);
    assert_eq!(op.user, user.pubkey());
    assert_eq!(op.mint, mint_kp.pubkey());
    assert!(op.deadline > 0);
}

// ---------------------------------------------------------------------------
// on_redeem — invalid operation_id
// ---------------------------------------------------------------------------

#[test]
fn test_on_redeem_invalid_operation_id() {
    let (mut svm, admin, pm_config, _redemption_cfg) = setup(3600);
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::CreateVault {
            admin: admin.pubkey(),
            redemption_config: redemption_config_pda().0,
            permission_manager_config: pm_config,
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            vault: vault_ata,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut svm, &admin, &user.pubkey(), 2_000_000_000);

    let wrong_op_id = [0u8; 32];
    let (op_pda, _) = redemption_operation_pda(&wrong_op_id);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::OnRedeem {
            user: user.pubkey(),
            mint_authority: mint_auth_kp.pubkey(),
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            redemption_config: redemption_config_pda().0,
            redemption_operation: op_pda,
            payer: user.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::OnRedeem {
            operation_id: wrong_op_id,
            amount: 1000,
            salt: 1,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &mint_auth_kp],
        blockhash,
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// on_redeem — no vault (unsupported mint)
// ---------------------------------------------------------------------------

#[test]
fn test_on_redeem_no_vault() {
    let (mut svm, admin, _pm_config, _redemption_cfg) = setup(3600);
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());
    // Note: NOT calling create_vault

    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut svm, &admin, &user.pubkey(), 2_000_000_000);

    let amount = 1000u64;
    let salt = 1u64;
    let operation_id = compute_operation_id(&user.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = redemption_operation_pda(&operation_id);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::OnRedeem {
            user: user.pubkey(),
            mint_authority: mint_auth_kp.pubkey(),
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            redemption_config: redemption_config_pda().0,
            redemption_operation: op_pda,
            payer: user.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::OnRedeem {
            operation_id,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &mint_auth_kp],
        blockhash,
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Execute (burn) — requires REDEMPTION_EXECUTOR role + tokens in vault
// ---------------------------------------------------------------------------

#[test]
fn test_execute_redemption() {
    let (mut svm, admin, pm_config, _redemption_cfg) = setup(86400);
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    // Create mint with vault_authority as the freeze/mint authority won't matter,
    // but we need admin as mint authority to mint tokens into vault
    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    // Create vault
    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::CreateVault {
            admin: admin.pubkey(),
            redemption_config: redemption_config_pda().0,
            permission_manager_config: pm_config,
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            vault: vault_ata,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Mint tokens into vault (simulating a redeem deposit)
    let amount = 1_000_000u64;
    let blockhash = svm.latest_blockhash();
    let mint_ix = spl_token_2022_interface::instruction::mint_to(
        &TOKEN_2022_PROGRAM_ID,
        &mint_kp.pubkey(),
        &vault_ata,
        &admin.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    let tx =
        Transaction::new_signed_with_payer(&[mint_ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Create on_redeem operation
    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut svm, &admin, &user.pubkey(), 2_000_000_000);
    let salt = 42u64;
    let operation_id = compute_operation_id(&user.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = redemption_operation_pda(&operation_id);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::OnRedeem {
            user: user.pubkey(),
            mint_authority: mint_auth_kp.pubkey(),
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            redemption_config: redemption_config_pda().0,
            redemption_operation: op_pda,
            payer: user.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::OnRedeem {
            operation_id,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &mint_auth_kp],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // Grant REDEMPTION_EXECUTOR to burner
    let burner = Keypair::new();
    fund(&mut svm, &admin, &burner.pubkey(), 1_000_000_000);
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &burner.pubkey(),
        permission_manager::constants::ROLE_REDEMPTION_EXECUTOR,
    );
    let (burner_perms, _) = user_permissions_pda(&burner.pubkey(), &pm_config);

    // Execute
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::Execute {
            burner: burner.pubkey(),
            mint: mint_kp.pubkey(),
            redemption_config: redemption_config_pda().0,
            redemption_operation: op_pda,
            vault: vault_ata,
            vault_authority: vault_auth,
            burner_permissions: burner_perms,
            permission_manager_config: pm_config,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::Execute {
            operation_id,
            amount,
            salt,
        }
        .data(),
    };
    let tx =
        Transaction::new_signed_with_payer(&[ix], Some(&burner.pubkey()), &[&burner], blockhash);
    svm.send_transaction(tx).unwrap();

    // Verify operation done
    let op = get_redemption_operation(&svm, op_pda);
    assert_eq!(op.status, 2); // STATUS_DONE
}

// ---------------------------------------------------------------------------
// Execute — unauthorized (no REDEMPTION_EXECUTOR)
// ---------------------------------------------------------------------------

#[test]
fn test_execute_unauthorized() {
    let (mut svm, admin, pm_config, _redemption_cfg) = setup(86400);
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    create_token2022_mint(&mut svm, &admin, &mint_kp, 6, &admin.pubkey());

    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::CreateVault {
            admin: admin.pubkey(),
            redemption_config: redemption_config_pda().0,
            permission_manager_config: pm_config,
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            vault: vault_ata,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: spl_associated_token_account::ID,
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Create operation
    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut svm, &admin, &user.pubkey(), 2_000_000_000);
    let amount = 1000u64;
    let salt = 1u64;
    let operation_id = compute_operation_id(&user.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = redemption_operation_pda(&operation_id);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::OnRedeem {
            user: user.pubkey(),
            mint_authority: mint_auth_kp.pubkey(),
            mint: mint_kp.pubkey(),
            vault_authority: vault_auth,
            redemption_config: redemption_config_pda().0,
            redemption_operation: op_pda,
            payer: user.pubkey(),
            system_program: system_program::ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::OnRedeem {
            operation_id,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&user.pubkey()),
        &[&user, &mint_auth_kp],
        blockhash,
    );
    svm.send_transaction(tx).unwrap();

    // Non-executor tries to execute
    let non_executor = Keypair::new();
    fund(&mut svm, &admin, &non_executor.pubkey(), 1_000_000_000);
    // Grant MINTER (not REDEMPTION_EXECUTOR)
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &non_executor.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    );
    let (perms, _) = user_permissions_pda(&non_executor.pubkey(), &pm_config);

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::Execute {
            burner: non_executor.pubkey(),
            mint: mint_kp.pubkey(),
            redemption_config: redemption_config_pda().0,
            redemption_operation: op_pda,
            vault: vault_ata,
            vault_authority: vault_auth,
            burner_permissions: perms,
            permission_manager_config: pm_config,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(&[b"__event_authority"], &redemption::ID)
                .0,
            program: redemption::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::Execute {
            operation_id,
            amount,
            salt,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&non_executor.pubkey()),
        &[&non_executor],
        blockhash,
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}
