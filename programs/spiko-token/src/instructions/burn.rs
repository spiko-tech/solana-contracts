use pinocchio::{
    account::AccountView, address::Address, cpi::Signer, error::ProgramError, ProgramResult,
};

use permission_manager::state::ROLE_BURNER;

use crate::{
    error::TokenError,
    events::emit_burn,
    helpers::{mint_authority_seeds, require_not_paused, require_permission},
    state::{TokenConfig, MINT_AUTHORITY_SEED},
};

/// Burn tokens from the caller's own token account (self-burn).
///
/// Only the BURNER role can call this instruction, and the source token
/// account must be owned by the caller. To burn from any account, use
/// the `BurnFrom` instruction (admin-only, discriminator 8).
///
/// The burn is performed using the mint authority PDA as the authority
/// (PermanentDelegate), with an additional check that the source token
/// account belongs to the caller.
///
/// Accounts:
///   0. `[signer]`   Caller (must have BURNER role)
///   1. `[]`         TokenConfig PDA
///   2. `[writable]` Token-2022 Mint
///   3. `[writable]` Source token account (must be owned by caller)
///   4. `[]`         Mint authority PDA (acts as authority for burn)
///   5. `[]`         Caller's UserPermissions PDA (from permission_manager)
///   6. `[]`         Token-2022 program
///
/// Data:
///   [0..8] amount (u64, little-endian)
pub struct BurnToken<'a> {
    pub caller: &'a AccountView,
    pub config: &'a AccountView,
    pub mint: &'a AccountView,
    pub source_token_account: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub caller_perms: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for BurnToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [caller, config, mint, source_token_account, mint_authority, caller_perms, token_2022_program, ..] =
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
            caller_perms,
            token_2022_program,
            amount,
        })
    }
}

impl<'a> BurnToken<'a> {
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

        // 4. Verify caller has BURNER role
        require_permission(
            self.caller_perms,
            &permission_manager_id,
            ROLE_BURNER,
            TokenError::Unauthorized.into(),
        )?;

        // 5. Verify the source token account is owned by the caller.
        //    In Token-2022 (and SPL Token) the token account layout has
        //    the owner pubkey at bytes [32..64] (right after mint at [0..32]).
        {
            let src_data = self.source_token_account.try_borrow()?;
            if src_data.len() < 64 {
                return Err(TokenError::Unauthorized.into());
            }
            let owner_bytes: &[u8; 32] = src_data[32..64].try_into().unwrap();
            let token_account_owner = Address::new_from_array(*owner_bytes);
            if self.caller.address() != &token_account_owner {
                return Err(TokenError::Unauthorized.into());
            }
        }

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

        emit_burn(
            &self.caller.address().to_bytes(),
            &self.mint.address().to_bytes(),
            &self.source_token_account.address().to_bytes(),
            self.amount,
        );

        Ok(())
    }
}
