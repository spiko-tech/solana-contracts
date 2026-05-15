mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_cancel_mint_should_set_status_to_canceled() {
    let (mut svm, _payer, admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(100_000);

    advance_slot(&mut svm);
    let salt = 20u64;
    let ix = ix_initiate_mint(
        &initiator.pubkey(),
        &mint_pub,
        &destination,
        &initiator.pubkey(),
        salt,
        recipient,
        500_000,
    );
    send_tx(&mut svm, &[ix], &initiator, &[&initiator]).unwrap();
    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_PENDING));

    // Admin cancels
    advance_slot(&mut svm);
    let ix = ix_cancel_mint(&admin.pubkey(), salt);
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_CANCELED));
}

#[test]
fn test_cancel_mint_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(100_000);

    advance_slot(&mut svm);
    let salt = 21u64;
    let ix = ix_initiate_mint(
        &initiator.pubkey(),
        &mint_pub,
        &destination,
        &initiator.pubkey(),
        salt,
        recipient,
        500_000,
    );
    send_tx(&mut svm, &[ix], &initiator, &[&initiator]).unwrap();

    advance_slot(&mut svm);
    let ix = ix_cancel_mint(&initiator.pubkey(), salt);
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}

#[test]
fn test_cancel_mint_should_fail_if_not_pending() {
    let (mut svm, _payer, admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(1_000_000);

    advance_slot(&mut svm);
    let salt = 22u64;
    let ix = ix_initiate_mint(
        &initiator.pubkey(),
        &mint_pub,
        &destination,
        &initiator.pubkey(),
        salt,
        recipient,
        100,
    );
    send_tx(&mut svm, &[ix], &initiator, &[&initiator]).unwrap();
    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_DONE));

    advance_slot(&mut svm);
    let ix = ix_cancel_mint(&admin.pubkey(), salt);
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
