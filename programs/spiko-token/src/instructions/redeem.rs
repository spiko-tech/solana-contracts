use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};

use permission_manager::state::ROLE_WHITELISTED;

use crate::{
    error::TokenError,
    events::emit_redeem_initiated,
    helpers::{require_not_paused, require_permission, token_config_seeds},
    state::{TokenConfig, TOKEN_DECIMALS},
};

/// Redeem tokens: transfer from user to vault, then CPI to Redemption::on_redeem.
///
/// This implements the Stellar pattern where the Token contract orchestrates
/// the redemption flow atomically. The user calls Token::redeem(), which:
///   1. Checks whitelist on user and vault authority
///   2. Transfers tokens from user to vault via Token-2022 (with Transfer Hook extra accounts)
///   3. CPIs to Redemption::on_redeem() to create the PENDING operation
///
/// The TokenConfig PDA signs the CPI to Redemption, proving the call
/// originates from this spiko_token program instance.
///
/// Accounts:
///   0.  `[signer]`   User (token holder)
///   1.  `[]`         TokenConfig PDA (will sign CPI to redemption)
///   2.  `[writable]` User's source token account
///   3.  `[writable]` Vault token account (destination for transfer)
///   4.  `[]`         Token-2022 Mint
///   5.  `[]`         User's UserPermissions PDA (whitelist check)
///   6.  `[]`         Vault authority's UserPermissions PDA (whitelist check)
///   7.  `[]`         Token-2022 program
///   8.  `[]`         Redemption program
///   9.  `[]`         RedemptionConfig PDA (from redemption program)
///  10.  `[writable]` RedemptionOperation PDA (to be created by redemption)
///  11.  `[]`         TokenMinimum PDA (from redemption program)
///  12.  `[]`         System program
///  --- Transfer Hook extra accounts ---
///  13.  `[]`         ExtraAccountMetaList PDA
///  14.  `[]`         PermissionManager program
///  15.  `[]`         SpikoToken program (this program, for hook resolution)
///  16.  `[]`         Transfer Hook program (spiko-transfer-hook)
///
/// Data:
///   [0..8]  amount (u64, little-endian)
///   [8..16] salt (u64, little-endian)
pub struct RedeemToken<'a> {
    pub user: &'a AccountView,
    pub config: &'a AccountView,
    pub user_source: &'a AccountView,
    pub vault: &'a AccountView,
    pub mint: &'a AccountView,
    pub user_perms: &'a AccountView,
    pub vault_authority_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub redemption_program: &'a AccountView,
    pub redemption_config: &'a AccountView,
    pub redemption_op: &'a AccountView,
    pub token_minimum: &'a AccountView,
    pub system_program: &'a AccountView,
    // Transfer Hook extra accounts
    pub extra_account_meta_list: &'a AccountView,
    pub permission_manager_program: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub hook_program: &'a AccountView,
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for RedeemToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [user, config, user_source, vault, mint, user_perms, vault_authority_perms, token_2022_program, redemption_program, redemption_config, redemption_op, token_minimum, system_program, extra_account_meta_list, permission_manager_program, spiko_token_program, hook_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !user.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 8 + 8 = 16 bytes
        if data.len() < 16 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
        let salt = u64::from_le_bytes(data[8..16].try_into().unwrap());

        Ok(Self {
            user,
            config,
            user_source,
            vault,
            mint,
            user_perms,
            vault_authority_perms,
            token_2022_program,
            redemption_program,
            redemption_config,
            redemption_op,
            token_minimum,
            system_program,
            extra_account_meta_list,
            permission_manager_program,
            spiko_token_program,
            hook_program,
            amount,
            salt,
        })
    }
}

impl<'a> RedeemToken<'a> {
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

