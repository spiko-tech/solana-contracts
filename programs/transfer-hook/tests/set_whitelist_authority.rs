mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_set_whitelist_authority_should_transfer_authority() {
    let (mut svm, _payer, whitelist_auth, _admin) = setup_initialized();
    let new_auth = Keypair::new();

    let ix = ix_set_whitelist_authority(&whitelist_auth.pubkey(), new_auth.pubkey());
    send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]).unwrap();

    advance_slot(&mut svm);

    // Old authority can no longer whitelist
    let wallet = Keypair::new();
    let ix = ix_whitelist(
        &whitelist_auth.pubkey(),
        &wallet.pubkey(),
        &whitelist_auth.pubkey(),
        false,
    );
    let result = send_tx(&mut svm, &[ix], &whitelist_auth, &[&whitelist_auth]);
    assert!(result.is_err());

    // New authority can whitelist
    svm.airdrop(&new_auth.pubkey(), 10_000_000_000).unwrap();
    let ix = ix_whitelist(
        &new_auth.pubkey(),
        &wallet.pubkey(),
        &new_auth.pubkey(),
        false,
    );
    send_tx(&mut svm, &[ix], &new_auth, &[&new_auth]).unwrap();
    assert_eq!(
        read_whitelist_role(&svm, &wallet.pubkey()),
        Some(ROLE_WHITELISTED)
    );
}

#[test]
fn test_set_whitelist_authority_should_fail_if_signer_is_not_current_authority() {
    let (mut svm, _payer, _whitelist_auth, _admin) = setup_initialized();
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_set_whitelist_authority(&fake.pubkey(), fake.pubkey());
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}
