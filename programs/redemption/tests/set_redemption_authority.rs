mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_set_redemption_authority_should_transfer_authority() {
    let (mut svm, _payer, admin, _redemption_auth, mint, vault) = setup_with_vault(1_000_000);
    let new_auth = Keypair::new();
    svm.airdrop(&new_auth.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_set_redemption_authority(&admin.pubkey(), new_auth.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);

    // New authority can redeem
    let ix = ix_redeem(
        &new_auth.pubkey(),
        &mint,
        &vault,
        &new_auth.pubkey(),
        1,
        100_000,
    );
    send_tx(&mut svm, &[ix], &new_auth, &[&new_auth]).unwrap();
    assert_eq!(read_token_balance(&svm, &vault), Some(900_000));
}

#[test]
fn test_set_redemption_authority_should_fail_if_signer_is_not_admin() {
    let (mut svm, _payer, _admin, _redemption_auth) = setup_initialized();
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_set_redemption_authority(&fake.pubkey(), fake.pubkey());
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}
