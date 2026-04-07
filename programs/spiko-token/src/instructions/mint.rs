use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use permission_manager::state::ROLE_MINTER;

use crate::{
    error::TokenError,
    events::emit_mint,
    helpers::{mint_authority_seeds, require_not_paused, require_permission},
    state::{TokenConfig, MINT_AUTHORITY_SEED},
};

/// Mint tokens to a whitelisted recipient.
///
/// Accounts:
///   0. `[signer]`   Caller (must have MINTER role)
///   1. `[]`         TokenConfig PDA
///   2. `[writable]` Token-2022 Mint
///   3. `[writable]` Recipient's token account
///   4. `[]`         Mint authority PDA
///   5. `[]`         Caller's UserPermissions PDA (from permission_manager)
///   6. `[]`         Recipient's UserPermissions PDA (whitelist check)
///   7. `[]`         Token-2022 program
///
/// Data:
///   [0..8] amount (u64, little-endian)
pub struct MintToken<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint: &'a AccountView,
    pub recipient_token_account: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub recipient_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for MintToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, mint, recipient_token_account, mint_authority, caller_perms, recipient_perms, token_2022_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !caller.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.len() < 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());

        Ok(Self {
            caller,
            config,
            mint,
            recipient_token_account,
            mint_authority,
            caller_perms,
            recipient_perms,
            token_2022_program,
            amount,
        })
    }
}

impl<'a> MintToken<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        if !self.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        {
            let config_data = self.config.try_borrow()?;
            require_not_paused(&config_data)?;
        }

        let (permission_manager_id, mint_auth_bump) = {
            let config_data = self.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;
            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.mint_authority_bump,
            )
        };

        require_permission(
            self.caller_perms,
            &permission_manager_id,
            ROLE_MINTER,
            TokenError::Unauthorized.into(),
        )?;

        require_permission(
            self.recipient_perms,
            &permission_manager_id,
            permission_manager::state::ROLE_WHITELISTED,
            TokenError::UnauthorizedTo.into(),
        )?;

        let mint_key = self.mint.address();
        let _ma_bump = crate::helpers::verify_pda(
            self.mint_authority,
            &[MINT_AUTHORITY_SEED, mint_key.as_ref()],
            program_id,
        )?;

        let bump_bytes = [mint_auth_bump];
        let ma_seeds = mint_authority_seeds(mint_key.as_ref(), &bump_bytes);
        let ma_signer = Signer::from(&ma_seeds);

        pinocchio_token_2022::instructions::MintTo {
            mint: self.mint,
            account: self.recipient_token_account,
            mint_authority: self.mint_authority,
            amount: self.amount,
            token_program: self.token_2022_program.address(),
        }
        .invoke_signed(&[ma_signer])?;

        emit_mint(
            &self.caller.address().to_bytes(),
            &self.mint.address().to_bytes(),
            &self.recipient_token_account.address().to_bytes(),
            self.amount,
        );

        Ok(())
    }
}
