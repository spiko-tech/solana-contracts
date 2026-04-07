use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use crate::{
    error::TokenError,
    events::emit_burn,
    helpers::{mint_authority_seeds, require_admin, require_not_paused},
    state::{TokenConfig, MINT_AUTHORITY_SEED},
};

/// Burn tokens from any token account (admin-only).
///
/// This instruction allows the admin to burn tokens from any token
/// account, regardless of ownership. It uses the PermanentDelegate
/// (mint authority PDA) to sign the Token-2022 burn CPI.
///
/// For self-burn (caller burns their own tokens), use the `BurnToken`
/// instruction (discriminator 2, BURNER role required).
///
/// Accounts:
///   0. `[signer]`   Caller (must be admin)
///   1. `[]`         TokenConfig PDA
///   2. `[writable]` Token-2022 Mint
///   3. `[writable]` Source token account (any account — admin can burn from anyone)
///   4. `[]`         Mint authority PDA (acts as authority for burn via PermanentDelegate)
///   5. `[]`         PermissionConfig PDA (from permission_manager, proves admin identity)
///   6. `[]`         Token-2022 program
///
/// Data:
///   [0..8] amount (u64, little-endian)
pub struct BurnFrom<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint: &'a AccountView,
    pub source_token_account: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub perm_config: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for BurnFrom<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, mint, source_token_account, mint_authority, perm_config, token_2022_program, ..] =
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
            source_token_account,
            mint_authority,
            perm_config,
            token_2022_program,
            amount,
        })
    }
}

impl<'a> BurnFrom<'a> {
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

        require_admin(
            self.caller,
            self.perm_config,
            &permission_manager_id,
            TokenError::Unauthorized.into(),
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

        pinocchio_token_2022::instructions::Burn {
            account: self.source_token_account,
            mint: self.mint,
            authority: self.mint_authority,
            amount: self.amount,
            token_program: self.token_2022_program.address(),
        }
        .invoke_signed(&[ma_signer])?;

        emit_burn(
            &self.caller.address().to_bytes(),
            &self.mint.address().to_bytes(),
            &self.source_token_account.address().to_bytes(),
            self.amount,
        );

        Ok(())
    }
}
