mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_accept_admin_should_transfer_admin() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_admin = Keypair::new();
    svm.airdrop(&new_admin.pubkey(), 10_000_000_000).unwrap();

    // Nominate
    advance_slot(&mut svm);
    let ix = ix_nominate_admin(&admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Accept
    advance_slot(&mut svm);
    let ix = ix_accept_admin(&new_admin.pubkey());
    send_tx(&mut svm, &[ix], &new_admin, &[&new_admin]).unwrap();

    // Verify admin changed and pending_admin cleared
    let (config_pda, _) = minter_config_pda();
    let acc = svm.get_account(&config_pda).unwrap();
    // admin at offset 8
    let stored_admin = &acc.data[8..40];
    assert_eq!(stored_admin, new_admin.pubkey().as_ref());
    // pending_admin tag at offset 40 should be 0 (None)
    assert_eq!(acc.data[40], 0);
}

#[test]
fn test_accept_admin_should_fail_if_wrong_signer() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_admin = Keypair::new();
    let wrong_signer = Keypair::new();
    svm.airdrop(&wrong_signer.pubkey(), 10_000_000_000).unwrap();

    // Nominate
    advance_slot(&mut svm);
    let ix = ix_nominate_admin(&admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Wrong signer tries to accept
    advance_slot(&mut svm);
    let ix = ix_accept_admin(&wrong_signer.pubkey());
    let result = send_tx(&mut svm, &[ix], &wrong_signer, &[&wrong_signer]);
    assert!(result.is_err());
}

#[test]
fn test_accept_admin_should_fail_if_no_pending() {
    let (mut svm, _payer, _admin, _initiator) = setup_initialized();
    let random = Keypair::new();
    svm.airdrop(&random.pubkey(), 10_000_000_000).unwrap();

    // Try to accept without nomination
    advance_slot(&mut svm);
    let ix = ix_accept_admin(&random.pubkey());
    let result = send_tx(&mut svm, &[ix], &random, &[&random]);
    assert!(result.is_err());
}
