mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_set_gatekeeper_initiator_should_transfer_initiator() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_initiator = Keypair::new();

    advance_slot(&mut svm);
    let ix = ix_set_gatekeeper_initiator(&admin.pubkey(), new_initiator.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    let (config, _) = gatekeeper_config_pda();
    let acc = svm.get_account(&config).unwrap();
    assert_eq!(&acc.data[40..72], new_initiator.pubkey().as_ref());
}

#[test]
fn test_set_gatekeeper_initiator_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator) = setup_initialized();

    advance_slot(&mut svm);
    let ix = ix_set_gatekeeper_initiator(&initiator.pubkey(), initiator.pubkey());
    assert!(send_tx(&mut svm, &[ix], &initiator, &[&initiator]).is_err());
}
