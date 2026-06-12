mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_initialize_should_create_config() {
    let (mut svm, payer) = setup();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let initiator = Keypair::new();
    let ix = ix_initialize(&admin.pubkey(), initiator.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    let (config, _) = minter_config_pda();
    let acc = svm.get_account(&config).unwrap();
    // discriminator(8) + admin(32) + pending_admin(1 for None OR 1+32 for Some) + mint_initiator(32) + bump(1)
    // Account allocated at max size: 106 bytes
    assert_eq!(acc.data.len(), 106);
    // admin stored at offset 8
    assert_eq!(&acc.data[8..40], admin.pubkey().as_ref());
    // pending_admin at offset 40: tag=0 (None)
    assert_eq!(acc.data[40], 0);
    // When None, mint_initiator starts at offset 41 (Borsh variable-length)
    assert_eq!(&acc.data[41..73], initiator.pubkey().as_ref());
}

#[test]
fn test_initialize_should_fail_if_already_initialized() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();

    advance_slot(&mut svm);
    let ix = ix_initialize(&admin.pubkey(), admin.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
