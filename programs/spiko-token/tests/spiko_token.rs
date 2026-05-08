use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
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
use spiko_token::state::TokenConfig;

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

fn token_config_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"token_config", mint.as_ref()], &spiko_token::ID)
}

fn mint_authority_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"mint_authority", mint.as_ref()], &spiko_token::ID)
}

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

const TOKEN_2022_PROGRAM_ID: Pubkey = spl_token_2022_interface::ID;

fn create_token2022_mint_with_permanent_delegate(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
    permanent_delegate: &Pubkey,
) {
    let blockhash = svm.latest_blockhash();

    // Calculate space with permanent delegate extension
    let extensions = [spl_token_2022_interface::extension::ExtensionType::PermanentDelegate];
    let space = spl_token_2022_interface::extension::ExtensionType::try_calculate_account_len::<
        spl_token_2022_interface::state::Mint,
    >(&extensions)
    .unwrap();
    let lamports = svm.minimum_balance_for_rent_exemption(space);

    let ix = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            lamports,
            space as u64,
            &TOKEN_2022_PROGRAM_ID,
        ),
        spl_token_2022_interface::instruction::initialize_permanent_delegate(
            &TOKEN_2022_PROGRAM_ID,
            &mint.pubkey(),
            permanent_delegate,
        )
        .unwrap(),
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

