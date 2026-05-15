mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_approve_mint_should_mint_tokens_for_pending_operation() {
    // Create a pending operation first (amount > limit)
    let (mut svm, _payer, admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(100_000);

    advance_slot(&mut svm);
    let salt = 10u64;
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
    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_PENDING));

    // Admin approves
    advance_slot(&mut svm);
    let ix = ix_approve_mint(&admin.pubkey(), &mint_pub, &destination, salt);
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_DONE));
    assert_eq!(read_token_balance(&svm, &destination), Some(amount));
}

#[test]
fn test_approve_mint_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(100_000);

    advance_slot(&mut svm);
    let salt = 11u64;
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

    // Non-admin tries to approve
    advance_slot(&mut svm);
    let ix = ix_approve_mint(&initiator.pubkey(), &mint_pub, &destination, salt);
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}

#[test]
fn test_approve_mint_should_fail_if_not_pending() {
    // Create and directly mint (within limit) -> STATUS_DONE
    let (mut svm, _payer, admin, initiator, mint_pub, destination, recipient) =
        setup_with_mint(1_000_000);

    advance_slot(&mut svm);
    let salt = 12u64;
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

    // Try to approve already-done operation
    advance_slot(&mut svm);
    let ix = ix_approve_mint(&admin.pubkey(), &mint_pub, &destination, salt);
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
