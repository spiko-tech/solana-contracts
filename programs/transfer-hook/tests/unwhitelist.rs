mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_unwhitelist_should_set_role_to_unwhitelisted() {
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
}

#[test]
fn test_unwhitelist_should_set_role_to_unwhitelisted_ext() {
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

    let ix = ix_unwhitelist(&whitelist_auth.pubkey(), &wallet.pubkey());
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_UNWHITELISTED_EXT)
    );
}

#[test]
fn test_unwhitelist_should_fail_if_signer_is_not_whitelist_authority() {
    let (mut svm, _payer, whitelist_auth, _admin) = setup_initialized();
    let wallet = Keypair::new();
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        false,
    );
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_unwhitelist(&fake.pubkey(), &wallet.pubkey());
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}

#[test]
fn test_unwhitelist_should_fail_if_already_unwhitelisted() {
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

    advance_slot(&mut svm);

    let ix = ix_unwhitelist(&whitelist_auth.pubkey(), &wallet.pubkey());
    let result = send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]);
    assert!(result.is_err());
}
