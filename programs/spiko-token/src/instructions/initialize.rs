use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};

use crate::{
    error::TokenError,
    events::emit_token_initialized,
    helpers::{create_pda_account, token_config_seeds, verify_pda},
    state::{
        TokenConfig, DISCRIMINATOR_TOKEN_CONFIG, MINT_AUTHORITY_SEED, TOKEN_CONFIG_SEED,
        TOKEN_DECIMALS,
    },
};

/// Initialize a new Spiko fund token (Token-2022 mint + config).
///
/// This instruction:
/// 1. Derives and verifies the TokenConfig PDA
/// 2. Derives the MintAuthority PDA
/// 3. Sends raw CPI to Token-2022 to initialize a Transfer Hook extension on the mint
///    (pointing to the separate spiko-transfer-hook program)
/// 4. Sends raw CPI to Token-2022 to initialize a PermanentDelegate extension on the mint
///    (delegate = MintAuthority PDA, enabling admin-forced burns from any account)
/// 5. Sends CPI to Token-2022 to InitializeMint2 (sets mint authority + freeze authority to PDA)
/// 6. Creates the TokenConfig PDA and stores configuration
///
/// Accounts:
///   0. `[signer, writable]` Admin / deployer (payer)
///   1. `[writable]`         TokenConfig PDA (to be created)
///   2. `[writable]`         Token-2022 Mint account (must be pre-allocated by the caller
///                            with enough space for base mint + extensions)
///   3. `[]`                 Mint authority PDA (derived from ["mint_authority", mint])
///   4. `[]`                 PermissionManager program ID (stored in config)
///   5. `[]`                 Token-2022 program
///   6. `[]`                 System program
///   7. `[]`                 Transfer Hook program (spiko-transfer-hook)
///
/// Data (after discriminator byte): (none)
pub struct InitializeToken<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub mint: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub permission_manager: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub system_program: &'a AccountView,
    pub transfer_hook_program: &'a AccountView,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitializeToken<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [admin, config, mint, mint_authority, permission_manager, token_2022_program, system_program, transfer_hook_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            admin,
            config,
            mint,
            mint_authority,
            permission_manager,
            token_2022_program,
            system_program,
            transfer_hook_program,
        })
    }
}

impl<'a> InitializeToken<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let mint_key = self.mint.address();

        // 1. Verify TokenConfig PDA is not yet initialized
        if self.config.owned_by(program_id) {
            return Err(TokenError::AlreadyInitialized.into());
        }

        // 2. Derive TokenConfig PDA: ["token_config", mint_pubkey]
        let config_bump = verify_pda(
            self.config,
            &[TOKEN_CONFIG_SEED, mint_key.as_ref()],
            program_id,
        )?;

        // 3. Derive MintAuthority PDA: ["mint_authority", mint_pubkey]
        let mint_auth_bump = verify_pda(
            self.mint_authority,
            &[MINT_AUTHORITY_SEED, mint_key.as_ref()],
            program_id,
        )?;

        // 4. Initialize the Transfer Hook extension on the mint via raw CPI.
        //    Token-2022 extension instruction namespace:
        //      byte[0] = 36 (ExtensionInstruction)
        //      byte[1] = 0 (InitializeTransferHook)
        //    Followed by: authority (32 bytes, Pubkey), program_id (32 bytes, Pubkey)
        {
            let mut ix_data = [0u8; 66]; // 2 + 32 + 32
            ix_data[0] = 36; // ExtensionInstruction
            ix_data[1] = 0; // InitializeTransferHook
                            // authority = mint_authority PDA
            ix_data[2..34].copy_from_slice(self.mint_authority.address().as_ref());
            // program_id = the separate spiko-transfer-hook program (the hook handler)
            ix_data[34..66].copy_from_slice(self.transfer_hook_program.address().as_ref());

            // Accounts: [writable] mint
            let account_metas = [InstructionAccount::writable(self.mint.address())];

            let ix = InstructionView {
                program_id: self.token_2022_program.address(),
                accounts: &account_metas,
                data: &ix_data,
            };

            pinocchio::cpi::invoke::<1>(&ix, &[self.mint])?;
        }

        // 5. Initialize the PermanentDelegate extension on the mint via raw CPI.
        //    Token-2022 instruction: byte[0] = 35 (InitializePermanentDelegate)
        //    Followed by: delegate (32 bytes, Pubkey = MintAuthority PDA)
        //    This allows the MintAuthority PDA to burn tokens from any account.
        {
            let mut ix_data = [0u8; 33]; // 1 + 32
            ix_data[0] = 35; // InitializePermanentDelegate
            ix_data[1..33].copy_from_slice(self.mint_authority.address().as_ref());

            // Accounts: [writable] mint
            let account_metas = [InstructionAccount::writable(self.mint.address())];

            let ix = InstructionView {
                program_id: self.token_2022_program.address(),
                accounts: &account_metas,
                data: &ix_data,
            };

            pinocchio::cpi::invoke::<1>(&ix, &[self.mint])?;
        }

        // 6. InitializeMint2 via pinocchio-token-2022 CPI
        let mint_authority_addr = self.mint_authority.address();
        pinocchio_token_2022::instructions::InitializeMint2 {
            mint: self.mint,
            decimals: TOKEN_DECIMALS,
            mint_authority: mint_authority_addr,
            freeze_authority: Some(mint_authority_addr),
            token_program: self.token_2022_program.address(),
        }
        .invoke()?;

        // 7. Create TokenConfig PDA
        let config_bump_bytes = [config_bump];
        let config_seeds = token_config_seeds(mint_key.as_ref(), &config_bump_bytes);
        let config_signer = Signer::from(&config_seeds);

        create_pda_account(
            self.admin,
            self.config,
            TokenConfig::LEN,
            program_id,
            &[config_signer],
        )?;

        // 8. Write config data
        {
            let mut data = self.config.try_borrow_mut()?;
            let config = TokenConfig::from_bytes_mut(&mut data)?;
            config.discriminator = DISCRIMINATOR_TOKEN_CONFIG;
            config.bump = config_bump;
            config.paused = 0;
            config.mint_authority_bump = mint_auth_bump;
            config.permission_manager =
                Address::new_from_array(self.permission_manager.address().to_bytes());
            config.spl_mint = Address::new_from_array(mint_key.to_bytes());
            config.redemption_contract = Address::new_from_array([0u8; 32]); // not set initially
        }

        emit_token_initialized(
            &self.admin.address().to_bytes(),
            &self.mint.address().to_bytes(),
        );

        Ok(())
    }
}
