mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_set_admin_should_transfer_admin() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_admin = Keypair::new();
    svm.airdrop(&new_admin.pubkey(), 10_000_000_000).unwrap();

    advance_slot(&mut svm);
    let ix = ix_set_admin(&admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);
    let another = Keypair::new();
    let ix = ix_set_admin(&new_admin.pubkey(), another.pubkey());
    send_tx(&mut svm, &[ix], &new_admin, &[&new_admin]).unwrap();
}

#[test]
fn test_set_admin_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator) = setup_initialized();

    advance_slot(&mut svm);
    let ix = ix_set_admin(&initiator.pubkey(), initiator.pubkey());
    assert!(send_tx(&mut svm, &[ix], &initiator, &[&initiator]).is_err());
}
