mod common;
use common::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_initiate_gatekeep_should_transfer_when_within_limit() {
    let (
        mut svm,
        _payer,
        _admin,
        initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        _sender_token,
        dest_wallet,
        dest_token,
    ) = setup_with_vault(1_000_000, 1_000_000);

    advance_slot(&mut svm);
    let salt = 1u64;
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

    assert_eq!(read_operation_status(&svm, salt), Some(STATUS_DONE));
    assert_eq!(read_token_balance(&svm, &dest_token), Some(amount));
}

#[test]
fn test_initiate_gatekeep_should_create_pending_when_over_limit() {
    let (
        mut svm,
        _payer,
        _admin,
        initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        _sender_token,
        dest_wallet,
        dest_token,
    ) = setup_with_vault(1_000_000, 100_000);

    advance_slot(&mut svm);
    let salt = 2u64;
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
    assert_eq!(read_token_balance(&svm, &dest_token), Some(0));
}

#[test]
fn test_initiate_gatekeep_should_fail_if_not_initiator() {
    let (
        mut svm,
        _payer,
        admin,
        _initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        _sender_token,
        dest_wallet,
        dest_token,
    ) = setup_with_vault(1_000_000, 1_000_000);

    advance_slot(&mut svm);
    let ix = ix_initiate_gatekeep(
        &admin.pubkey(),
        &admin.pubkey(),
        &mint_pub,
        &vault_pub,
        &dest_token,
        3,
        sender_wallet,
        dest_wallet,
        100,
    );
    assert!(send_tx(&mut svm, &[ix], &admin, &[&admin]).is_err());
}

#[test]
fn test_initiate_gatekeep_should_fail_if_amount_zero() {
    let (
        mut svm,
        _payer,
        _admin,
        initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        _sender_token,
        dest_wallet,
        dest_token,
    ) = setup_with_vault(1_000_000, 1_000_000);

    advance_slot(&mut svm);
    let ix = ix_initiate_gatekeep(
        &initiator.pubkey(),
        &initiator.pubkey(),
        &mint_pub,
        &vault_pub,
        &dest_token,
        4,
        sender_wallet,
        dest_wallet,
        0,
    );
    assert!(send_tx(&mut svm, &[ix], &initiator, &[&initiator]).is_err());
}

#[test]
fn test_initiate_gatekeep_should_fail_if_salt_already_used() {
    let (
        mut svm,
        _payer,
        _admin,
        initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        _sender_token,
        dest_wallet,
        dest_token,
    ) = setup_with_vault(1_000_000, 1_000_000);

    advance_slot(&mut svm);
    let salt = 5u64;
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

    advance_slot(&mut svm);
    let ix = ix_initiate_gatekeep(
        &initiator.pubkey(),
        &initiator.pubkey(),
        &mint_pub,
        &vault_pub,
        &dest_token,
        salt,
        sender_wallet,
        dest_wallet,
        200,
    );
    assert!(send_tx(&mut svm, &[ix], &initiator, &[&initiator]).is_err());
}

#[test]
fn test_initiate_gatekeep_should_fail_if_destination_owner_mismatch() {
    let (
        mut svm,
        _payer,
        _admin,
        initiator,
        mint_pub,
        vault_pub,
        sender_wallet,
        _sender_token,
        _dest_wallet,
        _dest_token,
    ) = setup_with_vault(1_000_000, 1_000_000);

    // Create a destination token account owned by wrong_owner
    let wrong_owner = Pubkey::new_unique();
    let real_dest = Pubkey::new_unique();
    let bad_dest_token = Pubkey::new_unique();
    create_fake_token_account(&mut svm, &bad_dest_token, &mint_pub, &wrong_owner, 0);

    advance_slot(&mut svm);
    let ix = ix_initiate_gatekeep(
        &initiator.pubkey(),
        &initiator.pubkey(),
        &mint_pub,
        &vault_pub,
        &bad_dest_token,
        6,
        sender_wallet,
        real_dest, // doesn't match wrong_owner
        100,
    );
    assert!(send_tx(&mut svm, &[ix], &initiator, &[&initiator]).is_err());
}
