mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_initialize_should_create_config_and_vault_authority() {
    let (mut svm, _payer) = setup();
    let admin = Keypair::new();
    let redemption_auth = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_initialize(&admin.pubkey(), redemption_auth.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify RedemptionConfig
    let (config_pda, _) = redemption_config_pda();
    let account = svm.get_account(&config_pda).unwrap();
    // 8 disc + 32 admin + 32 redemption_authority + 1 bump = 73
    assert_eq!(account.data.len(), 8 + 32 + 32 + 1);
    assert_eq!(&account.data[8..40], admin.pubkey().as_ref());
    assert_eq!(&account.data[40..72], redemption_auth.pubkey().as_ref());

    // Verify VaultAuthority exists
    let (vault_auth_pda, _) = vault_authority_pda();
    let va_account = svm.get_account(&vault_auth_pda).unwrap();
    // 8 disc + 1 bump = 9
    assert_eq!(va_account.data.len(), 8 + 1);
}

#[test]
fn test_initialize_should_fail_if_already_initialized() {
    let (mut svm, _payer, admin, _redemption_auth) = setup_initialized();

    advance_slot(&mut svm);

    let new_auth = Keypair::new();
    let ix = ix_initialize(&admin.pubkey(), new_auth.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
