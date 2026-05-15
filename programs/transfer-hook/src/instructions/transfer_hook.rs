use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use crate::constants::*;
use crate::state::HookConfig;

#[derive(Accounts)]
pub struct Execute<'info> {
    /// CHECK: Source token account.
    pub source: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Mint.
    pub mint: UncheckedAccount<'info>,

    /// CHECK: Destination token account.
    pub destination: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Source authority / owner.
    pub source_authority: UncheckedAccount<'info>,

    /// CHECK: ExtraAccountMetaList PDA.
    #[account(
        seeds = [EXTRA_ACCOUNT_METAS_SEED, mint.key().as_ref()],
        bump,
    )]
    pub extra_account_metas: UncheckedAccount<'info>,

    #[account(
        seeds = [HOOK_CONFIG_SEED],
        bump = hook_config.bump,
    )]
    pub hook_config: Account<'info, HookConfig>,

    /// CHECK: Source WhitelistState PDA. May not exist (treated as None role).
    #[account(
        seeds = [WHITELIST_SEED, source.owner.as_ref()],
        bump,
    )]
    pub source_whitelist: UncheckedAccount<'info>,

    /// CHECK: Destination WhitelistState PDA. May not exist (treated as None role).
    #[account(
        seeds = [WHITELIST_SEED, destination.owner.as_ref()],
        bump,
    )]
    pub dest_whitelist: UncheckedAccount<'info>,
}

fn get_role(account: &AccountInfo) -> u8 {
    let data = match account.try_borrow_data() {
        Ok(d) => d,
        Err(_) => return 0,
    };
    // Account layout: 8 byte discriminator + 1 byte role + 1 byte bump
    if data.len() < 9 {
        return 0;
    }
    data[8]
}

pub(crate) fn handler(ctx: Context<Execute>, _amount: u64) -> Result<()> {
    let source_role = get_role(&ctx.accounts.source_whitelist);
    let dest_role = get_role(&ctx.accounts.dest_whitelist);

    ctx.accounts
        .hook_config
        .validate_transfer(source_role, dest_role)?;

    Ok(())
}
