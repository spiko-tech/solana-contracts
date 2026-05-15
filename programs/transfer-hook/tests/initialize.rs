mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_initialize_should_create_hook_config_with_authorities() {
    let (mut svm, _payer) = setup();
    let whitelist_auth = Keypair::new();
    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_initialize(&admin.pubkey(), whitelist_auth.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify HookConfig account exists and has correct data
    let (hook_config_pda, _) = hook_config_pda();
    let account = svm.get_account(&hook_config_pda).unwrap();
    // 8 disc + 32 whitelist_authority + 32 admin + 1 bump = 73
    assert_eq!(account.data.len(), 8 + 32 + 32 + 1);
    assert_eq!(&account.data[8..40], whitelist_auth.pubkey().as_ref());
    assert_eq!(&account.data[40..72], admin.pubkey().as_ref());
}

#[test]
fn test_initialize_should_fail_if_already_initialized() {
    let (mut svm, _payer, _whitelist_auth, admin) = setup_initialized();

    advance_slot(&mut svm);

    let new_auth = Keypair::new();
    let ix = ix_initialize(&admin.pubkey(), new_auth.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
