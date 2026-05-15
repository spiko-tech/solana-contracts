mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_add_gate_should_set_role_to_whitelisted_gate() {
    let (mut svm, _payer, _whitelist_auth, admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_add_gate(&admin.pubkey(), &wallet.pubkey(), &admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_WHITELISTED_GATE)
    );
}

#[test]
fn test_add_gate_should_fail_if_signer_is_not_admin() {
    let (mut svm, _payer, _whitelist_auth, _admin) = setup_initialized();
    let wallet = Keypair::new();
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_add_gate(&fake.pubkey(), &wallet.pubkey(), &fake.pubkey());
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}

#[test]
fn test_add_gate_should_fail_if_wallet_already_whitelisted() {
    let (mut svm, _payer, whitelist_auth, admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        false,
    );
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_add_gate(&admin.pubkey(), &wallet.pubkey(), &admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}

#[test]
fn test_add_gate_should_succeed_if_already_gated() {
    let (mut svm, _payer, _whitelist_auth, admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_add_gate(&admin.pubkey(), &wallet.pubkey(), &admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_add_gate(&admin.pubkey(), &wallet.pubkey(), &admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_WHITELISTED_GATE)
    );
}
