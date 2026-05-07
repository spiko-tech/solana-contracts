use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token_interface::Mint;
use permission_manager::state::PermissionConfig;
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};

use crate::constants::*;
use crate::errors::TransferHookError;
use crate::events::HookInitialized;
use crate::state::HookConfig;

#[derive(Accounts)]
#[event_cpi]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [permission_manager::constants::CONFIG_SEED],
        bump = permission_manager_config.bump,
        seeds::program = permission_manager_program.key(),
        constraint = permission_manager_config.admin == admin.key() @ TransferHookError::Unauthorized,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    /// CHECK: The permission manager program.
    #[account(address = permission_manager::ID)]
    pub permission_manager_program: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + HookConfig::INIT_SPACE,
        seeds = [HOOK_CONFIG_SEED, mint.key().as_ref()],
        bump,
    )]
    pub hook_config: Account<'info, HookConfig>,

    /// CHECK: ExtraAccountMetaList PDA, initialized manually.
    #[account(
        mut,
        seeds = [EXTRA_ACCOUNT_METAS_SEED, mint.key().as_ref()],
        bump,
    )]
    pub extra_account_metas: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Initialize>) -> Result<()> {
    let permission_manager = ctx.accounts.permission_manager_program.key();
    ctx.accounts.hook_config.set_inner(HookConfig {
        paused: false,
        permission_manager,
        bump: ctx.bumps.hook_config,
    });

    let mint_key = ctx.accounts.mint.key();

    let permission_manager_config_seeds: &[Seed] = &[Seed::Literal {
        bytes: b"config".to_vec(),
    }];

    let hook_config_meta = ExtraAccountMeta::new_with_seeds(
        &[
            Seed::Literal {
                bytes: HOOK_CONFIG_SEED.to_vec(),
            },
            Seed::AccountKey { index: 1 },
        ],
        false,
        false,
    )
    .unwrap();

    let permission_manager_config_meta = ExtraAccountMeta::new_external_pda_with_seeds(
        6,
        permission_manager_config_seeds,
        false,
        false,
    )
    .unwrap();

    let source_permissions_meta = ExtraAccountMeta::new_external_pda_with_seeds(
        6,
        &[
            Seed::Literal {
                bytes: b"user_permission".to_vec(),
            },
            Seed::AccountData {
                account_index: 0,
                data_index: 32,
                length: 32,
            },
            Seed::AccountKey { index: 7 },
        ],
        false,
        false,
    )
    .unwrap();

    let destination_permissions_meta = ExtraAccountMeta::new_external_pda_with_seeds(
        6,
        &[
            Seed::Literal {
                bytes: b"user_permission".to_vec(),
            },
            Seed::AccountData {
                account_index: 2,
                data_index: 32,
                length: 32,
            },
            Seed::AccountKey { index: 7 },
        ],
        false,
        false,
    )
    .unwrap();

    let permission_manager_program_meta =
        ExtraAccountMeta::new_with_pubkey(&permission_manager, false, false).unwrap();

    let event_authority_meta = ExtraAccountMeta::new_with_seeds(
        &[Seed::Literal {
            bytes: b"__event_authority".to_vec(),
        }],
        false,
        false,
    )
    .unwrap();

    // The program itself (needed by #[event_cpi])
    let program_meta = ExtraAccountMeta::new_with_pubkey(ctx.program_id, false, false).unwrap();

    let extra_metas = vec![
        hook_config_meta,
        permission_manager_program_meta,
        permission_manager_config_meta,
        source_permissions_meta,
        destination_permissions_meta,
        event_authority_meta,
        program_meta,
    ];

    let account_size = ExtraAccountMetaList::size_of(extra_metas.len()).unwrap();
    let lamports = Rent::get()?.minimum_balance(account_size);

    let extra_metas_bump = ctx.bumps.extra_account_metas;
    let signer_seeds: &[&[u8]] = &[
        EXTRA_ACCOUNT_METAS_SEED,
        mint_key.as_ref(),
        &[extra_metas_bump],
    ];

    system_program::create_account(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.key(),
            system_program::CreateAccount {
                from: ctx.accounts.admin.to_account_info(),
                to: ctx.accounts.extra_account_metas.to_account_info(),
            },
            &[signer_seeds],
        ),
        lamports,
        account_size as u64,
        ctx.program_id,
    )?;

    let mut data = ctx.accounts.extra_account_metas.try_borrow_mut_data()?;
    ExtraAccountMetaList::init::<spl_transfer_hook_interface::instruction::ExecuteInstruction>(
        &mut data,
        &extra_metas,
    )?;

    emit_cpi!(HookInitialized {
        admin: ctx.accounts.admin.key(),
        mint: mint_key,
        permission_manager,
    });

    Ok(())
}
