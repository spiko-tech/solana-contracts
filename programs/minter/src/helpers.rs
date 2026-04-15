use pinocchio::{
    account::AccountView,
    cpi::{Seed, Signer},
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};

pub use spiko_common::{compute_operation_id, create_pda_account, verify_pda};
pub use spiko_permission::{require_admin, require_permission};

use crate::state::{DAILY_LIMIT_SEED, MINTER_CONFIG_SEED, MINT_OPERATION_SEED};

/// Build and invoke CPI to spiko_token program's mint instruction.
///
/// spiko_token mint (discriminator 1) expects accounts:
///   0. `[signer]`   Caller (the minter program — but since CPI, this is the calling account)
///   1. `[]`         TokenConfig PDA (spiko-token)
///   2. `[writable]` Token-2022 Mint
///   3. `[writable]` Recipient's token account
///   4. `[]`         Mint authority PDA (spiko-token)
///   5. `[]`         Caller's UserPermissions PDA (the minter's user-perm with ROLE_MINTER)
///   6. `[]`         Recipient's UserPermissions PDA (whitelist check)
///   7. `[]`         Token-2022 program
///
/// Data: discriminator(1) + amount(u64 LE, 8 bytes) = 9 bytes
///
/// The caller of this CPI must be a signer that has ROLE_MINTER in the permission_manager.
/// In practice, the "caller" account for the CPI is the actual human caller who initiated
/// the minter instruction. But the minter doesn't sign as a PDA for this CPI — the original
/// signer's signature propagates through.
///
/// Wait — actually, the spiko_token mint instruction checks `caller.is_signer()` and verifies
/// ROLE_MINTER for that caller. For the minter program to CPI into spiko_token.mint(), we need
/// a signer that has ROLE_MINTER. This means we need a PDA of the minter program that has
/// been granted ROLE_MINTER in the permission_manager. The MinterConfig PDA could serve this
/// purpose. The minter program would sign as its MinterConfig PDA.
///
/// Account ordering for the CPI:
///   0. MinterConfig PDA [signer via CPI] — must have ROLE_MINTER
///   1. TokenConfig PDA (spiko-token)
///   2. Token-2022 Mint [writable]
///   3. Recipient's token account [writable]
///   4. Mint authority PDA (spiko-token)
///   5. MinterConfig's UserPermissions PDA (proving ROLE_MINTER)
///   6. Recipient's UserPermissions PDA (whitelist)
///   7. Token-2022 program
///   8. Event authority PDA (spiko-token's)
///   9. Self program (spiko-token program)
pub fn cpi_spiko_token_mint<'a>(
    minter_config: &AccountView,
    minter_config_bump: u8,
    token_config: &AccountView,
    token_mint: &AccountView,
    recipient_token_account: &AccountView,
    mint_authority: &AccountView,
    minter_user_perms: &AccountView,
    recipient_perms: &AccountView,
    token_2022_program: &AccountView,
    spiko_token_program: &AccountView,
    st_event_authority: &AccountView,
    st_self_program: &AccountView,
    amount: u64,
) -> ProgramResult {
    // Build instruction data: discriminator(1) + amount(8 bytes LE)
    let mut ix_data = [0u8; 9];
    ix_data[0] = 1; // discriminator for mint
    ix_data[1..9].copy_from_slice(&amount.to_le_bytes());

    // Build CPI instruction accounts
    let ix_accounts = [
        InstructionAccount::writable_signer(minter_config.address()), // caller (PDA signer)
        InstructionAccount::readonly(token_config.address()),         // token config
        InstructionAccount::writable(token_mint.address()),           // token-2022 mint
        InstructionAccount::writable(recipient_token_account.address()), // recipient ATA
        InstructionAccount::readonly(mint_authority.address()),       // mint authority PDA
        InstructionAccount::readonly(minter_user_perms.address()), // minter's user perms (ROLE_MINTER)
        InstructionAccount::readonly(recipient_perms.address()), // recipient's user perms (whitelist)
        InstructionAccount::readonly(token_2022_program.address()), // token-2022 program
        InstructionAccount::readonly(st_event_authority.address()), // spiko-token event authority PDA
        InstructionAccount::readonly(st_self_program.address()), // spiko-token program (self_program)
    ];

    let instruction = InstructionView {
        program_id: spiko_token_program.address(),
        accounts: &ix_accounts,
        data: &ix_data,
    };

    // MinterConfig PDA signs the CPI
    let bump_bytes = [minter_config_bump];
    let seeds = minter_config_seeds(&bump_bytes);
    let signer = Signer::from(&seeds);

    pinocchio::cpi::invoke_signed(
        &instruction,
        &[
            minter_config,
            token_config,
            token_mint,
            recipient_token_account,
            mint_authority,
            minter_user_perms,
            recipient_perms,
            token_2022_program,
            st_event_authority,
            st_self_program,
            spiko_token_program,
        ],
        &[signer],
    )
}

/// Build Seed array for the MinterConfig PDA.
/// Seeds: ["minter_config", bump]
#[inline]
pub fn minter_config_seeds<'a>(bump: &'a [u8; 1]) -> [Seed<'a>; 2] {
    [Seed::from(MINTER_CONFIG_SEED), Seed::from(bump.as_ref())]
}

/// Build Seed array for a DailyLimit PDA.
/// Seeds: ["daily_limit", mint_pubkey, bump]
#[inline]
pub fn daily_limit_seeds<'a>(mint_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(DAILY_LIMIT_SEED),
        Seed::from(mint_key),
        Seed::from(bump.as_ref()),
    ]
}

/// Build Seed array for a MintOperation PDA.
/// Seeds: ["mint_op", operation_id, bump]
#[inline]
pub fn mint_operation_seeds<'a>(operation_id: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(MINT_OPERATION_SEED),
        Seed::from(operation_id),
        Seed::from(bump.as_ref()),
    ]
}
