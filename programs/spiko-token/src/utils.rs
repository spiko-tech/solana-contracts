use anchor_lang::prelude::*;
use solana_sha256_hasher::hashv;

pub fn compute_operation_id(user: &Pubkey, mint: &Pubkey, amount: u64, salt: u64) -> [u8; 32] {
    let amount_bytes = amount.to_le_bytes();
    let salt_bytes = salt.to_le_bytes();
    hashv(&[user.as_ref(), mint.as_ref(), &amount_bytes, &salt_bytes]).to_bytes()
}

/// Builds and invokes a Token-2022 `transfer_checked` instruction with remaining accounts
/// (transfer hook extra accounts) appended.
pub fn invoke_transfer_checked_with_remaining<'info>(
    token_program: &Pubkey,
    source: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    destination: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    amount: u64,
    decimals: u8,
    signer_seeds: &[&[&[u8]]],
    remaining_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    let mut ix = spl_token_2022_interface::instruction::transfer_checked(
        token_program,
        source.key,
        mint.key,
        destination.key,
        authority.key,
        &[],
        amount,
        decimals,
    )?;

    for acc in remaining_accounts {
        ix.accounts.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        });
    }

    let mut account_infos = vec![
        source.clone(),
        mint.clone(),
        destination.clone(),
        authority.clone(),
    ];
    for acc in remaining_accounts {
        account_infos.push(acc.clone());
    }

    if signer_seeds.is_empty() {
        anchor_lang::solana_program::program::invoke(&ix, &account_infos)?;
    } else {
        anchor_lang::solana_program::program::invoke_signed(&ix, &account_infos, signer_seeds)?;
    }

    Ok(())
}
