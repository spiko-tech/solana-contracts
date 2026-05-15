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
    solana_pubkey::pubkey!("5Y7mJuJRdBFTXBrXG3rCUZTjRtNKhrRjCA3vKnVX2Zb6");

pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

pub const GATEKEEPER_CONFIG_SEED: &[u8] = b"gatekeeper_config";
pub const VAULT_AUTHORITY_SEED: &[u8] = b"vault_authority";
pub const WITHDRAWAL_DAILY_LIMIT_SEED: &[u8] = b"withdrawal_limit";
pub const GATEKEEP_OPERATION_SEED: &[u8] = b"gatekeep_op";

pub const STATUS_PENDING: u8 = 1;
pub const STATUS_DONE: u8 = 2;
pub const STATUS_CANCELED: u8 = 3;

const MINT_LEN: usize = 82;
const TOKEN_ACCOUNT_LEN: usize = 165;

fn anchor_discriminator(name: &str) -> [u8; 8] {
    let full = format!("global:{}", name);
    let hash = <sha2::Sha256 as sha2::Digest>::digest(full.as_bytes());
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

pub fn gatekeeper_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[GATEKEEPER_CONFIG_SEED], &PROGRAM_ID)
}

pub fn vault_authority_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VAULT_AUTHORITY_SEED], &PROGRAM_ID)
}

pub fn withdrawal_daily_limit_pda(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WITHDRAWAL_DAILY_LIMIT_SEED, mint.as_ref()], &PROGRAM_ID)
}

pub fn gatekeep_operation_pda(salt: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[GATEKEEP_OPERATION_SEED, salt.to_le_bytes().as_ref()],
        &PROGRAM_ID,
    )
}

pub fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let program_bytes = include_bytes!("../../../../target/deploy/custodial_gatekeeper.so");
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

pub fn ix_initialize(admin: &Pubkey, gatekeeper_initiator: Pubkey) -> Instruction {
    let (config, _) = gatekeeper_config_pda();
    let (vault_auth, _) = vault_authority_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        gatekeeper_initiator: Pubkey,
    }

    let mut data = anchor_discriminator("initialize").to_vec();
    Args {
        gatekeeper_initiator,
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

pub fn ix_set_daily_limit(
    admin: &Pubkey,
    mint: &Pubkey,
    payer: &Pubkey,
    limit: u64,
) -> Instruction {
    let (config, _) = gatekeeper_config_pda();
    let (daily_limit, _) = withdrawal_daily_limit_pda(mint);

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

pub fn ix_initiate_gatekeep(
    initiator: &Pubkey,
    payer: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    destination_token_account: &Pubkey,
    salt: u64,
    sender: Pubkey,
    destination: Pubkey,
    amount: u64,
) -> Instruction {
    let (config, _) = gatekeeper_config_pda();
    let (operation, _) = gatekeep_operation_pda(salt);
    let (daily_limit, _) = withdrawal_daily_limit_pda(mint);
    let (vault_auth, _) = vault_authority_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        salt: u64,
        sender: Pubkey,
        destination: Pubkey,
        amount: u64,
    }

    let mut data = anchor_discriminator("initiate_gatekeep").to_vec();
    Args {
        salt,
        sender,
        destination,
        amount,
    }
    .serialize(&mut data)
    .unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*initiator, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(operation, false),
            AccountMeta::new(daily_limit, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(vault_auth, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(*destination_token_account, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_approve_gatekeep(
    admin: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    destination_token_account: &Pubkey,
    salt: u64,
) -> Instruction {
    let (config, _) = gatekeeper_config_pda();
    let (operation, _) = gatekeep_operation_pda(salt);
    let (vault_auth, _) = vault_authority_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        salt: u64,
    }

    let mut data = anchor_discriminator("approve_gatekeep").to_vec();
    Args { salt }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(operation, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(vault_auth, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(*destination_token_account, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_cancel_gatekeep(
    admin: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    sender_token_account: &Pubkey,
    salt: u64,
) -> Instruction {
    let (config, _) = gatekeeper_config_pda();
    let (operation, _) = gatekeep_operation_pda(salt);
    let (vault_auth, _) = vault_authority_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        salt: u64,
    }

    let mut data = anchor_discriminator("cancel_gatekeep").to_vec();
    Args { salt }.serialize(&mut data).unwrap();

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(*admin, true),
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(operation, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(vault_auth, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new(*sender_token_account, false),
            AccountMeta::new_readonly(TOKEN_2022_PROGRAM_ID, false),
        ],
        data,
    }
}

pub fn ix_set_admin(admin: &Pubkey, new_admin: Pubkey) -> Instruction {
    let (config, _) = gatekeeper_config_pda();

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

pub fn ix_set_gatekeeper_initiator(admin: &Pubkey, new_initiator: Pubkey) -> Instruction {
    let (config, _) = gatekeeper_config_pda();

    #[derive(AnchorSerialize)]
    struct Args {
        new_initiator: Pubkey,
    }

    let mut data = anchor_discriminator("set_gatekeeper_initiator").to_vec();
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
    let initiator = Keypair::new();
    svm.airdrop(&admin.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&initiator.pubkey(), 10_000_000_000).unwrap();

    let ix = ix_initialize(&admin.pubkey(), initiator.pubkey());
    send_tx(&mut svm, &[ix], &admin, &[&admin]).unwrap();

    (svm, payer, admin, initiator)
}

// === Token-2022 Helpers ===

pub fn create_fake_mint(svm: &mut LiteSVM, mint: &Pubkey) {
    let (vault_auth, _) = vault_authority_pda();

    let mut data = vec![0u8; MINT_LEN];
    data[0..4].copy_from_slice(&1u32.to_le_bytes());
    data[4..36].copy_from_slice(vault_auth.as_ref());
    data[36..44].copy_from_slice(&1_000_000_000u64.to_le_bytes());
    data[44] = 6;
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
    data[108] = 1; // Initialized

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

/// Read GatekeepOperation status from account data (discriminator[8] + status[1])
pub fn read_operation_status(svm: &LiteSVM, salt: u64) -> Option<u8> {
    let (op, _) = gatekeep_operation_pda(salt);
    let acc = svm.get_account(&op)?;
    if acc.data.len() < 9 {
        return None;
    }
    Some(acc.data[8])
}

/// Full setup: initialized config + mint + vault (owned by vault_authority) + daily limit
///            + sender token account + destination token account.
/// Returns (svm, payer, admin, initiator, mint_pubkey, vault_pubkey, sender_wallet, sender_token, dest_wallet, dest_token)
pub fn setup_with_vault(
    vault_amount: u64,
    limit: u64,
) -> (
    LiteSVM,
    Keypair,
    Keypair,
    Keypair,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
    Pubkey,
) {
    let (mut svm, payer, admin, initiator) = setup_initialized();
    let (vault_auth, _) = vault_authority_pda();

    let mint_pub = Pubkey::new_unique();
    let vault_pub = Pubkey::new_unique();
    let sender_wallet = Pubkey::new_unique();
    let sender_token = Pubkey::new_unique();
    let dest_wallet = Pubkey::new_unique();
    let dest_token = Pubkey::new_unique();

    create_fake_mint(&mut svm, &mint_pub);
    create_fake_token_account(&mut svm, &vault_pub, &mint_pub, &vault_auth, vault_amount);
    create_fake_token_account(&mut svm, &sender_token, &mint_pub, &sender_wallet, 0);
    create_fake_token_account(&mut svm, &dest_token, &mint_pub, &dest_wallet, 0);

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
        vault_pub,
        sender_wallet,
        sender_token,
        dest_wallet,
        dest_token,
    )
}
