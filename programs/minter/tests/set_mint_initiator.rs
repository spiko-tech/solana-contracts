mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_set_mint_initiator_should_transfer_initiator() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_initiator = Keypair::new();

    advance_slot(&mut svm);
    let ix = ix_set_mint_initiator(&admin.pubkey(), new_initiator.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify old initiator can no longer act (would need an initiate_mint test,
    // but we verify by checking account data)
    let (config, _) = minter_config_pda();
    let acc = svm.get_account(&config).unwrap();
    // Layout when pending_admin=None: disc(8) + admin(32) + pending_admin_tag(1) + mint_initiator(32) + bump(1)
    // mint_initiator at offset 41..73
    assert_eq!(&acc.data[41..73], new_initiator.pubkey().as_ref());
}

#[test]
fn test_set_mint_initiator_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator) = setup_initialized();

    advance_slot(&mut svm);
    let ix = ix_set_mint_initiator(&initiator.pubkey(), initiator.pubkey());
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}
