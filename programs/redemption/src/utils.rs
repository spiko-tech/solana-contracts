use anchor_lang::prelude::*;
use solana_sha256_hasher::hashv;

use crate::errors::RedemptionError;

pub fn compute_operation_id(user: &Pubkey, mint: &Pubkey, amount: u64, salt: u64) -> [u8; 32] {
    let amount_bytes = amount.to_le_bytes();
    let salt_bytes = salt.to_le_bytes();
    hashv(&[user.as_ref(), mint.as_ref(), &amount_bytes, &salt_bytes]).to_bytes()
}

pub fn verify_operation_id(
    user: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    salt: u64,
    expected: &[u8; 32],
) -> Result<()> {
    let computed = compute_operation_id(user, mint, amount, salt);
    require!(computed == *expected, RedemptionError::InvalidOperationId);
    Ok(())
}

/// Invokes Token-2022 `transfer_checked` with additional hook accounts appended.
///
/// This is necessary because `anchor_spl::token_interface::transfer_checked` ignores
/// `remaining_accounts` in the CPI, so Token-2022 never sees the hook accounts.
pub fn invoke_transfer_checked_with_hook<'info>(
    token_program_key: &Pubkey,
    from: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    authority: &AccountInfo<'info>,
    hook_accounts: &[AccountInfo<'info>],
    amount: u64,
    decimals: u8,
    signer_seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut ix = spl_token_2022_interface::instruction::transfer_checked(
        token_program_key,
        from.key,
        mint.key,
        to.key,
        authority.key,
        &[],
        amount,
        decimals,
    )?;
    for acc in hook_accounts {
        ix.accounts.push(AccountMeta {
            pubkey: *acc.key,
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        });
    }
    let mut account_infos = vec![from.clone(), mint.clone(), to.clone(), authority.clone()];
    account_infos.extend_from_slice(hook_accounts);
    anchor_lang::solana_program::program::invoke_signed(&ix, &account_infos, signer_seeds)?;
    Ok(())
}
