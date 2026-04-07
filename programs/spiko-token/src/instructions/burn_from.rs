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
        // 1. Verify TokenConfig is owned by this program
        if !self.config.owned_by(program_id) {
            return Err(TokenError::NotInitialized.into());
        }

        // 2. Check not paused
        {
            let config_data = self.config.try_borrow()?;
            require_not_paused(&config_data)?;
        }

        // 3. Get permission_manager_id and mint_authority_bump from config
        let (permission_manager_id, mint_auth_bump) = {
            let config_data = self.config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;
            (
                Address::new_from_array(config.permission_manager.to_bytes()),
                config.mint_authority_bump,
            )
        };

        // 4. Verify caller is the admin
        require_admin(
            self.caller,
            self.perm_config,
            &permission_manager_id,
            TokenError::Unauthorized.into(),
        )?;

        // 5. No ownership check on source — admin can burn from any account

        // 6. Verify mint authority PDA
        let mint_key = self.mint.address();
        let _ma_bump = crate::helpers::verify_pda(
            self.mint_authority,
            &[MINT_AUTHORITY_SEED, mint_key.as_ref()],
            program_id,
        )?;

        // 7. CPI to Token-2022: Burn (mint authority PDA signs as the authority)
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

        pinocchio_log::log!("Burned tokens from account (admin)");
        emit_burn(
            &self.caller.address().to_bytes(),
            &self.mint.address().to_bytes(),
            &self.source_token_account.address().to_bytes(),
            self.amount,
        );

        Ok(())
    }
}
