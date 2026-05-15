use anchor_lang::AnchorSerialize;
use litesvm::LiteSVM;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub const PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("Fzyd28cVXwzaqoU9bqU8hLpcYqhQQtSyJVaGCgcGFEjq");

pub const HOOK_CONFIG_SEED: &[u8] = b"hook_config";
pub const WHITELIST_SEED: &[u8] = b"whitelist";
pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra-account-metas";

pub const ROLE_WHITELISTED_GATE: u8 = 1;
pub const ROLE_WHITELISTED: u8 = 2;
pub const ROLE_UNWHITELISTED: u8 = 3;
pub const ROLE_WHITELISTED_EXT: u8 = 4;
pub const ROLE_UNWHITELISTED_EXT: u8 = 5;

/// Anchor discriminator: sha256("global:<name>")[..8]
fn anchor_discriminator(name: &str) -> [u8; 8] {
    let full = format!("global:{}", name);
    let hash = <sha2::Sha256 as sha2::Digest>::digest(full.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

pub fn hook_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[HOOK_CONFIG_SEED], &PROGRAM_ID)
}

pub fn whitelist_pda(wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WHITELIST_SEED, wallet.as_ref()], &PROGRAM_ID)
}

pub fn extra_account_metas_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[EXTRA_ACCOUNT_METAS_SEED, mint.as_ref()], &PROGRAM_ID)
}

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/transfer_hook.so");
    svm.add_program(PROGRAM_ID, program_bytes);

    let admin = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    (svm, admin)
}

pub fn send_tx(
    svm: &mut LiteSVM,
    ixs: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> Result<litesvm::types::TransactionMetadata, litesvm::types::FailedTransactionMetadata> {
    let blockhash = svm.latest_blockhash();
    let msg = Message::new(ixs, Some(&payer.pubkey()));
    let tx = Transaction::new(signers, msg, blockhash);
    svm.send_transaction(tx)
}

pub fn ix_initialize(admin: &Pubkey, whitelist_authority: Pubkey) -> Instruction {
    let (hook_config, _) = hook_config_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        whitelist_authority: Pubkey,
    }

    let mut data = anchor_discriminator("initialize").to_vec();
    let args = Args {
        whitelist_authority,
    };
    args.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(hook_config, false),
            AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ),
        ],
        data,
    }
}

pub fn ix_whitelist(authority: &Pubkey, wallet: &Pubkey, payer: &Pubkey, ext: bool) -> Instruction {
    let (hook_config, _) = hook_config_pda();
    let (whitelist_state, _) = whitelist_pda(wallet);

    #[derive(AnchorSerialize)]
    struct Args {
        ext: bool,
    }

    let mut data = anchor_discriminator("whitelist").to_vec();
    Args { ext }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(hook_config, false),
            AccountMeta::new_readonly(*wallet, false),
            AccountMeta::new(whitelist_state, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ),
        ],
        data,
    }
}

pub fn ix_unwhitelist(authority: &Pubkey, wallet: &Pubkey) -> Instruction {
    let (hook_config, _) = hook_config_pda();
    let (whitelist_state, _) = whitelist_pda(wallet);

    let data = anchor_discriminator("unwhitelist").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(hook_config, false),
            AccountMeta::new_readonly(*wallet, false),
            AccountMeta::new(whitelist_state, false),
        ],
        data,
    }
}

pub fn ix_add_gate(authority: &Pubkey, wallet: &Pubkey, payer: &Pubkey) -> Instruction {
    let (hook_config, _) = hook_config_pda();
    let (whitelist_state, _) = whitelist_pda(wallet);

    let data = anchor_discriminator("add_gate").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(hook_config, false),
            AccountMeta::new_readonly(*wallet, false),
            AccountMeta::new(whitelist_state, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(
                solana_pubkey::pubkey!("11111111111111111111111111111111"),
                false,
            ),
        ],
        data,
    }
}

pub fn ix_remove_gate(authority: &Pubkey, wallet: &Pubkey, payer: &Pubkey) -> Instruction {
    let (hook_config, _) = hook_config_pda();
    let (whitelist_state, _) = whitelist_pda(wallet);

    let data = anchor_discriminator("remove_gate").to_vec();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(hook_config, false),
            AccountMeta::new_readonly(*wallet, false),
            AccountMeta::new(whitelist_state, false),
            AccountMeta::new(*payer, true),
        ],
        data,
    }
}

pub fn ix_set_whitelist_authority(authority: &Pubkey, new_authority: Pubkey) -> Instruction {
    let (hook_config, _) = hook_config_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        new_authority: Pubkey,
    }

    let mut data = anchor_discriminator("set_whitelist_authority").to_vec();
    Args { new_authority }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(hook_config, false),
        ],
        data,
    }
}

pub fn ix_set_admin(authority: &Pubkey, new_admin: Pubkey) -> Instruction {
    let (hook_config, _) = hook_config_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        new_admin: Pubkey,
    }

    let mut data = anchor_discriminator("set_admin").to_vec();
    Args { new_admin }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new(hook_config, false),
        ],
        data,
    }
}

/// Returns (svm, payer, whitelist_authority, admin)
pub fn setup_initialized() -> (LiteSVM, Keypair, Keypair, Keypair) {
    let (mut svm, payer) = setup();
    let whitelist_authority = Keypair::new();
    let admin = Keypair::new();
    svm.airdrop(&whitelist_authority.pubkey(), 10_000_000_000)
        .unwrap();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_initialize(&admin.pubkey(), whitelist_authority.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    (svm, payer, whitelist_authority, admin)
}

pub fn read_whitelist_role(svm: &LiteSVM, wallet: &Pubkey) -> Option<u8> {
    let (pda, _) = whitelist_pda(wallet);
    let account = svm.get_account(&pda)?;
    if account.data.len() < 10 {
        return None;
    }
    Some(account.data[8])
}

pub fn advance_slot(svm: &mut LiteSVM) {
    let current = svm.get_sysvar::<solana_clock::Clock>().slot;
    svm.warp_to_slot(current + 1);
    svm.expire_blockhash();
}
