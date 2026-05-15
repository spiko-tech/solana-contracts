mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_initiate_mint_should_mint_directly_when_within_limit() {
    let (mut svm, _payer, _admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(1_000_000);

    advance_slot(&mut svm);
    let salt = 1u64;
    let amount = 500_000u64;
    let ix = ix_initiate_mint(
        &initiator.pubkey(),
        &mint_pub,
        &destination,
        &initiator.pubkey(),
        salt,
        recipient,
        amount,
    );
    send_tx(&mut svm, &[ix], &initiator, &[&initiator]).unwrap();

    // Operation should be STATUS_DONE
    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_DONE));
    // Destination should have tokens
    assert_eq!(read_token_balance(&svm, &destination), Some(amount));
}

#[test]
fn test_initiate_mint_should_create_pending_operation_when_over_limit() {
    let (mut svm, _payer, _admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(100_000);

    advance_slot(&mut svm);
    let salt = 2u64;
    let amount = 500_000u64;
    let ix = ix_initiate_mint(
        &initiator.pubkey(),
        &mint_pub,
        &destination,
        &initiator.pubkey(),
        salt,
        recipient,
        amount,
    );
    send_tx(&mut svm, &[ix], &initiator, &[&initiator]).unwrap();

    // Operation should be STATUS_PENDING
    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_PENDING));
    // Destination should NOT have tokens
    assert_eq!(read_token_balance(&svm, &destination), Some(0));
}

#[test]
fn test_initiate_mint_should_fail_if_not_initiator() {
    let (mut svm, _payer, admin, _initiator, mint_pub, destination, recipient) =
        setup_with_mint(1_000_000);

    advance_slot(&mut svm);
    let ix = ix_initiate_mint(
        &admin.pubkey(),
        &mint_pub,
        &destination,
        &admin.pubkey(),
        3,
        recipient,
        100,
    );
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}

#[test]
fn test_initiate_mint_should_fail_if_amount_is_zero() {
    let (mut svm, _payer, _admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(1_000_000);

    advance_slot(&mut svm);
    let ix = ix_initiate_mint(
        &initiator.pubkey(),
        &mint_pub,
        &destination,
        &initiator.pubkey(),
        4,
        recipient,
        0,
    );
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}

#[test]
fn test_initiate_mint_should_fail_if_destination_owner_mismatch() {
    let (mut svm, _payer, _admin, initiator, mint_pub, _destination, _recipient) =
        setup_with_mint(1_000_000);

    let wrong_owner = solana_pubkey::Pubkey::new_unique();
    let real_recipient = solana_pubkey::Pubkey::new_unique();
    let bad_destination = solana_pubkey::Pubkey::new_unique();
    create_fake_token_account(&mut svm, &bad_destination, &mint_pub, &wrong_owner, 0);

    advance_slot(&mut svm);
    let ix = ix_initiate_mint(
        &initiator.pubkey(),
        &mint_pub,
        &bad_destination,
        &initiator.pubkey(),
        5,
        real_recipient,
        100,
    );
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}
