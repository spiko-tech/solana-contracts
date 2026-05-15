mod common;
use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn test_set_daily_limit_should_create_limit_account() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();

    let mint_pub = solana_pubkey::Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    advance_slot(&mut svm);
    let ix = ix_set_daily_limit(&admin.pubkey(), &mint_pub, &admin.pubkey(), 1_000_000);
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    let (limit_pda, _) = mint_daily_limit_pda(&mint_pub);
    let acc = svm.get_account(&limit_pda).unwrap();
    // discriminator(8) + limit(8) + used_amount(8) + last_day(8) + bump(1) = 33
    assert_eq!(acc.data.len(), 33);
    // limit at offset 8
    let stored_limit = u64::from_le_bytes(acc.data[8..16].try_into().unwrap());
    assert_eq!(stored_limit, 1_000_000);
}

#[test]
fn test_set_daily_limit_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator) = setup_initialized();

    let mint_pub = solana_pubkey::Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub, 0);

    advance_slot(&mut svm);
    let ix = ix_set_daily_limit(
        &initiator.pubkey(),
        &mint_pub,
        &initiator.pubkey(),
        1_000_000,
    );
    let result = send_tx(&mut svm, &[ix], &initiator, &[&initiator]);
    assert!(result.is_err());
}
