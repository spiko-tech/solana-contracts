use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};

use permission_manager::state::ROLE_WHITELISTED;

use crate::{
    error::TokenError,
    helpers::{require_not_paused, require_permission},
    state::{TokenConfig, TOKEN_DECIMALS},
};

/// Transfer tokens between whitelisted accounts.
///
/// This instruction is the program-mediated transfer that enforces
/// whitelist checks. With Token-2022 Transfer Hook, this will also be
/// invoked automatically on direct SPL transfers.
///
/// Accounts:
///   0. `[signer]`   Sender (owner of source token account)
///   1. `[]`         TokenConfig PDA
///   2. `[writable]` Source token account
///   3. `[writable]` Destination token account
///   4. `[]`         Token-2022 Mint (needed for TransferChecked)
///   5. `[]`         Sender's UserPermissions PDA (whitelist check)
///   6. `[]`         Recipient's UserPermissions PDA (whitelist check)
///   7. `[]`         Token-2022 program
///   --- Transfer Hook extra accounts ---
///   8. `[]`         ExtraAccountMetaList PDA
///   9. `[]`         PermissionManager program
///  10. `[]`         SpikoToken program (this program, for Token-2022 hook resolution)
///  11. `[]`         Transfer Hook program (spiko-transfer-hook)
///
/// Data:
///   [0..8] amount (u64, little-endian)
pub struct TransferToken<'a> {
    pub sender: &'a AccountView,
    pub config: &'a AccountView,
    pub source: &'a AccountView,
    pub destination: &'a AccountView,
    pub mint: &'a AccountView,
    pub sender_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    // Transfer Hook extra accounts
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub hook_program: &'a AccountView,
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for TransferToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [sender, config, source, destination, mint, sender_perms, recipient_perms, token_2022_program, extra_account_meta_list, permission_manager_program, spiko_token_program, hook_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !sender.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.len() < 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());

        Ok(Self {
            sender,
            config,
            source,
            destination,
            mint,
            sender_perms,
            recipient_perms,
            token_2022_program,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            hook_program,
            amount,
        })
    }
}

impl<'a> TransferToken<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Verify TokenConfig is owned by this program
        if !self.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        // 2. Check not paused
        {
            let config_data = self.config.try_borrow()?;
            require_not_paused(&config_data)?;
        }

        // 3. Get permission_manager_id from config
        let permission_manager_id = {
            let config_data = self.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        // 4. Verify sender is whitelisted
        require_permission(
            self.sender_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TokenError::UnauthorizedFrom.into(),
        )?;

        // 5. Verify recipient is whitelisted
        require_permission(
            self.recipient_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TokenError::UnauthorizedTo.into(),
        )?;

        // 6. CPI to Token-2022: TransferChecked with Transfer Hook extra accounts
        //    Token-2022 TransferChecked (opcode 12) data:
        //      [0]    = 12 (instruction discriminator)
        //      [1..9] = amount (u64 LE)
        //      [9]    = decimals (u8)
        {
            let mut ix_data = [0u8; 10];
            ix_data[0] = 12; // TransferChecked opcode
            ix_data[1..9].copy_from_slice(&self.amount.to_le_bytes());
            ix_data[9] = TOKEN_DECIMALS;

            let ix_accounts = [
                // Standard TransferChecked accounts:
                InstructionAccount::writable(self.source.address()), // source
                InstructionAccount::readonly(self.mint.address()),   // mint
                InstructionAccount::writable(self.destination.address()), // destination
                InstructionAccount::readonly_signer(self.sender.address()), // authority
                // Transfer Hook extra accounts:
                InstructionAccount::readonly(self.extra_account_meta_list.address()),
                InstructionAccount::readonly(self.permission_manager_program.address()),
                InstructionAccount::readonly(self.spiko_token_program.address()), // SpikoToken program
                InstructionAccount::readonly(self.config.address()),              // TokenConfig
                InstructionAccount::readonly(self.sender_perms.address()), // Sender UserPerms
                InstructionAccount::readonly(self.recipient_perms.address()), // Recipient UserPerms
                InstructionAccount::readonly(self.hook_program.address()), // Hook program
            ];

            let instruction = InstructionView {
                program_id: self.token_2022_program.address(),
                accounts: &ix_accounts,
                data: &ix_data,
            };

            pinocchio::cpi::invoke(
                &instruction,
                &[
                    self.source,
                    self.mint,
                    self.destination,
                    self.sender,
                    self.extra_account_meta_list,
                    self.permission_manager_program,
                    self.spiko_token_program,
                    self.config,
                    self.sender_perms,
                    self.recipient_perms,
                    self.hook_program,
                    self.token_2022_program,
                ],
            )?;
        }

        pinocchio_log::log!("Transferred tokens");

        Ok(())
    }
}
