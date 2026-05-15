use anchor_lang::AnchorSerialize;
use litesvm::LiteSVM;
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub const PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("B3ustaVazAzqwbgkxARcsL9KKKaNKT6o6FFQyo4b4EBr");

pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

pub const REDEMPTION_CONFIG_SEED: &[u8] = b"redemption_config";
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
pub const REDEMPTION_RECORD_SEED: &[u8] = b"redemption_record";

/// Token-2022 account sizes
const MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;

fn anchor_discriminator(name: &str) -> [u8; 8] {
    let full = format!("global:{}", name);
    let hash = <sha2::Sha256 as sha2::Digest>::digest(full.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

pub fn redemption_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REDEMPTION_CONFIG_SEED], &PROGRAM_ID)
}

pub fn vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn redemption_record_pda(salt: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[REDEMPTION_RECORD_SEED, salt.to_le_bytes().as_ref()],
        &PROGRAM_ID,
    )
}

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/redemption.so");
    svm.add_program(PROGRAM_ID, program_bytes);

    // Load Token-2022 program
    let token_2022_bytes = include_bytes!("../../../../target/deploy/spl_token_2022.so");
    svm.add_program(TOKEN_2022_PROGRAM_ID, token_2022_bytes);

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    (svm, payer)
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

pub fn advance_slot(svm: &mut LiteSVM) {
    let current = svm.get_sysvar::<solana_clock::Clock>().slot;
    svm.warp_to_slot(current + 1);
    svm.expire_blockhash();
}

// === Instruction Builders ===

pub fn ix_initialize(admin: &Pubkey, redemption_authority: Pubkey) -> Instruction {
    let (config, _) = redemption_config_pda();
    let (vault_auth, _) = vault_authority_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        redemption_authority: Pubkey,
    }

    let mut data = anchor_discriminator("initialize").to_vec();
    Args {
        redemption_authority,
    }
    .serialize(&mut data)
    .unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(config, false),
            AccountMeta::new(vault_auth, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_redeem(
    authority: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    payer: &Pubkey,
    salt: u64,
    amount: u64,
) -> Instruction {
    let (config, _) = redemption_config_pda();
    let (vault_auth, _) = vault_authority_pda();
    let (record, _) = redemption_record_pda(salt);

    #[derive(AnchorSerialize)]
    struct Args {
        salt: u64,
        amount: u64,
    }

    let mut data = anchor_discriminator("redeem").to_vec();
    Args { salt, amount }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*authority, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new_readonly(vault_auth, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(record, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_cancel(
    admin: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    user_token_account: &Pubkey,
    payer: &Pubkey,
    salt: u64,
    amount: u64,
    user: Pubkey,
) -> Instruction {
    let (config, _) = redemption_config_pda();
    let (vault_auth, _) = vault_authority_pda();
    let (record, _) = redemption_record_pda(salt);

    #[derive(AnchorSerialize)]
    struct Args {
        salt: u64,
        amount: u64,
        user: Pubkey,
    }

    let mut data = anchor_discriminator("cancel").to_vec();
    Args { salt, amount, user }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(vault_auth, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new(record, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_set_admin(admin: &Pubkey, new_admin: Pubkey) -> Instruction {
    let (config, _) = redemption_config_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        new_admin: Pubkey,
    }

    let mut data = anchor_discriminator("set_admin").to_vec();
    Args { new_admin }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new(config, false),
        ],
        data,
    }
}

pub fn ix_set_redemption_authority(admin: &Pubkey, new_authority: Pubkey) -> Instruction {
    let (config, _) = redemption_config_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        new_authority: Pubkey,
    }

    let mut data = anchor_discriminator("set_redemption_authority").to_vec();
    Args { new_authority }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new(config, false),
        ],
        data,
    }
}

// === Setup Helpers ===

/// Returns (svm, payer, admin, redemption_authority)
pub fn setup_initialized() -> (LiteSVM, Keypair, Keypair, Keypair) {
    let (mut svm, payer) = setup();
    let admin = Keypair::new();
    let redemption_authority = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&redemption_authority.pubkey(), 10_000_000_000)
        .unwrap();

    let ix = ix_initialize(&admin.pubkey(), redemption_authority.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    (svm, payer, admin, redemption_authority)
}

// === Token-2022 Helpers (manual account creation via set_account) ===

/// Create a fake Token-2022 mint via set_account.
/// mint_authority = vault_authority PDA, decimals = 6.
pub fn create_fake_mint(svm: &mut LiteSVM, mint: &Pubkey, supply: u64) {
    let (vault_auth, _) = vault_authority_pda();

    let mut data = vec![0u8; MINT_LEN];
    // [0..4] COption<Pubkey> mint_authority: Some(1)
    data[0..4].copy_from_slice(&1u32.to_le_bytes());
    // [4..36] mint_authority pubkey
    data[4..36].copy_from_slice(vault_auth.as_ref());
    // [36..44] supply
    data[36..44].copy_from_slice(&supply.to_le_bytes());
    // [44] decimals
    data[44] = 6;
    // [45] is_initialized
    data[45] = 1;

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

/// Create a fake Token-2022 token account via set_account.
pub fn create_fake_token_account(
    svm: &mut LiteSVM,
    address: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) {
    let mut data = vec![0u8; TOKEN_ACCOUNT_LEN];
    // [0..32] mint
    data[0..32].copy_from_slice(mint.as_ref());
    // [32..64] owner
    data[32..64].copy_from_slice(owner.as_ref());
    // [64..72] amount
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    // [72..76] delegate COption: None (0)
    // [76..108] delegate pubkey (zeros)
    // [108] state: Initialized = 1
    data[108] = 1;

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

/// Read token balance from a token account
pub fn read_token_balance(svm: &LiteSVM, account: &Pubkey) -> Option<u64> {
    let acc = svm.get_account(account)?;
    if acc.data.len() < 72 {
        return None;
    }
    Some(u64::from_le_bytes(acc.data[64..72].try_into().unwrap()))
}

/// Full setup with initialized config + mint + funded vault.
/// Returns (svm, payer, admin, redemption_authority, mint_pubkey, vault_pubkey)
pub fn setup_with_vault(amount: u64) -> (LiteSVM, Keypair, Keypair, Keypair, Pubkey, Pubkey) {
    let (mut svm, payer, admin, redemption_authority) = setup_initialized();
    let (vault_auth, _) = vault_authority_pda();

    let mint_pub = Pubkey::new_unique();
    let vault_pub = Pubkey::new_unique();

    create_fake_mint(&mut svm, &mint_pub, amount);
    create_fake_token_account(&mut svm, &vault_pub, &mint_pub, &vault_auth, amount);

    (svm, payer, admin, redemption_authority, mint_pub, vault_pub)
}
