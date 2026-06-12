mod common;
use common::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_accept_mint_authority_should_transfer_and_close_pda() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_authority = Keypair::new();
    svm.airdrop(&new_authority.pubkey(), 10_000_000_000)
        .unwrap();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    // Nominate
    advance_slot(&mut svm);
    let ix = ix_nominate_mint_authority(
        &admin.pubkey(),
        &mint_pub,
        &admin.pubkey(),
        new_authority.pubkey(),
    );
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Accept
    advance_slot(&mut svm);
    let ix = ix_accept_mint_authority(&new_authority.pubkey(), &mint_pub);
    send_tx(&mut svm, &[ix], &new_authority, &[&new_authority]).unwrap();

    // Verify mint authority was transferred
    let authority = read_mint_authority(&svm, &mint_pub).unwrap();
    assert_eq!(authority, new_authority.pubkey());

    // Verify PDA was closed
    let (pda, _) = pending_mint_authority_pda(&mint_pub);
    assert!(svm.get_account(&pda).is_none());
}

#[test]
fn test_accept_mint_authority_should_fail_if_wrong_signer() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_authority = Keypair::new();
    let wrong_signer = Keypair::new();
    svm.airdrop(&wrong_signer.pubkey(), 10_000_000_000).unwrap();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    // Nominate
    advance_slot(&mut svm);
    let ix = ix_nominate_mint_authority(
        &admin.pubkey(),
        &mint_pub,
        &admin.pubkey(),
        new_authority.pubkey(),
    );
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Wrong signer tries to accept
    advance_slot(&mut svm);
    let ix = ix_accept_mint_authority(&wrong_signer.pubkey(), &mint_pub);
    let result = send_tx(&mut svm, &[ix], &wrong_signer, &[&wrong_signer]);
    assert!(result.is_err());
}
