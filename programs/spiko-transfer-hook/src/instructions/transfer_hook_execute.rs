use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use permission_manager::state::ROLE_WHITELISTED;
use spiko_token::state::TokenConfig;

use crate::{
    error::TransferHookError,
    events::emit_transfer,
    helpers::{require_not_paused, require_permission},
};

/// Transfer Hook Execute handler.
///
/// Token-2022 calls this instruction automatically whenever a
/// `TransferChecked` is executed on a mint that has the Transfer Hook
/// extension pointing to this program.
///
/// The handler enforces:
///   - Token is not paused (via TokenConfig, owned by spiko-token)
///   - Source wallet is WHITELISTED (via UserPermissions PDA)
///   - Destination wallet is WHITELISTED (via UserPermissions PDA)
///
/// Accounts (supplied by Token-2022 + ExtraAccountMetaList resolution):
///   0. `[]` Source token account
///   1. `[]` Mint
///   2. `[]` Destination token account
///   3. `[]` Source owner / delegate (authority)
///   4. `[]` ExtraAccountMetaList PDA (owned by this program)
///   --- Extra accounts declared in the meta list: ---
///   5. `[]` PermissionManager program (literal)
///   6. `[]` SpikoToken program (literal)
///   7. `[]` TokenConfig PDA (derived from SpikoToken program)
///   8. `[]` Sender's UserPermissions PDA (derived from PermissionManager)
///   9. `[]` Recipient's UserPermissions PDA (derived from PermissionManager)
///
/// Data (from Token-2022):
///   [0..8]  Execute discriminator (sighash): [105, 37, 101, 197, 75, 251, 102, 26]
///   [8..16] amount: u64 (little-endian) — not used by this handler
pub struct TransferHookExecute<'a> {
    pub source: &'a AccountView,
    pub mint: &'a AccountView,
    pub destination: &'a AccountView,
    pub authority: &'a AccountView,
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub sender_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for TransferHookExecute<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        // Token-2022 passes 5 fixed + 5 extra = 10 accounts minimum
        let [source, mint, destination, authority, extra_account_meta_list, permission_manager_program, spiko_token_program, token_config, sender_perms, recipient_perms, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Parse amount from data (u64 LE, first 8 bytes after sighash)
        let amount = if data.len() >= 8 {
            u64::from_le_bytes(data[0..8].try_into().unwrap())
        } else {
            0
        };

        Ok(Self {
            source,
            mint,
            destination,
            authority,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            token_config,
            sender_perms,
            recipient_perms,
            amount,
        })
    }
}

impl<'a> TransferHookExecute<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Verify ExtraAccountMetaList is owned by this program
        if !self.extra_account_meta_list.owned_by(program_id) {
            return Err(ProgramError::IllegalOwner);
        }

        // 2. Verify TokenConfig is owned by the SpikoToken program
        //    (not by this program — TokenConfig belongs to spiko-token)
        if !self
            .token_config
            .owned_by(self.spiko_token_program.address())
        {
            return Err(TransferHookError::NotInitialized.into());
        }

        // 3. Check not paused
        {
            let config_data = self.token_config.try_borrow()?;
            require_not_paused(&config_data)?;
        }

        // 4. Get permission_manager_id from config
        let permission_manager_id = {
            let config_data = self.token_config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        // 5. Verify the permission_manager_program account matches the stored ID
        if self.permission_manager_program.address() != &permission_manager_id {
            return Err(TransferHookError::Unauthorized.into());
        }

        // 6. Verify sender is whitelisted
        require_permission(
            self.sender_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TransferHookError::UnauthorizedFrom.into(),
        )?;

        // 7. Verify recipient is whitelisted
        require_permission(
            self.recipient_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TransferHookError::UnauthorizedTo.into(),
        )?;

        pinocchio_log::log!("Transfer hook: whitelist check passed");

        emit_transfer(
            &self.authority.address().to_bytes(),
            &self.mint.address().to_bytes(),
            &self.source.address().to_bytes(),
            &self.destination.address().to_bytes(),
            self.amount,
        );

        Ok(())
    }
}
