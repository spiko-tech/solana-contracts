use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use permission_manager::state::{PermissionConfig, UserPermissions};
use solana_program_test::*;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, TransactionError},
};

fn program_test() -> ProgramTest {
    ProgramTest::new("permission_manager", permission_manager::ID, None)
}

fn config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"config"], &permission_manager::ID)
}

fn user_permissions_pda(user: &Pubkey, config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"user_permission", user.as_ref(), config.as_ref()],
        &permission_manager::ID,
    )
}

/// Helper: initialize the permission manager and return the context + config PDA.
async fn setup() -> (BanksClient, Keypair, solana_sdk::hash::Hash, Pubkey) {
    let ctx = program_test().start_with_context().await;
    let admin = ctx.payer.insecure_clone();
    let admin_pubkey = admin.pubkey();
    let (config_pda, _) = config_pda();

    let accounts = permission_manager::accounts::Initialize {
        admin: admin_pubkey,
        config: config_pda,
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::Initialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin_pubkey),
        &[&admin],
        ctx.last_blockhash,
    );
    ctx.banks_client
        .to_owned()
        .process_transaction(tx)
        .await
        .unwrap();

    (ctx.banks_client, admin, ctx.last_blockhash, config_pda)
}

async fn get_config(banks: &mut BanksClient, pda: Pubkey) -> PermissionConfig {
    let account = banks.get_account(pda).await.unwrap().unwrap();
    PermissionConfig::deserialize(&mut &account.data[8..]).unwrap()
}

async fn get_user_permissions(banks: &mut BanksClient, pda: Pubkey) -> UserPermissions {
    let account = banks.get_account(pda).await.unwrap().unwrap();
    UserPermissions::deserialize(&mut &account.data[8..]).unwrap()
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_initialize() {
    let (mut banks, admin, _, config) = setup().await;
    let cfg = get_config(&mut banks, config).await;
    assert_eq!(cfg.admin, admin.pubkey());
    assert_eq!(cfg.pending_admin, Pubkey::default());
}

// ---------------------------------------------------------------------------
// Grant role (admin)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_grant_role() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let user = Keypair::new();
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);

    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRole {
            role: permission_manager::constants::ROLE_MINTER,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    let perms = get_user_permissions(&mut banks, user_perms_pda).await;
    assert_eq!(
        perms.roles & permission_manager::constants::ROLE_MINTER,
        permission_manager::constants::ROLE_MINTER
    );
}

#[tokio::test]
async fn test_grant_role_accumulates() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let user = Keypair::new();
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);

    // Grant MINTER
    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRole {
            role: permission_manager::constants::ROLE_MINTER,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Grant PAUSER on top (need fresh blockhash)
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRole {
            role: permission_manager::constants::ROLE_PAUSER,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    let perms = get_user_permissions(&mut banks, user_perms_pda).await;
    assert_eq!(
        perms.roles,
        permission_manager::constants::ROLE_MINTER | permission_manager::constants::ROLE_PAUSER
    );
}

#[tokio::test]
async fn test_grant_role_unauthorized() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let non_admin = Keypair::new();
    let user = Keypair::new();
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);

    // Fund non_admin
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &non_admin.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRole {
        admin: non_admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRole {
            role: permission_manager::constants::ROLE_MINTER,
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
// Revoke role
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_revoke_role() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let user = Keypair::new();
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);

    // Grant MINTER | PAUSER
    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRole {
            role: permission_manager::constants::ROLE_MINTER
                | permission_manager::constants::ROLE_PAUSER,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Revoke MINTER
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::RevokeRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::RevokeRole {
            role: permission_manager::constants::ROLE_MINTER,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    let perms = get_user_permissions(&mut banks, user_perms_pda).await;
    assert_eq!(perms.roles, permission_manager::constants::ROLE_PAUSER);
}

// ---------------------------------------------------------------------------
// Transfer admin + Accept admin
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_transfer_and_accept_admin() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let new_admin = Keypair::new();

    // Fund new_admin
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &new_admin.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Transfer admin
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::TransferAdmin {
        admin: admin.pubkey(),
        config,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::TransferAdmin {
            new_admin: new_admin.pubkey(),
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    let cfg = get_config(&mut banks, config).await;
    assert_eq!(cfg.admin, admin.pubkey()); // still old admin
    assert_eq!(cfg.pending_admin, new_admin.pubkey());

    // Accept admin
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::AcceptAdmin {
        new_admin: new_admin.pubkey(),
        config,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::AcceptAdmin {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&new_admin.pubkey()),
        &[&new_admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    let cfg = get_config(&mut banks, config).await;
    assert_eq!(cfg.admin, new_admin.pubkey());
    assert_eq!(cfg.pending_admin, Pubkey::default());
}

#[tokio::test]
async fn test_accept_admin_wrong_signer() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let new_admin = Keypair::new();
    let imposter = Keypair::new();

    // Fund imposter
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &imposter.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Transfer admin to new_admin
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::TransferAdmin {
        admin: admin.pubkey(),
        config,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::TransferAdmin {
            new_admin: new_admin.pubkey(),
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Imposter tries to accept
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::AcceptAdmin {
        new_admin: imposter.pubkey(),
        config,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::AcceptAdmin {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&imposter.pubkey()),
        &[&imposter],
        blockhash,
    );
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}

// ---------------------------------------------------------------------------
// Grant role whitelister
// ---------------------------------------------------------------------------

/// Helper: admin grants a role to a user.
async fn admin_grant_role(
    banks: &mut BanksClient,
    admin: &Keypair,
    config: Pubkey,
    user: &Pubkey,
    role: u16,
) {
    let (user_perms_pda, _) = user_permissions_pda(user, &config);
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: *user,
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRole { role }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[admin], blockhash);
    banks.process_transaction(tx).await.unwrap();
}

#[tokio::test]
async fn test_whitelister_grants_whitelisted() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &whitelister.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Admin grants WHITELISTER role to whitelister
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    )
    .await;

    // Whitelister grants WHITELISTED to user
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRoleWhitelister {
            role: permission_manager::constants::ROLE_WHITELISTED,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&whitelister.pubkey()),
        &[&whitelister],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    let perms = get_user_permissions(&mut banks, user_perms_pda).await;
    assert_eq!(
        perms.roles & permission_manager::constants::ROLE_WHITELISTED,
        permission_manager::constants::ROLE_WHITELISTED
    );
}

#[tokio::test]
async fn test_whitelister_grants_whitelisted_ext() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &whitelister.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Admin grants WHITELISTER role
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    )
    .await;

    // Whitelister grants WHITELISTED_EXT to user
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRoleWhitelister {
            role: permission_manager::constants::ROLE_WHITELISTED_EXT,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&whitelister.pubkey()),
        &[&whitelister],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    let perms = get_user_permissions(&mut banks, user_perms_pda).await;
    assert_eq!(
        perms.roles & permission_manager::constants::ROLE_WHITELISTED_EXT,
        permission_manager::constants::ROLE_WHITELISTED_EXT
    );
}

#[tokio::test]
async fn test_whitelister_cannot_grant_non_whitelist_role() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &whitelister.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Admin grants WHITELISTER role
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    )
    .await;

    // Whitelister tries to grant MINTER — should fail
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRoleWhitelister {
            role: permission_manager::constants::ROLE_MINTER,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&whitelister.pubkey()),
        &[&whitelister],
        blockhash,
    );
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}

