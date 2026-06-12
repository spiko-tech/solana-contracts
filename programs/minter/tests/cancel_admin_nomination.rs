mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_cancel_admin_nomination_should_clear_pending() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_admin = Keypair::new();

    // Nominate
    advance_slot(&mut svm);
    let ix = ix_nominate_admin(&admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Cancel
    advance_slot(&mut svm);
    let ix = ix_cancel_admin_nomination(&admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify pending_admin is None
    let (config_pda, _) = minter_config_pda();
    let acc = svm.get_account(&config_pda).unwrap();
    assert_eq!(acc.data[40], 0); // None
}

#[test]
fn test_cancel_admin_nomination_should_fail_if_no_pending() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();

    // Try to cancel without nomination
    advance_slot(&mut svm);
    let ix = ix_cancel_admin_nomination(&admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}

#[test]
fn test_cancel_admin_nomination_should_fail_if_not_admin() {
    let (mut svm, _payer, admin, initiator) = setup_initialized();
    let new_admin = Keypair::new();

    // Nominate
    advance_slot(&mut svm);
    let ix = ix_nominate_admin(&admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Non-admin tries to cancel
    advance_slot(&mut svm);
    let ix = ix_cancel_admin_nomination(&initiator.pubkey());
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}
