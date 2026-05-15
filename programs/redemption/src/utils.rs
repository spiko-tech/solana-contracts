use anchor_lang::prelude::*;

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
