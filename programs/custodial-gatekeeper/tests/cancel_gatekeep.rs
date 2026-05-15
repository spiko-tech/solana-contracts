mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_cancel_gatekeep_should_refund_to_sender_and_set_canceled() {
    let (
        mut svm,
        _payer,
        admin,
        initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        sender_token,
        dest_wallet,
        dest_token,
    ) = setup_with_vault(1_000_000, 100_000); // limit 100k, so 500k goes pending

    // Create pending operation
    advance_slot(&mut svm);
    let salt = 20u64;
    let amount = 500_000u64;
    let ix = ix_initiate_gatekeep(
        &initiator.pubkey(),
        &initiator.pubkey(),
        &mint_pub,
        &vault_pub,
        &dest_token,
        salt,
        sender_wallet,
        dest_wallet,
        amount,
    );
    send_tx(&mut svm, &[ix], &initiator, &[&initiator]).unwrap();
    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_PENDING));

    // Admin cancels — refund to sender
    advance_slot(&mut svm);
    let ix = ix_cancel_gatekeep(&admin.pubkey(), &mint_pub, &vault_pub, &sender_token, salt);
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_CANCELED));
    assert_eq!(read_token_balance(&svm, &sender_token), Some(amount));
    // Destination should still be 0
    assert_eq!(read_token_balance(&svm, &dest_token), Some(0));
}

#[test]
fn test_cancel_gatekeep_should_fail_if_not_admin() {
    let (
        mut svm,
        _payer,
        _admin,
        initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        sender_token,
        dest_wallet,
        dest_token,
    ) = setup_with_vault(1_000_000, 100_000);

    advance_slot(&mut svm);
    let salt = 21u64;
    let ix = ix_initiate_gatekeep(
        &initiator.pubkey(),
        &initiator.pubkey(),
        &mint_pub,
        &vault_pub,
        &dest_token,
        salt,
        sender_wallet,
        dest_wallet,
        500_000,
    );
    send_tx(&mut svm, &[ix], &initiator, &[&initiator]).unwrap();

    advance_slot(&mut svm);
    let ix = ix_cancel_gatekeep(
        &initiator.pubkey(),
        &mint_pub,
        &vault_pub,
        &sender_token,
        salt,
    );
    assert!(send_tx(&mut svm, &[ix], &initiator, &[&initiator]).is_err());
}

#[test]
fn test_cancel_gatekeep_should_fail_if_not_pending() {
    let (
        mut svm,
        _payer,
        admin,
        initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        sender_token,
        dest_wallet,
        dest_token,
    ) = setup_with_vault(1_000_000, 1_000_000);

    advance_slot(&mut svm);
    let salt = 22u64;
    let ix = ix_initiate_gatekeep(
        &initiator.pubkey(),
        &initiator.pubkey(),
        &mint_pub,
        &vault_pub,
        &dest_token,
        salt,
        sender_wallet,
        dest_wallet,
        100,
    );
    send_tx(&mut svm, &[ix], &initiator, &[&initiator]).unwrap();
    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_DONE));

    advance_slot(&mut svm);
    let ix = ix_cancel_gatekeep(&admin.pubkey(), &mint_pub, &vault_pub, &sender_token, salt);
    assert!(send_tx(&mut svm, &[ix], &admin, &[&admin]).is_err());
}
