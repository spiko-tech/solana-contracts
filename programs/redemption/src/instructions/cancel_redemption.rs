use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use crate::{
    error::RedemptionError,
    events::emit_redemption_canceled,
    helpers::{compute_operation_id, cpi_token_2022_transfer, verify_pda},
    state::{
        RedemptionConfig, RedemptionOperation, REDEMPTION_OPERATION_SEED, STATUS_CANCELED,
        STATUS_PENDING, VAULT_SEED,
    },
};

/// Cancel a redemption after the deadline has passed. Anyone can call.
/// Tokens are refunded from the vault to the user.
///
/// Accounts:
///   0. `[signer]`   Anyone (no permission check)
///   1. `[]`         RedemptionConfig PDA
///   2. `[writable]` RedemptionOperation PDA
///   3. `[writable]` Vault token account (source for refund)
///   4. `[writable]` User's token account (destination for refund)
///   5. `[]`         Vault authority PDA
///   6. `[]`         Token-2022 Mint
///   7. `[]`         Token-2022 program
///   --- Transfer Hook extra accounts ---
///   8. `[]`         ExtraAccountMetaList PDA
///   9. `[]`         PermissionManager program
///  10. `[]`         SpikoToken program
///  11. `[]`         TokenConfig PDA (spiko-token)
///  12. `[]`         Vault authority's UserPermissions PDA (sender perms)
///  13. `[]`         User's UserPermissions PDA (recipient perms)
///  14. `[]`         Hook program (spiko-transfer-hook)
///
/// Data:
///   [0..32]  user address (32 bytes)
///   [32..40] amount (u64, little-endian)
///   [40..48] salt (u64, little-endian)
pub struct CancelRedemption<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub redemption_op: &'a AccountView,
    pub vault: &'a AccountView,
    pub user_token_account: &'a AccountView,
    pub vault_authority: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    // Transfer Hook extra accounts
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub vault_authority_perms: &'a AccountView,
    pub user_perms: &'a AccountView,
    pub hook_program: &'a AccountView,
    pub user: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for CancelRedemption<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, redemption_op, vault, user_token_account, vault_authority, token_mint, token_2022_program, extra_account_meta_list, permission_manager_program, spiko_token_program, token_config, vault_authority_perms, user_perms, hook_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.len() < 48 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut user = [0u8; 32];
        user.copy_from_slice(&data[0..32]);
        let amount = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let salt = u64::from_le_bytes(data[40..48].try_into().unwrap());

        Ok(Self {
            caller,
            config,
            redemption_op,
            vault,
            user_token_account,
            vault_authority,
            token_mint,
            token_2022_program,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            token_config,
            vault_authority_perms,
            user_perms,
            hook_program,
            user,
            amount,
            salt,
        })
    }
}

impl<'a> CancelRedemption<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. NO permission check — anyone can call after deadline

        // 2. Read RedemptionConfig (verify ownership)
        {
            if !self.config.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let _config = RedemptionConfig::from_bytes(&data)?;
        }

        // 3. Recompute operation_id and verify RedemptionOperation PDA
        let mint_key_bytes = self.token_mint.address().to_bytes();
        let operation_id =
            compute_operation_id(&self.user, &mint_key_bytes, self.amount, self.salt);

        verify_pda(
            self.redemption_op,
            &[REDEMPTION_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        // 4. Verify vault authority PDA
        let vault_bump = verify_pda(self.vault_authority, &[VAULT_SEED], program_id)?;

        // 5. Check status == PENDING and deadline HAS passed
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        {
            if !self.redemption_op.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.redemption_op.try_borrow()?;
            let op = RedemptionOperation::from_bytes(&data)?;

            if op.status != STATUS_PENDING {
                return Err(RedemptionError::NotPending.into());
            }

            if now <= op.deadline() {
                return Err(RedemptionError::DeadlineNotPassed.into());
            }
        }

        // 6. Transfer tokens from vault back to user (with Transfer Hook extra accounts)
        cpi_token_2022_transfer(
            self.vault,
            self.token_mint,
            self.user_token_account,
            self.vault_authority,
            vault_bump,
            self.token_2022_program,
            self.extra_account_meta_list,
            self.permission_manager_program,
            self.spiko_token_program,
            self.token_config,
            self.vault_authority_perms,
            self.user_perms,
            self.hook_program,
            self.amount,
        )?;

        // 7. Set status = CANCELED
        {
            let mut data = self.redemption_op.try_borrow_mut()?;
            let op = RedemptionOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_CANCELED;
        }

        pinocchio_log::log!("RedemptionCanceled");
        emit_redemption_canceled(
            &self.caller.address().to_bytes(),
            &self.user,
            &self.token_mint.address().to_bytes(),
            self.amount,
            self.salt,
        );

        Ok(())
    }
}
