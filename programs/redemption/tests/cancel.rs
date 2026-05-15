mod common;

use common::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_cancel_should_transfer_tokens_back_to_user() {
    let (mut svm, _payer, admin, _redemption_auth, mint, vault) = setup_with_vault(1_000_000);

    // Create user token account
    let user = Keypair::new();
    let user_token_pub = Pubkey::new_unique();
    create_fake_token_account(&mut svm, &user_token_pub, &mint, &user.pubkey(), 0);

    let ix = ix_cancel(
        &admin.pubkey(),
        &mint,
        &vault,
        &user_token_pub,
        &admin.pubkey(),
        1,       // salt
        500_000, // amount
        user.pubkey(),
    );
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Vault balance should decrease
    assert_eq!(read_token_balance(&svm, &vault), Some(500_000));
    // User balance should increase
    assert_eq!(read_token_balance(&svm, &user_token_pub), Some(500_000));

    // RedemptionRecord should exist
    let (record, _) = redemption_record_pda(1);
    assert!(svm.get_account(&record).is_some());
}

#[test]
fn test_cancel_should_fail_if_signer_is_not_admin() {
    let (mut svm, _payer, _admin, _redemption_auth, mint, vault) = setup_with_vault(1_000_000);
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let user = Keypair::new();
    let user_token_pub = Pubkey::new_unique();
    create_fake_token_account(&mut svm, &user_token_pub, &mint, &user.pubkey(), 0);

    let ix = ix_cancel(
        &fake.pubkey(),
        &mint,
        &vault,
        &user_token_pub,
        &fake.pubkey(),
        1,
        500_000,
        user.pubkey(),
    );
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}

#[test]
fn test_cancel_should_fail_if_user_token_account_owner_mismatch() {
    let (mut svm, _payer, admin, _redemption_auth, mint, vault) = setup_with_vault(1_000_000);

    let real_user = Pubkey::new_unique();
    let wrong_owner = Pubkey::new_unique();
    let bad_token_pub = Pubkey::new_unique();
    create_fake_token_account(&mut svm, &bad_token_pub, &mint, &wrong_owner, 0);

    let ix = ix_cancel(
        &admin.pubkey(),
        &mint,
        &vault,
        &bad_token_pub,
        &admin.pubkey(),
        10,
        500_000,
        real_user,
    );
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}

#[test]
fn test_cancel_should_fail_if_amount_is_zero() {
    let (mut svm, _payer, admin, _redemption_auth, mint, vault) = setup_with_vault(1_000_000);

    let user = Keypair::new();
    let user_token_pub = Pubkey::new_unique();
    create_fake_token_account(&mut svm, &user_token_pub, &mint, &user.pubkey(), 0);

    let ix = ix_cancel(
        &admin.pubkey(),
        &mint,
        &vault,
        &user_token_pub,
        &admin.pubkey(),
        1,
        0, // zero amount
        user.pubkey(),
    );
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}

#[test]
fn test_cancel_should_fail_if_salt_already_used() {
    let (mut svm, _payer, admin, _redemption_auth, mint, vault) = setup_with_vault(1_000_000);

    let user = Keypair::new();
    let user_token_pub = Pubkey::new_unique();
    create_fake_token_account(&mut svm, &user_token_pub, &mint, &user.pubkey(), 0);

    let ix = ix_cancel(
        &admin.pubkey(),
        &mint,
        &vault,
        &user_token_pub,
        &admin.pubkey(),
        1,
        100_000,
        user.pubkey(),
    );
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);

    // Same salt again
    let ix = ix_cancel(
        &admin.pubkey(),
        &mint,
        &vault,
        &user_token_pub,
        &admin.pubkey(),
        1,
        100_000,
        user.pubkey(),
    );
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
