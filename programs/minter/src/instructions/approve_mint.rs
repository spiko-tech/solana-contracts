use pinocchio::{
    account::AccountView,
    address::Address,
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

use permission_manager::state::ROLE_MINT_APPROVER;

use crate::{
    error::MinterError,
    events::emit_mint_approved,
    helpers::{compute_operation_id, cpi_spiko_token_mint, require_permission, verify_pda},
    state::{MintOperation, MinterConfig, MINT_OPERATION_SEED, STATUS_DONE, STATUS_PENDING},
};

/// Approve a blocked (PENDING) mint operation. Executes the mint via CPI.
///
/// Accounts:
///   0.  `[signer]`            Caller (must have ROLE_MINT_APPROVER)
///   1.  `[]`                  MinterConfig PDA
///   2.  `[writable]`          MintOperation PDA
///   3.  `[]`                  Caller's UserPermissions PDA (from permission_manager)
///   4.  `[]`                  Spiko Token program (for CPI)
///   5.  `[]`                  TokenConfig PDA (on spiko-token, for CPI)
///   6.  `[writable]`          Token-2022 Mint (for CPI)
///   7.  `[writable]`          Recipient's token account (for CPI)
///   8.  `[]`                  Mint authority PDA (spiko-token, for CPI)
///   9.  `[]`                  Minter's UserPermissions PDA (MinterConfig has ROLE_MINTER)
///  10.  `[]`                  Recipient's UserPermissions PDA (whitelist check, for CPI)
///  11.  `[]`                  Token-2022 program
///
/// Data:
///   [0..32]  user/recipient address (32 bytes)
///   [32..64] token_mint address (32 bytes) — needed to recompute operation_id
///   [64..72] amount (u64, little-endian)
///   [72..80] salt (u64, little-endian)
pub struct ApproveMint<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint_operation: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub token_config: &'a AccountView,
    pub token_mint: &'a AccountView,
    pub recipient_token_account: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub minter_user_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub user: [u8; 32],
    pub token_mint_key: [u8; 32],
    pub amount: u64,
    pub salt: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for ApproveMint<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, mint_operation, caller_perms, spiko_token_program, token_config, token_mint, recipient_token_account, mint_authority, minter_user_perms, recipient_perms, token_2022_program, ..] =
            accounts
        else {
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
            spiko_token_program,
            token_config,
            token_mint,
            recipient_token_account,
            mint_authority,
            minter_user_perms,
            recipient_perms,
            token_2022_program,
            user,
            token_mint_key,
            amount,
            salt,
        })
    }
}

impl<'a> ApproveMint<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Read MinterConfig
        let (permission_manager_id, config_bump) = {
            if !self.config.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.config.try_borrow()?;
            let config = MinterConfig::from_bytes(&data)?;
            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.bump,
            )
        };

        // 2. Verify caller has ROLE_MINT_APPROVER
        require_permission(
            self.caller_perms,
            &permission_manager_id,
            ROLE_MINT_APPROVER,
            MinterError::Unauthorized.into(),
        )?;

        // 3. Recompute operation_id and verify MintOperation PDA
        let operation_id =
            compute_operation_id(&self.user, &self.token_mint_key, self.amount, self.salt);

        verify_pda(
            self.mint_operation,
            &[MINT_OPERATION_SEED, &operation_id],
            program_id,
        )?;

        // 4. Verify token_mint account matches the key in instruction data
        if self.token_mint.address().to_bytes() != self.token_mint_key {
            return Err(ProgramError::InvalidArgument);
        }

        // 5. Check MintOperation is PENDING and not expired
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        {
            if !self.mint_operation.owned_by(program_id) {
                return Err(MinterError::NotInitialized.into());
            }
            let data = self.mint_operation.try_borrow()?;
            let op = MintOperation::from_bytes(&data)?;

            if op.status != STATUS_PENDING {
                return Err(MinterError::NotPending.into());
            }

            if now > op.deadline() {
                return Err(MinterError::DeadlinePassed.into());
            }
        }

        // 6. CPI to spiko_token.mint()
        cpi_spiko_token_mint(
            self.config,
            config_bump,
            self.token_config,
            self.token_mint,
            self.recipient_token_account,
            self.mint_authority,
            self.minter_user_perms,
            self.recipient_perms,
            self.token_2022_program,
            self.spiko_token_program,
            self.amount,
        )?;

        // 7. Set status = DONE
        {
            let mut data = self.mint_operation.try_borrow_mut()?;
            let op = MintOperation::from_bytes_mut(&mut data)?;
            op.status = STATUS_DONE;
        }

        pinocchio_log::log!("MintApproved");
        emit_mint_approved(
            &self.caller.address().to_bytes(),
            &self.user,
            &self.token_mint_key,
            self.amount,
            self.salt,
        );

        Ok(())
    }
}
