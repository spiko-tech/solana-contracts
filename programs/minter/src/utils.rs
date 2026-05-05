use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hashv;

use crate::errors::MinterError;

pub fn compute_operation_id(user: &Pubkey, mint: &Pubkey, amount: u64, salt: u64) -> [u8; 32] {
    let amount_bytes = amount.to_le_bytes();
    let salt_bytes = salt.to_le_bytes();
    hashv(&[user.as_ref(), mint.as_ref(), &amount_bytes, &salt_bytes]).to_bytes()
}

pub fn verify_operation_id(
    recipient: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    salt: u64,
    expected: &[u8; 32],
) -> Result<()> {
    let computed = compute_operation_id(recipient, mint, amount, salt);
    require!(computed == *expected, MinterError::InvalidOperationId);
    Ok(())
}
