mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_set_admin_should_transfer_admin() {
    let (mut svm, _payer, admin, _redemption_auth) = setup_initialized();
    let new_admin = Keypair::new();
    svm.airdrop(&new_admin.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_set_admin(&admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);

    // Old admin should fail
    let ix = ix_set_admin(&admin.pubkey(), admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());

    // New admin should succeed
    let ix = ix_set_admin(&new_admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &new_admin, &[&new_admin]).unwrap();
}

#[test]
fn test_set_admin_should_fail_if_signer_is_not_current_admin() {
    let (mut svm, _payer, _admin, _redemption_auth) = setup_initialized();
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_set_admin(&fake.pubkey(), fake.pubkey());
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}
