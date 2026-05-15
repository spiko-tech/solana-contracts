mod common;

use common::*;
use litesvm::LiteSVM;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

/// The SPL Transfer Hook execute discriminator
const EXECUTE_DISCRIMINATOR: [u8; 8] = [105, 37, 101, 197, 75, 251, 102, 26];

/// Token-2022 program ID
const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Create a fake Token-2022 token account in LiteSVM
fn create_fake_token_account(
    svm: &mut LiteSVM,
    address: &Pubkey,
    owner: &Pubkey,
    mint: &Pubkey,
    amount: u64,
) {
    let mut data = vec![0u8; 165];
    data[0..32].copy_from_slice(mint.as_ref());
    data[32..64].copy_from_slice(owner.as_ref());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    data[108] = 2; // AccountState::Initialized

    svm.set_account(
        *address,
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

/// Build the execute instruction (SPL Transfer Hook Interface)
fn ix_execute(
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    source_authority: &Pubkey,
    extra_account_metas: &Pubkey,
    hook_config: &Pubkey,
    source_whitelist: &Pubkey,
    dest_whitelist: &Pubkey,
    amount: u64,
) -> Instruction {
    let mut data = EXECUTE_DISCRIMINATOR.to_vec();
    data.extend_from_slice(&amount.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*source, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(*destination, false),
            AccountMeta::new_readonly(*source_authority, false),
            AccountMeta::new_readonly(*extra_account_metas, false),
            AccountMeta::new_readonly(*hook_config, false),
            AccountMeta::new_readonly(*source_whitelist, false),
            AccountMeta::new_readonly(*dest_whitelist, false),
        ],
        data,
    }
}

/// Set up a full environment for transfer hook testing
fn setup_transfer_env() -> (LiteSVM, Keypair, Keypair, Pubkey, Pubkey) {
    let (mut svm, _payer, whitelist_auth, admin) = setup_initialized();
    let mint = Pubkey::new_unique();
    let (extra_metas_pda, _) = extra_account_metas_pda(&mint);

    svm.set_account(
        extra_metas_pda,
        Account {
            lamports: 1_000_000,
            data: vec![0u8; 128],
            owner: PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    (svm, whitelist_auth, admin, mint, extra_metas_pda)
}

/// Whitelist a wallet with the given role
fn whitelist_wallet_with_role(
    svm: &mut LiteSVM,
    whitelist_auth: &Keypair,
    admin: &Keypair,
    role: u8,
) -> Pubkey {
    let wallet = Keypair::new();
    let wallet_pub = wallet.pubkey();

    match role {
        ROLE_WHITELISTED => {
            let ix = ix_whitelist(
                &whitelist_auth.pubkey(),
                &wallet_pub,
                &whitelist_auth.pubkey(),
                false,
            );
            send_tx(svm, &[ix], whitelist_auth, &[whitelist_auth]).unwrap();
        }
        ROLE_WHITELISTED_EXT => {
            let ix = ix_whitelist(
                &whitelist_auth.pubkey(),
                &wallet_pub,
                &whitelist_auth.pubkey(),
                true,
            );
            send_tx(svm, &[ix], whitelist_auth, &[whitelist_auth]).unwrap();
        }
        ROLE_WHITELISTED_GATE => {
            let ix = ix_add_gate(&admin.pubkey(), &wallet_pub, &admin.pubkey());
            send_tx(svm, &[ix], admin, &[admin]).unwrap();
        }
        ROLE_UNWHITELISTED => {
            let ix = ix_whitelist(
                &whitelist_auth.pubkey(),
                &wallet_pub,
                &whitelist_auth.pubkey(),
                false,
            );
            send_tx(svm, &[ix], whitelist_auth, &[whitelist_auth]).unwrap();
            advance_slot(svm);
            let ix = ix_unwhitelist(&whitelist_auth.pubkey(), &wallet_pub);
            send_tx(svm, &[ix], whitelist_auth, &[whitelist_auth]).unwrap();
        }
        ROLE_UNWHITELISTED_EXT => {
            let ix = ix_whitelist(
                &whitelist_auth.pubkey(),
                &wallet_pub,
                &whitelist_auth.pubkey(),
                true,
            );
            send_tx(svm, &[ix], whitelist_auth, &[whitelist_auth]).unwrap();
            advance_slot(svm);
            let ix = ix_unwhitelist(&whitelist_auth.pubkey(), &wallet_pub);
            send_tx(svm, &[ix], whitelist_auth, &[whitelist_auth]).unwrap();
        }
        _ => panic!("Unknown role"),
    }
    advance_slot(svm);
    wallet_pub
}

/// Test a single transfer from source_role → dest_role
fn test_transfer(source_role: u8, dest_role: u8, expect_success: bool) {
    let (mut svm, whitelist_auth, admin, mint, extra_metas_pda) = setup_transfer_env();

    let source_owner = whitelist_wallet_with_role(&mut svm, &whitelist_auth, &admin, source_role);
    let dest_owner = whitelist_wallet_with_role(&mut svm, &whitelist_auth, &admin, dest_role);

    let source_token = Pubkey::new_unique();
    let dest_token = Pubkey::new_unique();
    create_fake_token_account(&mut svm, &source_token, &source_owner, &mint, 1000);
    create_fake_token_account(&mut svm, &dest_token, &dest_owner, &mint, 0);

    let (hook_config, _) = hook_config_pda();
    let (source_wl, _) = whitelist_pda(&source_owner);
    let (dest_wl, _) = whitelist_pda(&dest_owner);

    let ix = ix_execute(
        &source_token,
        &mint,
        &dest_token,
        &source_owner,
        &extra_metas_pda,
        &hook_config,
        &source_wl,
        &dest_wl,
        100,
    );

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    let result = send_tx(&mut svm, &[ix], &payer, &[&payer]);

    if expect_success {
        assert!(
            result.is_ok(),
            "Expected success for {} -> {}",
            source_role,
            dest_role
        );
    } else {
        assert!(
            result.is_err(),
            "Expected failure for {} -> {}",
            source_role,
            dest_role
        );
    }
}

// === VALIDATION MATRIX TESTS ===

#[test]
fn test_execute_should_allow_whitelisted_to_whitelisted() {
    test_transfer(ROLE_WHITELISTED, ROLE_WHITELISTED, true);
}

#[test]
fn test_execute_should_allow_whitelisted_to_gate() {
    test_transfer(ROLE_WHITELISTED, ROLE_WHITELISTED_GATE, true);
}

#[test]
fn test_execute_should_reject_whitelisted_to_ext() {
    test_transfer(ROLE_WHITELISTED, ROLE_WHITELISTED_EXT, false);
}

#[test]
fn test_execute_should_reject_whitelisted_to_unwhitelisted() {
    test_transfer(ROLE_WHITELISTED, ROLE_UNWHITELISTED, false);
}

#[test]
fn test_execute_should_allow_ext_to_ext() {
    test_transfer(ROLE_WHITELISTED_EXT, ROLE_WHITELISTED_EXT, true);
}

#[test]
fn test_execute_should_allow_ext_to_whitelisted() {
    test_transfer(ROLE_WHITELISTED_EXT, ROLE_WHITELISTED, true);
}

#[test]
fn test_execute_should_allow_ext_to_gate() {
    test_transfer(ROLE_WHITELISTED_EXT, ROLE_WHITELISTED_GATE, true);
}

#[test]
fn test_execute_should_reject_ext_to_unwhitelisted() {
    test_transfer(ROLE_WHITELISTED_EXT, ROLE_UNWHITELISTED, false);
}

#[test]
fn test_execute_should_allow_gate_to_ext() {
    test_transfer(ROLE_WHITELISTED_GATE, ROLE_WHITELISTED_EXT, true);
}

#[test]
fn test_execute_should_allow_gate_to_whitelisted() {
    test_transfer(ROLE_WHITELISTED_GATE, ROLE_WHITELISTED, true);
}

#[test]
fn test_execute_should_allow_gate_to_gate() {
    test_transfer(ROLE_WHITELISTED_GATE, ROLE_WHITELISTED_GATE, true);
}

#[test]
fn test_execute_should_reject_gate_to_unwhitelisted() {
    test_transfer(ROLE_WHITELISTED_GATE, ROLE_UNWHITELISTED, false);
}

#[test]
fn test_execute_should_reject_unwhitelisted_to_whitelisted() {
    test_transfer(ROLE_UNWHITELISTED, ROLE_WHITELISTED, false);
}

#[test]
fn test_execute_should_reject_unwhitelisted_ext_to_whitelisted() {
    test_transfer(ROLE_UNWHITELISTED_EXT, ROLE_WHITELISTED, false);
}
