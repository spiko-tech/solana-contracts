use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use crate::constants::*;
use crate::events::TransferExecuted;
use crate::state::HookConfig;

#[derive(Accounts)]
#[event_cpi]
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
        seeds = [HOOK_CONFIG_SEED, mint.key().as_ref()],
        bump = hook_config.bump,
    )]
    pub hook_config: Account<'info, HookConfig>,

    /// CHECK: Permission manager program.
    #[account(address = hook_config.permission_manager)]
    pub permission_manager_program: UncheckedAccount<'info>,

    /// CHECK: Permission manager config PDA.
    #[account(owner = hook_config.permission_manager)]
    pub permission_manager_config: UncheckedAccount<'info>,

    /// CHECK: Source user permissions PDA on permission manager.
    #[account(owner = hook_config.permission_manager)]
    pub source_permissions: UncheckedAccount<'info>,

    /// CHECK: Destination user permissions PDA on permission manager.
    #[account(owner = hook_config.permission_manager)]
    pub destination_permissions: UncheckedAccount<'info>,
}

fn get_roles(permissions_info: &AccountInfo) -> Result<u16> {
    let data = permissions_info.try_borrow_data()?;
    if data.len() < 8 + 2 {
        return Ok(0);
    }
    Ok(u16::from_le_bytes([data[8], data[8 + 1]]))
}

pub(crate) fn handler(ctx: Context<Execute>, amount: u64) -> Result<()> {
    let source_roles = get_roles(&ctx.accounts.source_permissions)?;
    let dest_roles = get_roles(&ctx.accounts.destination_permissions)?;

    ctx.accounts
        .hook_config
        .validate_transfer(source_roles, dest_roles)?;

    emit_cpi!(TransferExecuted {
        source: ctx.accounts.source.key(),
        destination: ctx.accounts.destination.key(),
        mint: ctx.accounts.mint.key(),
        amount,
    });

    Ok(())
}