/// Full setup: PM init, spiko-token init, mint created with mint_authority as the authority.
/// Returns (svm, admin, pm_config, mint, token_config_pda, mint_authority_pda).
fn setup(decimals: u8) -> (LiteSVM, Keypair, Pubkey, Pubkey, Pubkey, Pubkey) {
    let mut svm = LiteSVM::new().with_default_programs();
    svm.add_program_from_file(
        spiko_token::ID,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/spiko_token.so"
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
    svm.add_program_from_file(
        redemption::ID,
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/deploy/redemption.so"
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

    // Create Token-2022 mint with permanent delegate (mint_authority PDA)
    // Mint authority is set to admin temporarily; spiko-token::initialize will
    // transfer it to the MintAuthority PDA via SetAuthority.
    let mint_kp = Keypair::new();
    let (mint_auth_pda, _) = mint_authority_pda(&mint_kp.pubkey());
    create_token2022_mint_with_permanent_delegate(
        &mut svm,
        &admin,
        &mint_kp,
        decimals,
        &admin.pubkey(), // admin as temporary mint authority
        &mint_auth_pda,
    );

    // Init spiko-token
    let (tok_config, _) = token_config_pda(&mint_kp.pubkey());
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::Initialize {
            admin: admin.pubkey(),
            mint: mint_kp.pubkey(),
            token_config: tok_config,
            mint_authority: mint_auth_pda,
            permission_manager_config: pm_config,
            system_program: system_program::ID,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
            program: spiko_token::ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Initialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    (
        svm,
        admin,
        pm_config,
        mint_kp.pubkey(),
        tok_config,
        mint_auth_pda,
    )
}

// ---------------------------------------------------------------------------
// Initialize
// ---------------------------------------------------------------------------

#[test]
fn test_initialize() {
    let (svm, _admin, pm_config, mint, tok_config, _mint_auth) = setup(6);
    let account = svm.get_account(&tok_config).unwrap();
    let cfg = TokenConfig::deserialize(&mut &account.data[8..]).unwrap();
    assert!(!cfg.paused);
    assert_eq!(cfg.mint, mint);
    assert_eq!(cfg.permission_manager, pm_config);
}

// ---------------------------------------------------------------------------
// Mint to
// ---------------------------------------------------------------------------

#[test]
fn test_mint_to() {
    let (mut svm, admin, pm_config, mint, tok_config, mint_auth) = setup(6);

    // Grant MINTER role to admin
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &admin.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    );
    let (minter_perms, _) = user_permissions_pda(&admin.pubkey(), &pm_config);

    // Create destination ATA
    let recipient = Keypair::new();
    fund(&mut svm, &admin, &recipient.pubkey(), 1_000_000_000);
    let dest_ata = create_ata(&mut svm, &admin, &mint, &recipient.pubkey());

    let amount = 5_000_000u64;
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::MintTokens {
            minter: admin.pubkey(),
            token_config: tok_config,
            mint,
            destination: dest_ata,
            mint_authority: mint_auth,
            minter_permissions: minter_perms,
            permission_manager_config: pm_config,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
            program: spiko_token::ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Mint { amount }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Verify balance
    let account = svm.get_account(&dest_ata).unwrap();
    let token_account =
        spl_token_2022_interface::state::Account::unpack_from_slice(&account.data).unwrap();
    assert_eq!(token_account.amount, amount);
}

// ---------------------------------------------------------------------------
// Mint to — unauthorized
// ---------------------------------------------------------------------------

#[test]
fn test_mint_to_unauthorized() {
    let (mut svm, admin, pm_config, mint, tok_config, mint_auth) = setup(6);

    // Grant PAUSER (not MINTER) to user
    let user = Keypair::new();
    fund(&mut svm, &admin, &user.pubkey(), 1_000_000_000);
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &user.pubkey(),
        permission_manager::constants::ROLE_PAUSER,
    );
    let (user_perms, _) = user_permissions_pda(&user.pubkey(), &pm_config);

    let dest_ata = create_ata(&mut svm, &admin, &mint, &user.pubkey());

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::MintTokens {
            minter: user.pubkey(),
            token_config: tok_config,
            mint,
            destination: dest_ata,
            mint_authority: mint_auth,
            minter_permissions: user_perms,
            permission_manager_config: pm_config,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
            program: spiko_token::ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Mint { amount: 1000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
    let err = svm.send_transaction(tx).unwrap_err();
    assert!(matches!(err.err, TransactionError::InstructionError(..)));
}

// ---------------------------------------------------------------------------
// Burn
// ---------------------------------------------------------------------------

#[test]
fn test_burn() {
    let (mut svm, admin, pm_config, mint, tok_config, mint_auth) = setup(6);

    // Grant MINTER + BURNER
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &admin.pubkey(),
        permission_manager::constants::ROLE_MINTER | permission_manager::constants::ROLE_BURNER,
    );
    let (admin_perms, _) = user_permissions_pda(&admin.pubkey(), &pm_config);

    // Mint some tokens
    let dest = create_ata(&mut svm, &admin, &mint, &admin.pubkey());
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::MintTokens {
            minter: admin.pubkey(),
            token_config: tok_config,
            mint,
            destination: dest,
            mint_authority: mint_auth,
            minter_permissions: admin_perms,
            permission_manager_config: pm_config,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
            program: spiko_token::ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Mint { amount: 1000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Burn
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::BurnTokens {
            burner: admin.pubkey(),
            token_config: tok_config,
            mint,
            source: dest,
            burner_permissions: admin_perms,
            permission_manager_config: pm_config,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
            program: spiko_token::ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Burn { amount: 400 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Verify balance
    let account = svm.get_account(&dest).unwrap();
    let token_account =
        spl_token_2022_interface::state::Account::unpack_from_slice(&account.data).unwrap();
    assert_eq!(token_account.amount, 600);
}

// ---------------------------------------------------------------------------
// Burn from (admin)
// ---------------------------------------------------------------------------

#[test]
fn test_burn_from() {
    let (mut svm, admin, pm_config, mint, tok_config, mint_auth) = setup(6);

    // Grant MINTER to mint tokens
    grant_role(
        &mut svm,
        &admin,
        pm_config,
        &admin.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    );
    let (admin_perms, _) = user_permissions_pda(&admin.pubkey(), &pm_config);

    // Mint tokens to a user's account
    let user = Keypair::new();
    fund(&mut svm, &admin, &user.pubkey(), 1_000_000_000);
    let user_ata = create_ata(&mut svm, &admin, &mint, &user.pubkey());

    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::MintTokens {
            minter: admin.pubkey(),
            token_config: tok_config,
            mint,
            destination: user_ata,
            mint_authority: mint_auth,
            minter_permissions: admin_perms,
            permission_manager_config: pm_config,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
            program: spiko_token::ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Mint { amount: 1000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    // Admin burn from user's account
    let blockhash = svm.latest_blockhash();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::BurnFrom {
            admin: admin.pubkey(),
            token_config: tok_config,
            mint,
            source: user_ata,
            permission_manager_config: pm_config,
            mint_authority: mint_auth,
            token_program: TOKEN_2022_PROGRAM_ID,
            event_authority: Pubkey::find_program_address(
                &[b"__event_authority"],
                &spiko_token::ID,
            )
            .0,
            program: spiko_token::ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::BurnFrom { amount: 300 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    svm.send_transaction(tx).unwrap();

    let account = svm.get_account(&user_ata).unwrap();
    let token_account =
        spl_token_2022_interface::state::Account::unpack_from_slice(&account.data).unwrap();
    assert_eq!(token_account.amount, 700);
}
