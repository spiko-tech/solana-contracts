use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use redemption::state::{RedemptionConfig, RedemptionOperation, VaultAuthority};
use redemption::utils::compute_operation_id;
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_instruction,
    transaction::{Transaction, TransactionError},
};

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

const TOKEN_2022_PROGRAM_ID: Pubkey = spl_token_2022::ID;

fn program_test() -> ProgramTest {
    let mut pt = ProgramTest::new("redemption", redemption::ID, None);
    pt.add_program("permission_manager", permission_manager::ID, None);
    pt
}

async fn create_token2022_mint(
    banks: &mut BanksClient,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
) {
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let space = spl_token_2022::state::Mint::LEN;
    let rent = banks.get_rent().await.unwrap();
    let lamports = rent.minimum_balance(space);

    let ix = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            lamports,
            space as u64,
            &TOKEN_2022_PROGRAM_ID,
        ),
        spl_token_2022::instruction::initialize_mint2(
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
    banks.process_transaction(tx).await.unwrap();
}

async fn create_ata(
    banks: &mut BanksClient,
    payer: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
) -> Pubkey {
    let ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        owner,
        mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        owner,
        mint,
        &TOKEN_2022_PROGRAM_ID,
    );
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);
    banks.process_transaction(tx).await.unwrap();
    ata
}

/// Setup: init permission-manager + redemption program. Returns everything needed.
async fn setup(deadline_delay: i64) -> (BanksClient, Keypair, Pubkey, Pubkey) {
    let ctx = program_test().start_with_context().await;
    let admin = ctx.payer.insecure_clone();
    let mut banks = ctx.banks_client.clone();
    let blockhash = ctx.last_blockhash;

    // Init permission-manager
    let (pm_config, _) = permission_config_pda();
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: permission_manager::accounts::Initialize {
            admin: admin.pubkey(),
            config: pm_config,
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: permission_manager::instruction::Initialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Init redemption
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let (redemption_cfg, _) = redemption_config_pda();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::Initialize {
            admin: admin.pubkey(),
            redemption_config: redemption_cfg,
            permission_manager_config: pm_config,
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::Initialize { deadline_delay }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    (banks, admin, pm_config, redemption_cfg)
}

async fn grant_role(
    banks: &mut BanksClient,
    admin: &Keypair,
    pm_config: Pubkey,
    user: &Pubkey,
    role: u16,
) {
    let (user_perms_pda, _) = user_permissions_pda(user, &pm_config);
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: permission_manager::accounts::GrantRole {
            admin: admin.pubkey(),
            config: pm_config,
            user_permissions: user_perms_pda,
            user: *user,
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: permission_manager::instruction::GrantRole { role }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[admin], blockhash);
    banks.process_transaction(tx).await.unwrap();
}

async fn fund(banks: &mut BanksClient, payer: &Keypair, to: &Pubkey, lamports: u64) {
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let ix = system_instruction::transfer(&payer.pubkey(), to, lamports);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], blockhash);
    banks.process_transaction(tx).await.unwrap();
}

async fn get_redemption_config(banks: &mut BanksClient, pda: Pubkey) -> RedemptionConfig {
    let account = banks.get_account(pda).await.unwrap().unwrap();
    RedemptionConfig::deserialize(&mut &account.data[8..]).unwrap()
}

