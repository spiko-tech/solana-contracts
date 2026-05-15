mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_whitelist_should_set_role_to_whitelisted() {
    let (mut svm, _payer, whitelist_auth, _admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        false,
    );
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_WHITELISTED)
    );
}

#[test]
fn test_whitelist_should_set_role_to_whitelisted_ext() {
    let (mut svm, _payer, whitelist_auth, _admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        true,
    );
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_WHITELISTED_EXT)
    );
}

#[test]
fn test_whitelist_should_fail_if_signer_is_not_whitelist_authority() {
    let (mut svm, _payer, _whitelist_auth, _admin) = setup_initialized();
    let wallet = Keypair::new();
    let fake_auth = Keypair::new();
    svm.airdrop(&fake_auth.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_whitelist(
        &fake_auth.pubkey(),
        &wallet.pubkey(),
        &fake_auth.pubkey(),
        false,
    );
    let result = send_tx(&mut svm, &[ix], &fake_auth, &[&fake_auth]);
    assert!(result.is_err());
}

#[test]
fn test_whitelist_should_fail_if_ext_wallet_switches_to_normal() {
    let (mut svm, _payer, whitelist_auth, _admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        true,
    );
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        false,
    );
    let result = send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]);
    assert!(result.is_err());
}

#[test]
fn test_whitelist_should_fail_if_normal_wallet_switches_to_ext() {
    let (mut svm, _payer, whitelist_auth, _admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        false,
    );
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        true,
    );
    let result = send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]);
    assert!(result.is_err());
}

#[test]
fn test_whitelist_should_re_whitelist_after_unwhitelist_in_same_group() {
    let (mut svm, _payer, whitelist_auth, _admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        false,
    );
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_unwhitelist(&whitelist_auth.pubkey(), &wallet.pubkey());
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();
    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_UNWHITELISTED)
    );

    advance_slot(&mut svm);

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        false,
    );
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();
    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_WHITELISTED)
    );
}
