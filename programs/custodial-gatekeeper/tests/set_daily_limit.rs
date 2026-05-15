mod common;
use common::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

#[test]
fn test_set_daily_limit_should_create_limit_account() {
    let (mut svm, _payer, admin, _initiator) = setup_initialized();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub);

    advance_slot(&mut svm);
    let ix = ix_set_daily_limit(&admin.pubkey(), &mint_pub, &admin.pubkey(), 500_000);
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    let (limit_pda, _) = withdrawal_daily_limit_pda(&mint_pub);
    let acc = svm.get_account(&limit_pda).unwrap();
    let stored_limit = u64::from_le_bytes(acc.data[8..16].try_into().unwrap());
    assert_eq!(stored_limit, 500_000);
}

#[test]
fn test_set_daily_limit_should_fail_if_not_admin() {
    let (mut svm, _payer, _admin, initiator) = setup_initialized();

    let mint_pub = Pubkey::new_unique();
    create_fake_mint(&mut svm, &mint_pub);

    advance_slot(&mut svm);
    let ix = ix_set_daily_limit(&initiator.pubkey(), &mint_pub, &initiator.pubkey(), 500_000);
    assert!(send_tx(&mut svm, &[ix], &initiator, &[&initiator]).is_err());
}
