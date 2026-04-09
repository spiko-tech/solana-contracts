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
    state::{TokenConfig, DISCRIMINATOR_TOKEN_CONFIG, MINT_AUTHORITY_SEED, TOKEN_CONFIG_SEED},
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
/// 5. Sends raw CPI to Token-2022 to initialize a MetadataPointer extension on the mint
///    (pointing to the mint itself, so metadata is stored inline)
/// 6. Sends CPI to Token-2022 to InitializeMint2 (sets mint authority + freeze authority to PDA)
/// 7. Sends CPI to Token-2022 to InitializeTokenMetadata (sets name, symbol, URI on the mint)
/// 8. Creates the TokenConfig PDA and stores configuration
///
/// Accounts:
///   0. `[signer, writable]` Admin / deployer (payer)
///   1. `[writable]`         TokenConfig PDA (to be created)
///   2. `[writable]`         Token-2022 Mint account (must be pre-allocated by the caller
///                            with enough space for base mint + extensions + metadata)
///   3. `[]`                 Mint authority PDA (derived from ["mint_authority", mint])
///   4. `[]`                 PermissionManager program ID (stored in config)
///   5. `[]`                 Token-2022 program
///   6. `[]`                 System program
///   7. `[]`                 Transfer Hook program (spiko-transfer-hook)
///
/// Data (after discriminator byte):
///   [0]        decimals (u8)
///   [1..5]     name_len (u32 LE)
///   [5..5+N]   name (UTF-8 bytes)
///   [5+N..9+N] symbol_len (u32 LE)
///   [9+N..9+N+S] symbol (UTF-8 bytes)
///   [9+N+S..13+N+S] uri_len (u32 LE)
///   [13+N+S..13+N+S+U] uri (UTF-8 bytes)
pub struct InitializeToken<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub mint: &'a AccountView,
    pub mint_authority: &'a AccountView,
    pub permission_manager: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub system_program: &'a AccountView,
    pub transfer_hook_program: &'a AccountView,
    pub decimals: u8,
    pub name: &'a [u8],
    pub symbol: &'a [u8],
    pub uri: &'a [u8],
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitializeToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [admin, config, mint, mint_authority, permission_manager, token_2022_program, system_program, transfer_hook_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Parse instruction data: decimals(1) + name(4+N) + symbol(4+S) + uri(4+U)
        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        let decimals = data[0];
        let mut offset = 1;

        // Parse name
        if data.len() < offset + 4 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let name_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if data.len() < offset + name_len {
            return Err(ProgramError::InvalidInstructionData);
        }
        let name = &data[offset..offset + name_len];
        offset += name_len;

        // Parse symbol
        if data.len() < offset + 4 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let symbol_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if data.len() < offset + symbol_len {
            return Err(ProgramError::InvalidInstructionData);
        }
        let symbol = &data[offset..offset + symbol_len];
        offset += symbol_len;

        // Parse uri
        if data.len() < offset + 4 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let uri_len = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if data.len() < offset + uri_len {
            return Err(ProgramError::InvalidInstructionData);
        }
        let uri = &data[offset..offset + uri_len];

        Ok(Self {
            admin,
            config,
            mint,
            mint_authority,
            permission_manager,
            token_2022_program,
            system_program,
            transfer_hook_program,
            decimals,
            name,
            symbol,
            uri,
        })
    }
}

/// SPL Token Metadata Interface discriminator for Initialize:
/// SHA256("spl_token_metadata_interface:initialize_account")[0..8]
const TOKEN_METADATA_INITIALIZE_DISC: [u8; 8] = [210, 225, 30, 162, 88, 184, 77, 141];

