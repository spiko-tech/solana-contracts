use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use permission_manager::state::ROLE_MINT_APPROVER;

use crate::{
    error::MinterError,
    events::emit_mint_canceled,
    helpers::{compute_operation_id, require_permission, verify_pda},
    state::{MintOperation, MinterConfig, MINT_OPERATION_SEED, STATUS_DONE, STATUS_PENDING},
};

/// Cancel a PENDING mint operation. No tokens are minted.
/// Can cancel if PENDING (regardless of expiry) — the approver can always cancel.
///
/// Accounts:
///   0. `[signer]`   Caller (must have ROLE_MINT_APPROVER)
///   1. `[]`         MinterConfig PDA
///   2. `[writable]` MintOperation PDA
///   3. `[]`         Caller's UserPermissions PDA (from permission_manager)
///
/// Data:
///   [0..32]  user/recipient address (32 bytes)
///   [32..64] token_mint address (32 bytes)
///   [64..72] amount (u64, little-endian)
///   [72..80] salt (u64, little-endian)
pub struct CancelMint<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint_operation: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub user: [u8; 32],
    pub token_mint_key: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for CancelMint<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, mint_operation, caller_perms, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // 32 user + 32 token_mint + 8 amount + 8 salt = 80
        if data.len() < 80 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut user = [0u8; 32];
        user.copy_from_slice(&data[0..32]);
        let mut token_mint_key = [0u8; 32];
        token_mint_key.copy_from_slice(&data[32..64]);
        let amount = u64::from_le_bytes(data[64..72].try_into().unwrap());
        let salt = u64::from_le_bytes(data[72..80].try_into().unwrap());

        Ok(Self {
            caller,
            config,
            mint_operation,
            caller_perms,
            user,
            token_mint_key,
            amount,
            salt,
        })
    }
}

impl<'a> CancelMint<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Read MinterConfig to get permission_manager
        let permission_manager_id = {
            if !self.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            Address::new_from_array(config.permission_manager.to_bytes())
        };

        // 2. Verify caller has ROLE_MINT_APPROVER
        require_permission(
            self.caller_perms,
            &permission_manager_id,
            ROLE_MINT_APPROVER,
        )?;

        // 3. Recompute operation_id and verify MintOperation PDA
        let operation_id =
            compute_operation_id(&self.user, &self.token_mint_key, self.amount, self.salt);

        verify_pda(
            self.mint_operation,
            &[MINT_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        // 4. Verify MintOperation is PENDING (can cancel regardless of deadline)
        {
            if !self.mint_operation.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.mint_operation.try_borrow()?;
            let op = MintOperation::from_bytes(&data)?;

            if op.status != STATUS_PENDING {
                return Err(MinterError::NotPending.into());
            }
        }

        // 5. Set status = DONE
        {
            let mut data = self.mint_operation.try_borrow_mut()?;
            let op = MintOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_DONE;
        }

        pinocchio_log::log!("MintCanceled");
        emit_mint_canceled(
            &self.caller.address().to_bytes(),
            &self.user,
            &self.token_mint_key,
            self.amount,
            self.salt,
        );

        Ok(())
    }
}
