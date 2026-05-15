mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_remove_gate_should_close_whitelist_state_account() {
    let (mut svm, _payer, _whitelist_auth, admin) = setup_initialized();
    let wallet = Keypair::new();

    let ix = ix_add_gate(&admin.pubkey(), &wallet.pubkey(), &admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_remove_gate(&admin.pubkey(), &wallet.pubkey(), &admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    assert_eq!(read_whitelist_role(&svm, &wallet.pubkey()), None);
}

#[test]
fn test_remove_gate_should_fail_if_signer_is_not_admin() {
    let (mut svm, _payer, _whitelist_auth, admin) = setup_initialized();
    let wallet = Keypair::new();
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_add_gate(&admin.pubkey(), &wallet.pubkey(), &admin.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_remove_gate(&fake.pubkey(), &wallet.pubkey(), &fake.pubkey());
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}
