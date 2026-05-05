use anchor_lang::{AnchorDeserialize, InstructionData, ToAccountMetas};
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

const TOKEN_2022_PROGRAM_ID: Pubkey = spl_token_2022::ID;

fn program_test() -> ProgramTest {
    let mut pt = ProgramTest::new("spiko_token", spiko_token::ID, None);
    pt.add_program("permission_manager", permission_manager::ID, None);
    pt.add_program("redemption", redemption::ID, None);
    pt
}

async fn create_token2022_mint_with_permanent_delegate(
    banks: &mut BanksClient,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
    permanent_delegate: &Pubkey,
) {
    let blockhash = banks.get_latest_blockhash().await.unwrap();

    // Calculate space with permanent delegate extension
    let extensions = [spl_token_2022::extension::ExtensionType::PermanentDelegate];
    let space = spl_token_2022::extension::ExtensionType::try_calculate_account_len::<
        spl_token_2022::state::Mint,
    >(&extensions)
    .unwrap();
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
        spl_token_2022::instruction::initialize_permanent_delegate(
            &TOKEN_2022_PROGRAM_ID,
            &mint.pubkey(),
            permanent_delegate,
        )
        .unwrap(),
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

/// Full setup: PM init, spiko-token init, mint created with mint_authority as the authority.
/// Returns (banks, admin, pm_config, mint, token_config_pda, mint_authority_pda).
async fn setup(decimals: u8) -> (BanksClient, Keypair, Pubkey, Pubkey, Pubkey, Pubkey) {
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

    // Create Token-2022 mint with permanent delegate (mint_authority PDA)
    // Mint authority is set to admin temporarily; spiko-token::initialize will
    // transfer it to the MintAuthority PDA via SetAuthority.
    let mint_kp = Keypair::new();
    let (mint_auth_pda, _) = mint_authority_pda(&mint_kp.pubkey());
    create_token2022_mint_with_permanent_delegate(
        &mut banks,
        &admin,
        &mint_kp,
        decimals,
        &admin.pubkey(), // admin as temporary mint authority
        &mint_auth_pda,
    )
    .await;

    // Init spiko-token
    let (tok_config, _) = token_config_pda(&mint_kp.pubkey());
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::Initialize {
            admin: admin.pubkey(),
            mint: mint_kp.pubkey(),
            token_config: tok_config,
            mint_authority: mint_auth_pda,
            permission_manager_config: pm_config,
            system_program: solana_sdk::system_program::ID,
            token_program: TOKEN_2022_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Initialize {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    (
        banks,
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

#[tokio::test]
async fn test_initialize() {
    let (mut banks, admin, pm_config, mint, tok_config, _mint_auth) = setup(6).await;
    let account = banks.get_account(tok_config).await.unwrap().unwrap();
    let cfg = TokenConfig::deserialize(&mut &account.data[8..]).unwrap();
    assert!(!cfg.paused);
    assert_eq!(cfg.mint, mint);
    assert_eq!(cfg.permission_manager, pm_config);
}

// ---------------------------------------------------------------------------
// Mint to
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mint_to() {
    let (mut banks, admin, pm_config, mint, tok_config, mint_auth) = setup(6).await;

    // Grant MINTER role to admin
    grant_role(
        &mut banks,
        &admin,
        pm_config,
        &admin.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    )
    .await;
    let (minter_perms, _) = user_permissions_pda(&admin.pubkey(), &pm_config);

    // Create destination ATA
    let recipient = Keypair::new();
    fund(&mut banks, &admin, &recipient.pubkey(), 1_000_000_000).await;
    let dest_ata = create_ata(&mut banks, &admin, &mint, &recipient.pubkey()).await;

    let amount = 5_000_000u64;
    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Mint { amount }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Verify balance
    let account = banks.get_account(dest_ata).await.unwrap().unwrap();
    let token_account = spl_token_2022::state::Account::unpack_from_slice(&account.data).unwrap();
    assert_eq!(token_account.amount, amount);
}

// ---------------------------------------------------------------------------
// Mint to — unauthorized
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_mint_to_unauthorized() {
    let (mut banks, admin, pm_config, mint, tok_config, mint_auth) = setup(6).await;

    // Grant PAUSER (not MINTER) to user
    let user = Keypair::new();
    fund(&mut banks, &admin, &user.pubkey(), 1_000_000_000).await;
    grant_role(
        &mut banks,
        &admin,
        pm_config,
        &user.pubkey(),
        permission_manager::constants::ROLE_PAUSER,
    )
    .await;
    let (user_perms, _) = user_permissions_pda(&user.pubkey(), &pm_config);

    let dest_ata = create_ata(&mut banks, &admin, &mint, &user.pubkey()).await;

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Mint { amount: 1000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&user.pubkey()), &[&user], blockhash);
    let err = banks.process_transaction(tx).await.unwrap_err();
    assert!(matches!(
        err,
        BanksClientError::TransactionError(TransactionError::InstructionError(..))
    ));
}

// ---------------------------------------------------------------------------
// Burn
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_burn() {
    let (mut banks, admin, pm_config, mint, tok_config, mint_auth) = setup(6).await;

    // Grant MINTER + BURNER
    grant_role(
        &mut banks,
        &admin,
        pm_config,
        &admin.pubkey(),
        permission_manager::constants::ROLE_MINTER | permission_manager::constants::ROLE_BURNER,
    )
    .await;
    let (admin_perms, _) = user_permissions_pda(&admin.pubkey(), &pm_config);

    // Mint some tokens
    let dest = create_ata(&mut banks, &admin, &mint, &admin.pubkey()).await;
    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Mint { amount: 1000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Burn
    let blockhash = banks.get_latest_blockhash().await.unwrap();
    let ix = Instruction {
        program_id: spiko_token::ID,
        accounts: spiko_token::accounts::BurnTokens {
            burner: admin.pubkey(),
            token_config: tok_config,
            mint,
            source: dest,
            burner_permissions: admin_perms,
            token_program: TOKEN_2022_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Burn { amount: 400 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Verify balance
    let account = banks.get_account(dest).await.unwrap().unwrap();
    let token_account = spl_token_2022::state::Account::unpack_from_slice(&account.data).unwrap();
    assert_eq!(token_account.amount, 600);
}

// ---------------------------------------------------------------------------
// Burn from (admin)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_burn_from() {
    let (mut banks, admin, pm_config, mint, tok_config, mint_auth) = setup(6).await;

    // Grant MINTER to mint tokens
    grant_role(
        &mut banks,
        &admin,
        pm_config,
        &admin.pubkey(),
        permission_manager::constants::ROLE_MINTER,
    )
    .await;
    let (admin_perms, _) = user_permissions_pda(&admin.pubkey(), &pm_config);

    // Mint tokens to a user's account
    let user = Keypair::new();
    fund(&mut banks, &admin, &user.pubkey(), 1_000_000_000).await;
    let user_ata = create_ata(&mut banks, &admin, &mint, &user.pubkey()).await;

    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
        }
        .to_account_metas(None),
        data: spiko_token::instruction::Mint { amount: 1000 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    // Admin burn from user's account
    let blockhash = banks.get_latest_blockhash().await.unwrap();
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
        }
        .to_account_metas(None),
        data: spiko_token::instruction::BurnFrom { amount: 300 }.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&admin.pubkey()), &[&admin], blockhash);
    banks.process_transaction(tx).await.unwrap();

    let account = banks.get_account(user_ata).await.unwrap().unwrap();
    let token_account = spl_token_2022::state::Account::unpack_from_slice(&account.data).unwrap();
    assert_eq!(token_account.amount, 700);
}
