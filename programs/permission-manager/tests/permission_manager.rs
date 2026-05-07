use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
use litesvm::LiteSVM;
use permission_manager::state::{PermissionConfig, UserPermissions};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, TransactionError},
};
use solana_sdk_ids::system_program;
use solana_system_interface::instruction as system_instruction;

fn config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"config"], &permission_manager::ID)
}

fn user_permissions_pda(user: &Pubkey, config: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"user_permission", user.as_ref(), config.as_ref()],
        &permission_manager::ID,
    )
}

/// Helper: initialize the permission manager and return the SVM + admin + config PDA.
fn setup() -> (LiteSVM, Keypair, Pubkey) {
    let mut svm = LiteSVM::new().with_default_programs();
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

    let admin_pubkey = admin.pubkey();
    let (config_pda, _) = config_pda();

    let accounts = permission_manager::accounts::Initialize {
        admin: admin_pubkey,
        config: config_pda,
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    (svm, admin, config_pda)
}

fn get_config(svm: &LiteSVM, pda: Pubkey) -> PermissionConfig {
    let account = svm.get_account(&pda).unwrap();
    PermissionConfig::deserialize(&mut &account.data[8..]).unwrap()
}

fn get_user_permissions(svm: &LiteSVM, pda: Pubkey) -> UserPermissions {
    let account = svm.get_account(&pda).unwrap();
    UserPermissions::deserialize(&mut &account.data[8..]).unwrap()
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

#[test]
fn test_initialize() {
    let (svm, admin, config) = setup();
    let cfg = get_config(&svm, config);
    assert_eq!(cfg.admin, admin.pubkey());
    assert_eq!(cfg.pending_admin, Pubkey::default());
}

// ---------------------------------------------------------------------------
// Grant role (admin)
// ---------------------------------------------------------------------------

#[test]
fn test_grant_role() {
    let (mut svm, admin, config) = setup();
    let user = Keypair::new();
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);

    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let perms = get_user_permissions(&svm, user_perms_pda);
    assert_eq!(
        perms.roles & permission_manager::constants::ROLE_MINTER,
        permission_manager::constants::ROLE_MINTER
    );
}

#[test]
fn test_grant_role_accumulates() {
    let (mut svm, admin, config) = setup();
    let user = Keypair::new();
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);

    // Grant MINTER
    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Grant PAUSER on top
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRole {
            role: permission_manager::constants::ROLE_PAUSER,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let perms = get_user_permissions(&svm, user_perms_pda);
    assert_eq!(
        perms.roles,
        permission_manager::constants::ROLE_MINTER | permission_manager::constants::ROLE_PAUSER
    );
}

#[test]
fn test_grant_role_unauthorized() {
    let (mut svm, admin, config) = setup();
    let non_admin = Keypair::new();
    let user = Keypair::new();
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);

    // Fund non_admin
    svm.airdrop(&non_admin.pubkey(), 1_000_000_000).unwrap();

    let accounts = permission_manager::accounts::GrantRole {
        admin: non_admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Revoke role
// ---------------------------------------------------------------------------

#[test]
fn test_revoke_role() {
    let (mut svm, admin, config) = setup();
    let user = Keypair::new();
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);

    // Grant MINTER | PAUSER
    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Revoke MINTER
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::RevokeRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::RevokeRole {
            role: permission_manager::constants::ROLE_MINTER,
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let perms = get_user_permissions(&svm, user_perms_pda);
    assert_eq!(perms.roles, permission_manager::constants::ROLE_PAUSER);
}

// ---------------------------------------------------------------------------
// Transfer admin + Accept admin
// ---------------------------------------------------------------------------

#[test]
fn test_transfer_and_accept_admin() {
    let (mut svm, admin, config) = setup();
    let new_admin = Keypair::new();

    // Fund new_admin
    svm.airdrop(&new_admin.pubkey(), 1_000_000_000).unwrap();

    // Transfer admin
    let accounts = permission_manager::accounts::TransferAdmin {
        admin: admin.pubkey(),
        config,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::TransferAdmin {
            new_admin: new_admin.pubkey(),
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let cfg = get_config(&svm, config);
    assert_eq!(cfg.admin, admin.pubkey()); // still old admin
    assert_eq!(cfg.pending_admin, new_admin.pubkey());

    // Accept admin
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::AcceptAdmin {
        new_admin: new_admin.pubkey(),
        config,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let cfg = get_config(&svm, config);
    assert_eq!(cfg.admin, new_admin.pubkey());
    assert_eq!(cfg.pending_admin, Pubkey::default());
}

#[test]
fn test_accept_admin_wrong_signer() {
    let (mut svm, admin, config) = setup();
    let new_admin = Keypair::new();
    let imposter = Keypair::new();

    // Fund imposter
    svm.airdrop(&imposter.pubkey(), 1_000_000_000).unwrap();

    // Transfer admin to new_admin
    let accounts = permission_manager::accounts::TransferAdmin {
        admin: admin.pubkey(),
        config,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::TransferAdmin {
            new_admin: new_admin.pubkey(),
        }
        .data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Imposter tries to accept
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::AcceptAdmin {
        new_admin: imposter.pubkey(),
        config,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Grant role whitelister
// ---------------------------------------------------------------------------

/// Helper: admin grants a role to a user.
fn admin_grant_role(svm: &mut LiteSVM, admin: &Keypair, config: Pubkey, user: &Pubkey, role: u16) {
    let (user_perms_pda, _) = user_permissions_pda(user, &config);
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRole {
        admin: admin.pubkey(),
        config,
        user_permissions: user_perms_pda,
        user: *user,
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
    };
    let ix = Instruction {
        program_id: permission_manager::ID,
        accounts: accounts.to_account_metas(None),
        data: permission_manager::instruction::GrantRole { role }.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[admin],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();
}

#[test]
fn test_whitelister_grants_whitelisted() {
    let (mut svm, admin, config) = setup();
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    svm.airdrop(&whitelister.pubkey(), 1_000_000_000).unwrap();

    // Admin grants WHITELISTER role to whitelister
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    );

    // Whitelister grants WHITELISTED to user
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let perms = get_user_permissions(&svm, user_perms_pda);
    assert_eq!(
        perms.roles & permission_manager::constants::ROLE_WHITELISTED,
        permission_manager::constants::ROLE_WHITELISTED
    );
}

#[test]
fn test_whitelister_grants_whitelisted_ext() {
    let (mut svm, admin, config) = setup();
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    svm.airdrop(&whitelister.pubkey(), 1_000_000_000).unwrap();

    // Admin grants WHITELISTER role
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    );

    // Whitelister grants WHITELISTED_EXT to user
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    let perms = get_user_permissions(&svm, user_perms_pda);
    assert_eq!(
        perms.roles & permission_manager::constants::ROLE_WHITELISTED_EXT,
        permission_manager::constants::ROLE_WHITELISTED_EXT
    );
}

#[test]
fn test_whitelister_cannot_grant_non_whitelist_role() {
    let (mut svm, admin, config) = setup();
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    svm.airdrop(&whitelister.pubkey(), 1_000_000_000).unwrap();

    // Admin grants WHITELISTER role
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    );

    // Whitelister tries to grant MINTER — should fail
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

#[test]
fn test_whitelister_mutual_exclusion_whitelisted_blocks_whitelisted_ext() {
    let (mut svm, admin, config) = setup();
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    svm.airdrop(&whitelister.pubkey(), 1_000_000_000).unwrap();

    // Admin grants WHITELISTER
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    );

    // Admin grants WHITELISTED to user
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &user.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED,
    );

    // Whitelister tries to grant WHITELISTED_EXT to user who already has WHITELISTED — should fail
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

#[test]
fn test_whitelister_mutual_exclusion_whitelisted_ext_blocks_whitelisted() {
    let (mut svm, admin, config) = setup();
    let whitelister = Keypair::new();
    let user = Keypair::new();

    // Fund whitelister
    svm.airdrop(&whitelister.pubkey(), 1_000_000_000).unwrap();

    // Admin grants WHITELISTER
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &whitelister.pubkey(),
        permission_manager::constants::ROLE_WHITELISTER,
    );

    // Whitelister grants WHITELISTED_EXT to user
    let (caller_perms_pda, _) = user_permissions_pda(&whitelister.pubkey(), &config);
    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    // Whitelister tries to grant WHITELISTED to same user — should fail
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: whitelister.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: user_perms_pda,
        user: user.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

#[test]
fn test_admin_bypasses_mutual_exclusion() {
    let (mut svm, admin, config) = setup();
    let user = Keypair::new();

    // Admin grants WHITELISTED
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &user.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED,
    );

    // Admin grants WHITELISTED_EXT on top — should succeed (no exclusion for admin)
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &user.pubkey(),
        permission_manager::constants::ROLE_WHITELISTED_EXT,
    );

    let (user_perms_pda, _) = user_permissions_pda(&user.pubkey(), &config);
    let perms = get_user_permissions(&svm, user_perms_pda);
    assert_eq!(
        perms.roles,
        permission_manager::constants::ROLE_WHITELISTED
            | permission_manager::constants::ROLE_WHITELISTED_EXT
    );
}

#[test]
fn test_non_whitelister_cannot_use_grant_role_whitelister() {
    let (mut svm, admin, config) = setup();
    let random_user = Keypair::new();
    let target = Keypair::new();

    // Fund random_user
    svm.airdrop(&random_user.pubkey(), 1_000_000_000).unwrap();

    // Grant MINTER to random_user (not WHITELISTER)
    admin_grant_role(
        &mut svm,
        &admin,
        config,
        &random_user.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    );

    // random_user tries grant_role_whitelister — should fail (constraint: ROLE_WHITELISTER not set)
    let (caller_perms_pda, _) = user_permissions_pda(&random_user.pubkey(), &config);
    let (target_perms_pda, _) = user_permissions_pda(&target.pubkey(), &config);
    svm.expire_blockhash();
    let accounts = permission_manager::accounts::GrantRoleWhitelister {
        caller: random_user.pubkey(),
        config,
        caller_permissions: caller_perms_pda,
        user_permissions: target_perms_pda,
        user: target.pubkey(),
        system_program: system_program::ID,
        event_authority: Pubkey::find_program_address(
            &[b"__event_authority"],
            &permission_manager::ID,
        )
        .0,
        program: permission_manager::ID,
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
        svm.latest_blockhash(),
    );
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}