impl<'a> InitializeToken<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let mint_key = self.mint.address();

        if self.config.owned_by(program_id) {
            return Err(TokenError::AlreadyInitialized.into());
        }

        let config_bump = verify_pda(
            self.config,
            &[TOKEN_CONFIG_SEED, mint_key.as_ref()],
            program_id,
        )?;

        let mint_auth_bump = verify_pda(
            self.mint_authority,
            &[MINT_AUTHORITY_SEED, mint_key.as_ref()],
            program_id,
        )?;

        // 1. Initialize TransferHook extension
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

        // 2. Initialize PermanentDelegate extension
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

        // 3. Initialize MetadataPointer extension (points to the mint itself)
        {
            let mut ix_data = [0u8; 66]; // 2 + 32 + 32
            ix_data[0] = 39; // TokenInstruction::MetadataPointerExtension
            ix_data[1] = 0; // MetadataPointerInstruction::Initialize
                            // authority = mint_authority PDA
            ix_data[2..34].copy_from_slice(self.mint_authority.address().as_ref());
            // metadata_address = the mint itself (inline metadata)
            ix_data[34..66].copy_from_slice(mint_key.as_ref());

            let account_metas = [InstructionAccount::writable(self.mint.address())];

            let ix = InstructionView {
                program_id: self.token_2022_program.address(),
                accounts: &account_metas,
                data: &ix_data,
            };

            pinocchio::cpi::invoke::<1>(&ix, &[self.mint])?;
        }

        // 4. InitializeMint2 (sets mint authority + freeze authority to PDA)
        let mint_authority_addr = self.mint_authority.address();
        pinocchio_token_2022::instructions::InitializeMint2 {
            mint: self.mint,
            decimals: self.decimals,
            mint_authority: mint_authority_addr,
            freeze_authority: Some(mint_authority_addr),
            token_program: self.token_2022_program.address(),
        }
        .invoke()?;

        // 5. Initialize TokenMetadata on the mint (name, symbol, uri)
        //    This must happen AFTER InitializeMint2 because it requires
        //    the mint authority to be a signer.
        {
            // Build variable-length instruction data:
            // disc(8) + name_len(4) + name + symbol_len(4) + symbol + uri_len(4) + uri
            let total_len = 8 + 4 + self.name.len() + 4 + self.symbol.len() + 4 + self.uri.len();
            let mut ix_data = [0u8; 512]; // max reasonable metadata size
            if total_len > ix_data.len() {
                return Err(ProgramError::InvalidInstructionData);
            }
            ix_data[..8].copy_from_slice(&TOKEN_METADATA_INITIALIZE_DISC);
            let mut off = 8;

            // name (Borsh string: u32 LE length prefix + bytes)
            ix_data[off..off + 4].copy_from_slice(&(self.name.len() as u32).to_le_bytes());
            off += 4;
            ix_data[off..off + self.name.len()].copy_from_slice(self.name);
            off += self.name.len();

            // symbol
            ix_data[off..off + 4].copy_from_slice(&(self.symbol.len() as u32).to_le_bytes());
            off += 4;
            ix_data[off..off + self.symbol.len()].copy_from_slice(self.symbol);
            off += self.symbol.len();

            // uri
            ix_data[off..off + 4].copy_from_slice(&(self.uri.len() as u32).to_le_bytes());
            off += 4;
            ix_data[off..off + self.uri.len()].copy_from_slice(self.uri);
            off += self.uri.len();

            // Accounts for TokenMetadataInitialize:
            //   0. [writable] metadata = mint (inline metadata)
            //   1. []         update_authority = mint_authority PDA
            //   2. []         mint
            //   3. [signer]   mint_authority (PDA signs via invoke_signed)
            //
            // Pinocchio requires a 1:1 mapping between account_views and
            // instruction account metas — duplicates must be repeated.
            let account_metas = [
                InstructionAccount::writable(self.mint.address()),
                InstructionAccount::readonly(self.mint_authority.address()),
                InstructionAccount::readonly(self.mint.address()),
                InstructionAccount::readonly_signer(self.mint_authority.address()),
            ];

            let ix = InstructionView {
                program_id: self.token_2022_program.address(),
                accounts: &account_metas,
                data: &ix_data[..off],
            };

            // Mint authority is a PDA — must sign via CPI
            let mint_auth_bump_bytes = [mint_auth_bump];
            let mint_auth_seeds: [pinocchio::cpi::Seed; 3] = [
                pinocchio::cpi::Seed::from(MINT_AUTHORITY_SEED),
                pinocchio::cpi::Seed::from(mint_key.as_ref()),
                pinocchio::cpi::Seed::from(mint_auth_bump_bytes.as_ref()),
            ];
            let mint_auth_signer = Signer::from(&mint_auth_seeds);

            // 4 account views matching the 4 instruction accounts (1:1):
            //   mint, mint_authority, mint (dup), mint_authority (dup)
            pinocchio::cpi::invoke_signed::<4>(
                &ix,
                &[
                    self.mint,
                    self.mint_authority,
                    self.mint,
                    self.mint_authority,
                ],
                &[mint_auth_signer],
            )?;
        }

        // 6. Create TokenConfig PDA
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
