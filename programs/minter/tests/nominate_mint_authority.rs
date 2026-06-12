mod common;
use common::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_nominate_mint_authority_should_create_pending_transfer() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let new_authority = Keypair::new();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    advance_slot(&mut svm);
    let ix = ix_nominate_mint_authority(
        &admin.pubkey(),
        &mint_pub,
        &admin.pubkey(),
        new_authority.pubkey(),
    );
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify PDA was created
    let (pda, _) = pending_mint_authority_pda(&mint_pub);
    let acc = svm.get_account(&pda).unwrap();
    // discriminator(8) + new_authority(32) + mint(32) + bump(1) = 73
    assert_eq!(acc.data.len(), 73);
    // new_authority at offset 8
    let stored_authority = &acc.data[8..40];
    assert_eq!(stored_authority, new_authority.pubkey().as_ref());
    // mint at offset 40
    let stored_mint = &acc.data[40..72];
    assert_eq!(stored_mint, mint_pub.as_ref());
}

#[test]
fn test_nominate_mint_authority_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator) = setup_initialized();
    let new_authority = Keypair::new();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    advance_slot(&mut svm);
    let ix = ix_nominate_mint_authority(
        &initiator.pubkey(),
        &mint_pub,
        &initiator.pubkey(),
        new_authority.pubkey(),
    );
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}

#[test]
fn test_nominate_mint_authority_should_fail_if_already_pending() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();
    let first = Keypair::new();
    let second = Keypair::new();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    advance_slot(&mut svm);
    let ix =
        ix_nominate_mint_authority(&admin.pubkey(), &mint_pub, &admin.pubkey(), first.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Second nomination should fail (PDA already exists, `init` not `init_if_needed`)
    advance_slot(&mut svm);
    let ix =
        ix_nominate_mint_authority(&admin.pubkey(), &mint_pub, &admin.pubkey(), second.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
