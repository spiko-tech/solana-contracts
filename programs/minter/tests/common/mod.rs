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
    solana_pubkey::pubkey!("Hygpx48FpJyDjW1uW8fykwb94Jmak4CaWvihRREsJyFX");

pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

pub const MINTER_CONFIG_SEED: &[u8] = b"minter_config";
pub const MINT_DAILY_LIMIT_SEED: &[u8] = b"mint_daily_limit";
pub const MINT_OPERATION_SEED: &[u8] = b"mint_op";

const MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;

pub const STATUS_PENDING: u8 = 1;
pub const STATUS_DONE: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;

fn anchor_discriminator(name: &str) -> [u8; 8] {
    let full = format!("global:{}", name);
    let hash = <sha2::Sha256 as sha2::Digest>::digest(full.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

pub fn minter_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINTER_CONFIG_SEED], &PROGRAM_ID)
}

pub fn mint_daily_limit_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINT_DAILY_LIMIT_SEED, mint.as_ref()], &PROGRAM_ID)
}

pub fn mint_operation_pda(salt: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MINT_OPERATION_SEED, salt.to_le_bytes().as_ref()],
        &PROGRAM_ID,
    )
}

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/minter.so");
    svm.add_program(PROGRAM_ID, program_bytes);

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

pub fn ix_initialize(admin: &Pubkey, mint_initiator: Pubkey) -> Instruction {
    let (config, _) = minter_config_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        mint_initiator: Pubkey,
    }

    let mut data = anchor_discriminator("initialize").to_vec();
    Args { mint_initiator }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*admin, true),
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_set_daily_limit(
    admin: &Pubkey,
    mint: &Pubkey,
    payer: &Pubkey,
    limit: u64,
) -> Instruction {
    let (config, _) = minter_config_pda();
    let (daily_limit, _) = mint_daily_limit_pda(mint);

    #[derive(AnchorSerialize)]
    struct Args {
        limit: u64,
    }

    let mut data = anchor_discriminator("set_daily_limit").to_vec();
    Args { limit }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(daily_limit, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_initiate_mint(
    initiator: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    payer: &Pubkey,
    salt: u64,
    recipient: Pubkey,
    amount: u64,
) -> Instruction {
    let (config, _) = minter_config_pda();
    let (daily_limit, _) = mint_daily_limit_pda(mint);
    let (operation, _) = mint_operation_pda(salt);

    #[derive(AnchorSerialize)]
    struct Args {
        salt: u64,
        recipient: Pubkey,
        amount: u64,
    }

    let mut data = anchor_discriminator("initiate_mint").to_vec();
    Args {
        salt,
        recipient,
        amount,
    }
    .serialize(&mut data)
    .unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*initiator, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(daily_limit, false),
            AccountMeta::new(operation, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_approve_mint(
    admin: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    salt: u64,
) -> Instruction {
    let (config, _) = minter_config_pda();
    let (operation, _) = mint_operation_pda(salt);

    #[derive(AnchorSerialize)]
    struct Args {
        salt: u64,
    }

    let mut data = anchor_discriminator("approve_mint").to_vec();
    Args { salt }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(operation, false),
            AccountMeta::new(*mint, false),
            AccountMeta::new(*destination, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_cancel_mint(admin: &Pubkey, salt: u64) -> Instruction {
    let (config, _) = minter_config_pda();
    let (operation, _) = mint_operation_pda(salt);

    #[derive(AnchorSerialize)]
    struct Args {
        salt: u64,
    }

    let mut data = anchor_discriminator("cancel_mint").to_vec();
    Args { salt }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(operation, false),
        ],
        data,
    }
}

pub fn ix_set_admin(admin: &Pubkey, new_admin: Pubkey) -> Instruction {
    let (config, _) = minter_config_pda();

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

pub fn ix_set_mint_initiator(admin: &Pubkey, new_initiator: Pubkey) -> Instruction {
    let (config, _) = minter_config_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        new_initiator: Pubkey,
    }

    let mut data = anchor_discriminator("set_mint_initiator").to_vec();
    Args { new_initiator }.serialize(&mut data).unwrap();

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

pub fn setup_initialized() -> (LiteSVM, Keypair, Keypair, Keypair) {
    let (mut svm, payer) = setup();
    let admin = Keypair::new();
    let mint_initiator = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&mint_initiator.pubkey(), 10_000_000_000)
        .unwrap();

    let ix = ix_initialize(&admin.pubkey(), mint_initiator.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    (svm, payer, admin, mint_initiator)
}

// === Token-2022 Helpers ===

/// Create a fake Token-2022 mint with minter_config PDA as mint authority.
pub fn create_fake_mint(svm: &mut LiteSVM, mint: &Pubkey, supply: u64) {
    let (config, _) = minter_config_pda();

    let mut data = vec![0u8; MINT_LEN];
    // COption<Pubkey> mint_authority: Some(1) + pubkey
    data[0..4].copy_from_slice(&1u32.to_le_bytes());
    data[4..36].copy_from_slice(config.as_ref());
    // supply
    data[36..44].copy_from_slice(&supply.to_le_bytes());
    // decimals
    data[44] = 6;
    // is_initialized
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

/// Create a fake Token-2022 token account.
pub fn create_fake_token_account(
    svm: &mut LiteSVM,
    address: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) {
    let mut data = vec![0u8; TOKEN_ACCOUNT_LEN];
    data[0..32].copy_from_slice(mint.as_ref());
    data[32..64].copy_from_slice(owner.as_ref());
    data[64..72].copy_from_slice(&amount.to_le_bytes());
    // state: Initialized = 1
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

pub fn read_token_balance(svm: &LiteSVM, account: &Pubkey) -> Option<u64> {
    let acc = svm.get_account(account)?;
    if acc.data.len() < 72 {
        return None;
    }
    Some(u64::from_le_bytes(acc.data[64..72].try_into().unwrap()))
}

/// Read MintOperation status from account data (discriminator[8] + status[1])
pub fn read_operation_status(svm: &LiteSVM, salt: u64) -> Option<u8> {
    let (op, _) = mint_operation_pda(salt);
    let acc = svm.get_account(&op)?;
    if acc.data.len() < 9 {
        return None;
    }
    Some(acc.data[8])
}

/// Setup with initialized config + a mint + daily limit set + a destination token account.
/// Returns (svm, payer, admin, initiator, mint_pubkey, destination_pubkey, recipient_pubkey)
pub fn setup_with_mint(limit: u64) -> (LiteSVM, Keypair, Keypair, Keypair, Pubkey, Pubkey, Pubkey) {
    let (mut svm, payer, admin, initiator) = setup_initialized();

    let mint_pub = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();
    let destination = Pubkey::new_unique();

    create_fake_mint(&mut svm, &mint_pub, 0);
    create_fake_token_account(&mut svm, &destination, &mint_pub, &recipient, 0);

    // Set daily limit
    advance_slot(&mut svm);
    let ix = ix_set_daily_limit(&admin.pubkey(), &mint_pub, &admin.pubkey(), limit);
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    (
        svm,
        payer,
        admin,
        initiator,
        mint_pub,
        destination,
        recipient,
    )
}
