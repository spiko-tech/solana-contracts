mod common;
use common::*;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_set_mint_authority_should_transfer_authority() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    let new_authority = Pubkey::new_unique();

    advance_slot(&mut svm);
    let ix = ix_set_mint_authority(&admin.pubkey(), &mint_pub, new_authority);
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    // Verify the mint authority was changed
    let authority = read_mint_authority(&svm, &mint_pub).unwrap();
    assert_eq!(authority, new_authority);
}

#[test]
fn test_set_mint_authority_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator) = setup_initialized();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    let new_authority = Pubkey::new_unique();

    advance_slot(&mut svm);
    let ix = ix_set_mint_authority(&initiator.pubkey(), &mint_pub, new_authority);
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}

#[test]
fn test_set_mint_authority_should_fail_if_not_current_authority() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();

    // Create a mint with a different authority (not the minter_config PDA)
    let mint_pub = Pubkey::new_unique();
    let other_authority = Pubkey::new_unique();

    let mut data = vec![0u8; 82];
    // COption<Pubkey> mint_authority: Some(1) + other_authority
    data[0..4].copy_from_slice(&1u32.to_le_bytes());
    data[4..36].copy_from_slice(other_authority.as_ref());
    // decimals
    data[44] = 6;
    // is_initialized
    data[45] = 1;

    svm.set_account(
        mint_pub,
        solana_account::Account {
            lamports: 1_000_000_000,
            data,
            owner: TOKEN_2022_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let new_authority = Pubkey::new_unique();

    advance_slot(&mut svm);
    let ix = ix_set_mint_authority(&admin.pubkey(), &mint_pub, new_authority);
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