#[tokio::test]
async fn test_whitelister_mutual_exclusion_whitelisted_blocks_whitelisted_ext() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &whitelister.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Admin grants WHITELISTER
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    )
    .await;

    // Admin grants WHITELISTED to user
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &user.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED,
    )
    .await;

    // Whitelister tries to grant WHITELISTED_EXT to user who already has WHITELISTED — should fail
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRoleWhitelister {
            role: permission_manager::constants::ROLE_WHITELISTED_EXT,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&whitelister.pubkey()),
        &[&whitelister],
        blockhash,
    );
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}

#[tokio::test]
async fn test_whitelister_mutual_exclusion_whitelisted_ext_blocks_whitelisted() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &whitelister.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Admin grants WHITELISTER
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    )
    .await;

    // Whitelister grants WHITELISTED_EXT to user
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRoleWhitelister {
            role: permission_manager::constants::ROLE_WHITELISTED_EXT,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&whitelister.pubkey()),
        &[&whitelister],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Whitelister tries to grant WHITELISTED to same user — should fail
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRoleWhitelister {
            role: permission_manager::constants::ROLE_WHITELISTED,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&whitelister.pubkey()),
        &[&whitelister],
        blockhash,
    );
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}

#[tokio::test]
async fn test_admin_bypasses_mutual_exclusion() {
    let (mut banks, admin, _, config) = setup().await;
    let user = Keypair::new();

    // Admin grants WHITELISTED
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &user.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED,
    )
    .await;

    // Admin grants WHITELISTED_EXT on top — should succeed (no exclusion for admin)
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &user.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED_EXT,
    )
    .await;

    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    let perms = get_user_permissions(&mut banks, user_perms_pda).await;
    assert_eq!(
        perms.roles,
        permission_manager::constants::ROLE_WHITELISTED
            | permission_manager::constants::ROLE_WHITELISTED_EXT
    );
}

#[tokio::test]
async fn test_non_whitelister_cannot_use_grant_role_whitelister() {
    let (mut banks, admin, blockhash, config) = setup().await;
    let random_user = Keypair::new();
    let target = Keypair::new();

    // Fund random_user
    let transfer_ix = solana_sdk::system_instruction::transfer(
        &admin.pubkey(),
        &random_user.pubkey(),
        1_000_000_000,
    );
    let tx = Transaction::new_signed_with_payer(
        &[transfer_ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash,
    );
    banks.process_transaction(tx).await.unwrap();

    // Grant MINTER to random_user (not WHITELISTER)
    admin_grant_role(
        &mut banks,
        &admin,
        config,
        &random_user.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    )
    .await;

    // random_user tries grant_role_whitelister — should fail (constraint: ROLE_WHITELISTER not set)
    let (caller_perms_pda, _) = user_permissions_pda(&random_user.pubkey(), &config);
    let (target_perms_pda, _) = user_permissions_pda(&target.pubkey(), &config);
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: random_user.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: target_perms_pda,
        user: target.pubkey(),
        system_program: solana_sdk::system_program::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRoleWhitelister {
            role: permission_manager::constants::ROLE_WHITELISTED,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&random_user.pubkey()),
        &[&random_user],
        blockhash,
    );
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}
