mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_redeem_should_burn_tokens_from_vault() {
    let (mut svm, _payer, _admin, redemption_auth, mint, vault) = setup_with_vault(1_000_000);

    let ix = ix_redeem(
        &redemption_auth.pubkey(),
        &mint,
        &vault,
        &redemption_auth.pubkey(),
        1,       // salt
        500_000, // amount
    );
    send_tx(&mut svm, &[ix], &redemption_auth, &[&redemption_auth]).unwrap();

    // Vault balance should decrease
    assert_eq!(read_token_balance(&svm, &vault), Some(500_000));

    // RedemptionRecord should exist
    let (record, _) = redemption_record_pda(1);
    assert!(svm.get_account(&record).is_some());
}

#[test]
fn test_redeem_should_fail_if_signer_is_not_redemption_authority() {
    let (mut svm, _payer, _admin, _redemption_auth, mint, vault) = setup_with_vault(1_000_000);
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_redeem(&fake.pubkey(), &mint, &vault, &fake.pubkey(), 1, 500_000);
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}

#[test]
fn test_redeem_should_fail_if_amount_is_zero() {
    let (mut svm, _payer, _admin, redemption_auth, mint, vault) = setup_with_vault(1_000_000);

    let ix = ix_redeem(
        &redemption_auth.pubkey(),
        &mint,
        &vault,
        &redemption_auth.pubkey(),
        1,
        0, // zero amount
    );
    let result = send_tx(&mut svm, &[ix], &redemption_auth, &[&redemption_auth]);
    assert!(result.is_err());
}

#[test]
fn test_redeem_should_fail_if_salt_already_used() {
    let (mut svm, _payer, _admin, redemption_auth, mint, vault) = setup_with_vault(1_000_000);

    let ix = ix_redeem(
        &redemption_auth.pubkey(),
        &mint,
        &vault,
        &redemption_auth.pubkey(),
        1,
        100_000,
    );
    send_tx(&mut svm, &[ix], &redemption_auth, &[&redemption_auth]).unwrap();

    advance_slot(&mut svm);

    // Same salt again
    let ix = ix_redeem(
        &redemption_auth.pubkey(),
        &mint,
        &vault,
        &redemption_auth.pubkey(),
        1,
        100_000,
    );
    let result = send_tx(&mut svm, &[ix], &redemption_auth, &[&redemption_auth]);
    assert!(result.is_err());
}
