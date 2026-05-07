use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::HookConfig;

#[derive(Accounts)]
pub struct PauseHook<'info> {
    /// The original caller (admin/pauser). Auth is checked by spiko-token before CPI.
    pub caller: Signer<'info>,

    /// spiko-token MintAuthority PDA — must be a signer, enforcing CPI-only access.
    pub mint_authority: Signer<'info>,

    /// CHECK: The mint for which this hook is configured.
    pub mint: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [HOOK_CONFIG_SEED, mint.key().as_ref()],
        bump = hook_config.bump,
    )]
    pub hook_config: Account<'info, HookConfig>,
}

pub(crate) fn handler(ctx: Context<PauseHook>) -> Result<()> {
    ctx.accounts.hook_config.paused = true;
    Ok(())
}
