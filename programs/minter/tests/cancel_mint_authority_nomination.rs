mod common;
use common::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_cancel_mint_authority_nomination_should_close_pda() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_authority = Keypair::new();

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

    // Cancel
    advance_slot(&mut svm);
    let ix = ix_cancel_mint_authority_nomination(&admin.pubkey(), &mint_pub);
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify PDA was closed
    let (pda, _) = pending_mint_authority_pda(&mint_pub);
    assert!(svm.get_account(&pda).is_none());
}

#[test]
fn test_cancel_mint_authority_nomination_should_fail_if_not_admin() {
    let (mut svm, _payer, admin, initiator) = setup_initialized();
    let new_authority = Keypair::new();

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

    // Non-admin tries to cancel
    advance_slot(&mut svm);
    let ix = ix_cancel_mint_authority_nomination(&initiator.pubkey(), &mint_pub);
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}
