mod common;

use common::*;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

fn create_fake_mint(svm: &mut litesvm::LiteSVM, mint: &Pubkey) {
    let mut data = vec![0u8; 82];
    data[45] = 1;
    data[44] = 6;

    svm.set_account(
        *mint,
        Account {
            lamports: 1_000_000_000,
            data,
            owner: TOKEN_2022_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();
}

fn ix_register_mint(admin: &Pubkey, mint: &Pubkey) -> Instruction {
    let (hook_config, _) = hook_config_pda();
    let (extra_metas, _) = extra_account_metas_pda(mint);

    use sha2::Digest;
    let hash = sha2::Sha256::digest(b"global:register_mint");
    let data = hash[..8].to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(hook_config, false),
            AccountMeta::new(extra_metas, false),
            AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ),
        ],
        data,
    }
}

#[test]
fn test_register_mint_should_create_extra_account_meta_list() {
    let (mut svm, _payer, _whitelist_auth, admin) = setup_initialized();
    let mint = Keypair::new();
    create_fake_mint(&mut svm, &mint.pubkey());

    let ix = ix_register_mint(&admin.pubkey(), &mint.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    let (extra_metas, _) = extra_account_metas_pda(&mint.pubkey());
    let account = svm.get_account(&extra_metas).unwrap();
    assert!(account.data.len() > 0);
    assert_eq!(account.owner, PROGRAM_ID);
}

#[test]
fn test_register_mint_should_fail_if_signer_is_not_admin() {
    let (mut svm, _payer, _whitelist_auth, _admin) = setup_initialized();
    let mint = Keypair::new();
    create_fake_mint(&mut svm, &mint.pubkey());
    let fake = Keypair::new();
    svm.airdrop(&fake.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_register_mint(&fake.pubkey(), &mint.pubkey());
    let result = send_tx(&mut svm, &[ix], &fake, &[&fake]);
    assert!(result.is_err());
}

#[test]
fn test_register_mint_should_fail_if_already_registered() {
    let (mut svm, _payer, _whitelist_auth, admin) = setup_initialized();
    let mint = Keypair::new();
    create_fake_mint(&mut svm, &mint.pubkey());

    let ix = ix_register_mint(&admin.pubkey(), &mint.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    advance_slot(&mut svm);

    let ix = ix_register_mint(&admin.pubkey(), &mint.pubkey());
    let result = send_tx(&mut svm, &[ix], &admin, &[&admin]);
    assert!(result.is_err());
}
