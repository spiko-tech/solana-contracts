mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_set_admin_should_transfer_admin() {
    let (mut svm, _payer, _whitelist_auth, admin) = setup_initialized();
    let new_admin = Keypair::new();

    let ix = ix_set_admin(&admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);

    // Old admin can no longer add_gate
    let wallet = Keypair::new();
    let ix = ix_add_gate(&admin.pubkey(), &wallet.pubkey(), &admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());

    // New admin can add_gate
    svm.airdrop(&new_admin.pubkey(), 10_000_000_000).unwrap();
    let ix = ix_add_gate(&new_admin.pubkey(), &wallet.pubkey(), &new_admin.pubkey());
    send_tx(&mut svm, &[ix], &new_admin, &[&new_admin]).unwrap();
    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_WHITELISTED_GATE)
    );
}

#[test]
fn test_set_admin_should_fail_if_signer_is_not_current_admin() {
    let (mut svm, _payer, _whitelist_auth, _admin) = setup_initialized();
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_set_admin(&fake.pubkey(), fake.pubkey());
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}