async fn get_redemption_operation(banks: &mut BanksClient, pda: Pubkey) -> RedemptionOperation {
    let account = banks.get_account(pda).await.unwrap().unwrap();
    RedemptionOperation::deserialize(&mut &account.data[8..]).unwrap()
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_initialize() {
    let (mut banks, admin, pm_config, redemption_cfg) = setup(7200).await;
    let cfg = get_redemption_config(&mut banks, redemption_cfg).await;
    assert_eq!(cfg.deadline_delay, 7200);
    assert_eq!(cfg.permission_manager, pm_config);
}

#[tokio::test]
async fn test_initialize_unauthorized() {
    let ctx = program_test().start_with_context().await;
    let admin = ctx.payer.insecure_clone();
    let non_admin = Keypair::new();
    let mut banks = ctx.banks_client.clone();
    let blockhash = ctx.last_blockhash;

    // Init permission-manager
    let (pm_config, _) = permission_config_pda();
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: permission_manager::accounts::Initialize {
            admin: admin.pubkey(),
            config: pm_config,
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: permission_manager::instruction::Initialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Fund non_admin
    fund(&mut banks, &admin, &non_admin.pubkey(), 1_000_000_000).await;

    // Non-admin tries to initialize redemption
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let (redemption_cfg, _) = redemption_config_pda();
    let ix = Instruction {
        program_id: redemption::ID,
        accounts: redemption::accounts::Initialize {
            admin: non_admin.pubkey(),
            redemption_config: redemption_cfg,
            permission_manager_config: pm_config,
            system_program: solana_sdk::system_program::ID,
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
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}

// ---------------------------------------------------------------------------
// Create vault
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_vault() {
    let (mut banks, admin, pm_config, _redemption_cfg) = setup(3600).await;
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    // Create Token-2022 mint
    create_token2022_mint(&mut banks, &admin, &mint_kp, 6, &admin.pubkey()).await;

    // Create vault ATA for vault_authority
    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Verify vault authority exists
    let account = banks.get_account(vault_auth).await.unwrap().unwrap();
    let va = VaultAuthority::deserialize(&mut &account.data[8..]).unwrap();
    assert!(va.bump > 0);
}

// ---------------------------------------------------------------------------
// on_redeem
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_on_redeem() {
    let (mut banks, admin, pm_config, _redemption_cfg) = setup(3600).await;
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    // Create mint + vault
    create_token2022_mint(&mut banks, &admin, &mint_kp, 6, &admin.pubkey()).await;

    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // on_redeem
    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut banks, &admin, &user.pubkey(), 2_000_000_000).await;
    let amount = 1_000_000u64;
    let salt = 42u64;
    let operation_id = compute_operation_id(&user.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = redemption_operation_pda(&operation_id);

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
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
    banks.process_transaction(tx).await.unwrap();

    // Verify operation
    let op = get_redemption_operation(&mut banks, op_pda).await;
    assert_eq!(op.status, 1); // STATUS_PENDING
    assert_eq!(op.amount, amount);
    assert_eq!(op.user, user.pubkey());
    assert_eq!(op.mint, mint_kp.pubkey());
    assert!(op.deadline > 0);
}

// ---------------------------------------------------------------------------
// on_redeem — invalid operation_id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_on_redeem_invalid_operation_id() {
    let (mut banks, admin, pm_config, _redemption_cfg) = setup(3600).await;
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    create_token2022_mint(&mut banks, &admin, &mint_kp, 6, &admin.pubkey()).await;

    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut banks, &admin, &user.pubkey(), 2_000_000_000).await;

    let wrong_op_id = [0u8; 32];
    let (op_pda, _) = redemption_operation_pda(&wrong_op_id);

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
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
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}

// ---------------------------------------------------------------------------
// on_redeem — no vault (unsupported mint)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_on_redeem_no_vault() {
    let (mut banks, admin, _pm_config, _redemption_cfg) = setup(3600).await;
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    create_token2022_mint(&mut banks, &admin, &mint_kp, 6, &admin.pubkey()).await;
    // Note: NOT calling create_vault

    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut banks, &admin, &user.pubkey(), 2_000_000_000).await;

    let amount = 1000u64;
    let salt = 1u64;
    let operation_id = compute_operation_id(&user.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = redemption_operation_pda(&operation_id);

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
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
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}

// ---------------------------------------------------------------------------
// Execute (burn) — requires REDEMPTION_EXECUTOR role + tokens in vault
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_execute_redemption() {
    let (mut banks, admin, pm_config, _redemption_cfg) = setup(86400).await;
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    // Create mint with vault_authority as the freeze/mint authority won't matter,
    // but we need admin as mint authority to mint tokens into vault
    create_token2022_mint(&mut banks, &admin, &mint_kp, 6, &admin.pubkey()).await;

    // Create vault
    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Mint tokens into vault (simulating a redeem deposit)
    let amount = 1_000_000u64;
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let mint_ix = spl_token_2022::instruction::mint_to(
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
    banks.process_transaction(tx).await.unwrap();

    // Create on_redeem operation
    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut banks, &admin, &user.pubkey(), 2_000_000_000).await;
    let salt = 42u64;
    let operation_id = compute_operation_id(&user.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = redemption_operation_pda(&operation_id);

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
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
    banks.process_transaction(tx).await.unwrap();

    // Grant REDEMPTION_EXECUTOR to burner
    let burner = Keypair::new();
    fund(&mut banks, &admin, &burner.pubkey(), 1_000_000_000).await;
    grant_role(
        &mut banks,
        &admin,
        pm_config,
        &burner.pubkey(),
        permission_manager::constants::ROLE_REDEMPTION_EXECUTOR,
    )
    .await;
    let (burner_perms, _) = user_permissions_pda(&burner.pubkey(), &pm_config);

    // Execute
    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
    banks.process_transaction(tx).await.unwrap();

    // Verify operation done
    let op = get_redemption_operation(&mut banks, op_pda).await;
    assert_eq!(op.status, 2); // STATUS_DONE
}

// ---------------------------------------------------------------------------
// Execute — unauthorized (no REDEMPTION_EXECUTOR)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_execute_unauthorized() {
    let (mut banks, admin, pm_config, _redemption_cfg) = setup(86400).await;
    let mint_kp = Keypair::new();
    let (vault_auth, _) = vault_authority_pda(&mint_kp.pubkey());

    create_token2022_mint(&mut banks, &admin, &mint_kp, 6, &admin.pubkey()).await;

    let vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &vault_auth,
        &mint_kp.pubkey(),
        &TOKEN_2022_PROGRAM_ID,
    );

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
        }
        .to_account_metas(None),
        data: redemption::instruction::CreateVault {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Create operation
    let user = Keypair::new();
    let mint_auth_kp = Keypair::new();
    fund(&mut banks, &admin, &user.pubkey(), 2_000_000_000).await;
    let amount = 1000u64;
    let salt = 1u64;
    let operation_id = compute_operation_id(&user.pubkey(), &mint_kp.pubkey(), amount, salt);
    let (op_pda, _) = redemption_operation_pda(&operation_id);

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
            system_program: solana_sdk::system_program::ID,
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
    banks.process_transaction(tx).await.unwrap();

    // Non-executor tries to execute
    let non_executor = Keypair::new();
    fund(&mut banks, &admin, &non_executor.pubkey(), 1_000_000_000).await;
    // Grant MINTER (not REDEMPTION_EXECUTOR)
    grant_role(
        &mut banks,
        &admin,
        pm_config,
        &non_executor.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    )
    .await;
    let (perms, _) = user_permissions_pda(&non_executor.pubkey(), &pm_config);

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}
