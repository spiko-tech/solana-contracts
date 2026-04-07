use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use permission_manager::state::ROLE_REDEMPTION_EXECUTOR;

use crate::{
    error::RedemptionError,
    events::emit_redemption_executed,
    helpers::{compute_operation_id, cpi_spiko_token_burn, require_permission, verify_pda},
    state::{
        RedemptionConfig, RedemptionOperation, REDEMPTION_OPERATION_SEED, STATUS_EXECUTED,
        STATUS_PENDING, VAULT_SEED,
    },
};

/// Execute a pending redemption. Burns the held tokens via CPI to spiko_token.burn().
///
/// Accounts:
///   0.  `[signer]`   Operator (must have ROLE_REDEMPTION_EXECUTOR)
///   1.  `[]`         RedemptionConfig PDA
///   2.  `[writable]` RedemptionOperation PDA
///   3.  `[]`         Operator's UserPermissions PDA (from permission_manager)
///   4.  `[]`         Spiko Token program (for CPI: burn)
///   5.  `[]`         TokenConfig PDA (on spiko-token)
///   6.  `[writable]` Token-2022 Mint
///   7.  `[writable]` Vault token account (source for burn)
///   8.  `[]`         Mint authority PDA (spiko-token)
///   9.  `[]`         Vault authority PDA
///  10.  `[]`         Vault authority's UserPermissions PDA (ROLE_BURNER)
///  11.  `[]`         Token-2022 program
///
/// Data:
///   [0..32]  user address (32 bytes)
///   [32..40] amount (u64, little-endian)
///   [40..48] salt (u64, little-endian)
pub struct ExecuteRedemption<'a> {
    pub operator: &'a AccountView,
    pub config: &'a AccountView,
    pub redemption_op: &'a AccountView,
    pub operator_perms: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub vault: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub vault_authority: &'a AccountView,
    pub vault_authority_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub user: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for ExecuteRedemption<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [operator, config, redemption_op, operator_perms, spiko_token_program, token_config, token_mint, vault, mint_authority, vault_authority, vault_authority_perms, token_2022_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !operator.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 32 + 8 + 8 = 48 bytes minimum
        if data.len() < 48 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut user = [0u8; 32];
        user.copy_from_slice(&data[0..32]);
        let amount = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let salt = u64::from_le_bytes(data[40..48].try_into().unwrap());

        Ok(Self {
            operator,
            config,
            redemption_op,
            operator_perms,
            spiko_token_program,
            token_config,
            token_mint,
            vault,
            mint_authority,
            vault_authority,
            vault_authority_perms,
            token_2022_program,
            user,
            amount,
            salt,
        })
    }
}

impl<'a> ExecuteRedemption<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Read RedemptionConfig to get permission_manager
        let permission_manager_id = {
            if !self.config.owned_by(program_id) {
                return Err(RedemptionError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let config = RedemptionConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        // 2. Verify operator has ROLE_REDEMPTION_EXECUTOR
        require_permission(
            self.operator_perms,
            &permission_manager_id,
            ROLE_REDEMPTION_EXECUTOR,
            RedemptionError::Unauthorized.into(),
        )?;

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

        // 5. Check status == PENDING and deadline not passed
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

            if now > op.deadline() {
                return Err(RedemptionError::DeadlinePassed.into());
            }
        }

        // 6. CPI to spiko_token.burn()
        cpi_spiko_token_burn(
            self.vault_authority,
            vault_bump,
            self.token_config,
            self.token_mint,
            self.vault,
            self.mint_authority,
            self.vault_authority_perms,
            self.token_2022_program,
            self.spiko_token_program,
            self.amount,
        )?;

        // 7. Set status = EXECUTED
        {
            let mut data = self.redemption_op.try_borrow_mut()?;
            let op = RedemptionOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_EXECUTED;
        }

        emit_redemption_executed(
            &self.operator.address().to_bytes(),
            &self.user,
            &self.token_mint.address().to_bytes(),
            self.amount,
            self.salt,
        );

        Ok(())
    }
}
