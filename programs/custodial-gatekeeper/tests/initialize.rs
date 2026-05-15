mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_initialize_should_create_config_and_vault_authority() {
    let (mut svm, _payer) = setup();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let initiator = Keypair::new();
    let ix = ix_initialize(&admin.pubkey(), initiator.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    let (config, _) = gatekeeper_config_pda();
    let acc = svm.get_account(&config).unwrap();
    assert_eq!(&acc.data[8..40], admin.pubkey().as_ref());
    assert_eq!(&acc.data[40..72], initiator.pubkey().as_ref());

    let (vault_auth, _) = vault_authority_pda();
    assert!(svm.get_account(&vault_auth).is_some());
}

#[test]
fn test_initialize_should_fail_if_already_initialized() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    advance_slot(&mut svm);
    let ix = ix_initialize(&admin.pubkey(), admin.pubkey());
    assert!(send_tx(&mut svm, &[ix], &admin, &[&admin]).is_err());
}