        // 3. Get permission_manager_id, config bump, and redemption_contract from config
        let (permission_manager_id, config_bump, redemption_contract) = {
            let config_data = self.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;

            // Verify redemption contract has been set
            if !config.has_redemption_contract() {
                return Err(TokenError::RedemptionContractNotSet.into());
            }

            // Verify the redemption program matches the stored contract address
            if self.redemption_program.address() != &config.redemption_contract {
                return Err(TokenError::RedemptionContractMismatch.into());
            }

            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.bump,
                Address::new_from_array(config.redemption_contract.to_bytes()),
            )
        };

        // Suppress unused variable warning — redemption_contract was validated above
        let _ = redemption_contract;

        // 4. Verify user is whitelisted
        require_permission(
            self.user_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TokenError::UnauthorizedFrom.into(),
        )?;

        // 5. Verify vault authority is whitelisted (recipient of transfer)
        require_permission(
            self.vault_authority_perms,
            &permission_manager_id,
            ROLE_WHITELISTED,
            TokenError::UnauthorizedTo.into(),
        )?;

        // 6. CPI to Token-2022: TransferChecked (user -> vault)
        //    Must include Transfer Hook extra accounts so Token-2022 can invoke the hook.
        //
        //    Token-2022 TransferChecked (opcode 12) data:
        //      [0]    = 12 (instruction discriminator)
        //      [1..9] = amount (u64 LE)
        //      [9]    = decimals (u8)
        {
            let mut ix_data = [0u8; 10];
            ix_data[0] = 12; // TransferChecked opcode
            ix_data[1..9].copy_from_slice(&self.amount.to_le_bytes());
            ix_data[9] = TOKEN_DECIMALS;

            // Standard TransferChecked accounts + Transfer Hook extra accounts
            let ix_accounts = [
                // Standard TransferChecked accounts:
                InstructionAccount::writable(self.user_source.address()), // source
                InstructionAccount::readonly(self.mint.address()),        // mint
                InstructionAccount::writable(self.vault.address()),       // destination
                InstructionAccount::readonly_signer(self.user.address()), // authority (user signs)
                // Transfer Hook extra accounts (for Token-2022 to forward to hook):
                InstructionAccount::readonly(self.extra_account_meta_list.address()), // ExtraAccountMetaList
                InstructionAccount::readonly(self.permission_manager_program.address()), // PermissionManager program
                InstructionAccount::readonly(self.spiko_token_program.address()), // SpikoToken program
                InstructionAccount::readonly(self.config.address()),              // TokenConfig PDA
                InstructionAccount::readonly(self.user_perms.address()), // Sender UserPerms
                InstructionAccount::readonly(self.vault_authority_perms.address()), // Recipient UserPerms
                InstructionAccount::readonly(self.hook_program.address()), // Hook program (spiko-transfer-hook)
            ];

            let instruction = InstructionView {
                program_id: self.token_2022_program.address(),
                accounts: &ix_accounts,
                data: &ix_data,
            };

            pinocchio::cpi::invoke(
                &instruction,
                &[
                    self.user_source,
                    self.mint,
                    self.vault,
                    self.user,
                    self.extra_account_meta_list,
                    self.permission_manager_program,
                    self.spiko_token_program,
                    self.config,
                    self.user_perms,
                    self.vault_authority_perms,
                    self.hook_program,
                    self.token_2022_program,
                ],
            )?;
        }

        // 7. CPI to Redemption::on_redeem (discriminator 4)
        //    The TokenConfig PDA signs, proving this CPI originates from spiko_token.
        //
        //    on_redeem accounts:
        //      0. token_config (signer — PDA signer proof)
        //      1. user (signer, writable — payer for operation PDA)
        //      2. redemption_config
        //      3. redemption_op (writable)
        //      4. token_minimum
        //      5. token_mint
        //      6. system_program
        //
        //    on_redeem data: disc(4) + user(32) + amount(8) + salt(8) = 49 bytes
        let user_key_bytes = self.user.address().to_bytes();
        let mut ix_data = [0u8; 49];
        ix_data[0] = 4; // discriminator for on_redeem
        ix_data[1..33].copy_from_slice(&user_key_bytes);
        ix_data[33..41].copy_from_slice(&self.amount.to_le_bytes());
        ix_data[41..49].copy_from_slice(&self.salt.to_le_bytes());

        let ix_accounts = [
            InstructionAccount::readonly_signer(self.config.address()), // token_config (PDA signer)
            InstructionAccount::writable_signer(self.user.address()),   // user (payer)
            InstructionAccount::readonly(self.redemption_config.address()), // redemption_config
            InstructionAccount::writable(self.redemption_op.address()), // redemption_op
            InstructionAccount::readonly(self.token_minimum.address()), // token_minimum
            InstructionAccount::readonly(self.mint.address()),          // token_mint
            InstructionAccount::readonly(self.system_program.address()), // system_program
        ];

        let instruction = InstructionView {
            program_id: self.redemption_program.address(),
            accounts: &ix_accounts,
            data: &ix_data,
        };

        // Sign with TokenConfig PDA
        let mint_key = self.mint.address();
        let bump_bytes = [config_bump];
        let tc_seeds = token_config_seeds(mint_key.as_ref(), &bump_bytes);
        let tc_signer = Signer::from(&tc_seeds);

        pinocchio::cpi::invoke_signed(
            &instruction,
            &[
                self.config,
                self.user,
                self.redemption_config,
                self.redemption_op,
                self.token_minimum,
                self.mint,
                self.system_program,
                self.redemption_program,
            ],
            &[tc_signer],
        )?;

        emit_redeem_initiated(
            &self.user.address().to_bytes(),
            &self.mint.address().to_bytes(),
            self.amount,
            self.salt,
        );

        Ok(())
    }
}
