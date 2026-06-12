mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_nominate_admin_should_set_pending_admin() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_admin = Keypair::new();

    advance_slot(&mut svm);
    let ix = ix_nominate_admin(&admin.pubkey(), new_admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify pending_admin is set
    // Layout when Some: disc(8) + admin(32) + pending_admin_tag(1) + pending_admin_pubkey(32) + mint_initiator(32) + bump(1)
    let (config_pda, _) = minter_config_pda();
    let acc = svm.get_account(&config_pda).unwrap();
    // pending_admin tag at offset 40
    assert_eq!(acc.data[40], 1); // Some
                                 // pending_admin pubkey at offset 41..73
    let pending = &acc.data[41..73];
    assert_eq!(pending, new_admin.pubkey().as_ref());
}

#[test]
fn test_nominate_admin_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator) = setup_initialized();
    let new_admin = Keypair::new();

    advance_slot(&mut svm);
    let ix = ix_nominate_admin(&initiator.pubkey(), new_admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}

#[test]
fn test_nominate_admin_should_overwrite_previous_nomination() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let first_nominee = Keypair::new();
    let second_nominee = Keypair::new();

    advance_slot(&mut svm);
    let ix = ix_nominate_admin(&admin.pubkey(), first_nominee.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);
    let ix = ix_nominate_admin(&admin.pubkey(), second_nominee.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify pending_admin is the second nominee
    let (config_pda, _) = minter_config_pda();
    let acc = svm.get_account(&config_pda).unwrap();
    assert_eq!(acc.data[40], 1); // Some
    let pending = &acc.data[41..73];
    assert_eq!(pending, second_nominee.pubkey().as_ref());
}
