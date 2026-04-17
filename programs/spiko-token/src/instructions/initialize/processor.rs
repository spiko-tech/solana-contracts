use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::Signer,
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
    ProgramResult,
};

use spiko_events::EventSerialize;

use crate::{
    error::TokenError,
    events::TokenInitializedEvent,
    helpers::{create_pda_account, token_config_seeds, verify_pda},
    state::{TokenConfig, MINT_AUTHORITY_SEED, TOKEN_CONFIG_SEED},
};

use spiko_common::{AccountDeserialize, AccountSize};

use super::accounts::InitializeTokenAccounts;
use super::data::InitializeTokenData;

/// SPL Token Metadata Interface discriminator for Initialize:
/// SHA256("spl_token_metadata_interface:initialize_account")[0..8]
const TOKEN_METADATA_INITIALIZE_DISC: [u8; 8] = [210, 225, 30, 162, 88, 184, 77, 141];

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
pub struct InitializeToken<'a> {
    pub accounts: InitializeTokenAccounts<'a>,
    pub data: InitializeTokenData<'a>,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitializeToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: InitializeTokenAccounts::try_from(accounts)?,
            data: InitializeTokenData::try_from(data)?,
        })
    }
}

impl<'a> InitializeToken<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let mint_key = self.accounts.mint.address();

        if self.accounts.config.owned_by(program_id) {
            return Err(TokenError::AlreadyInitialized.into());
        }

        let config_bump = verify_pda(
            self.accounts.config,
            &[TOKEN_CONFIG_SEED, mint_key.as_ref()],
            program_id,
        )?;

        let mint_auth_bump = verify_pda(
            self.accounts.mint_authority,
            &[MINT_AUTHORITY_SEED, mint_key.as_ref()],
            program_id,
        )?;

        {
            let mut ix_data = [0u8; 66]; // 2 + 32 + 32
            ix_data[0] = 36; // ExtensionInstruction
            ix_data[1] = 0; // InitializeTransferHook
                            // authority = mint_authority PDA
            ix_data[2..34].copy_from_slice(self.accounts.mint_authority.address().as_ref());
            // program_id = the separate spiko-transfer-hook program (the hook handler)
            ix_data[34..66].copy_from_slice(self.accounts.transfer_hook_program.address().as_ref());

            let account_metas = [InstructionAccount::writable(self.accounts.mint.address())];

            let ix = InstructionView {
                program_id: self.accounts.token_2022_program.address(),
                accounts: &account_metas,
                data: &ix_data,
            };

            pinocchio::cpi::invoke::<1>(&ix, &[self.accounts.mint])?;
        }

        {
            let mut ix_data = [0u8; 33]; // 1 + 32
            ix_data[0] = 35; // InitializePermanentDelegate
            ix_data[1..33].copy_from_slice(self.accounts.mint_authority.address().as_ref());

            let account_metas = [InstructionAccount::writable(self.accounts.mint.address())];

            let ix = InstructionView {
                program_id: self.accounts.token_2022_program.address(),
                accounts: &account_metas,
                data: &ix_data,
            };

            pinocchio::cpi::invoke::<1>(&ix, &[self.accounts.mint])?;
        }
        {
            let mut ix_data = [0u8; 66]; // 2 + 32 + 32
            ix_data[0] = 39; // TokenInstruction::MetadataPointerExtension
            ix_data[1] = 0; // MetadataPointerInstruction::Initialize
                            // authority = mint_authority PDA
            ix_data[2..34].copy_from_slice(self.accounts.mint_authority.address().as_ref());
            // metadata_address = the mint itself (inline metadata)
            ix_data[34..66].copy_from_slice(mint_key.as_ref());

            let account_metas = [InstructionAccount::writable(self.accounts.mint.address())];

            let ix = InstructionView {
                program_id: self.accounts.token_2022_program.address(),
                accounts: &account_metas,
                data: &ix_data,
            };

            pinocchio::cpi::invoke::<1>(&ix, &[self.accounts.mint])?;
        }

        let mint_authority_addr = self.accounts.mint_authority.address();
        pinocchio_token_2022::instructions::InitializeMint2 {
            mint: self.accounts.mint,
            decimals: self.data.decimals,
            mint_authority: mint_authority_addr,
            freeze_authority: Some(mint_authority_addr),
            token_program: self.accounts.token_2022_program.address(),
        }
        .invoke()?;

        // Must happen AFTER InitializeMint2 because it requires
        // the mint authority to be a signer.
        {
            let total_len =
                8 + 4 + self.data.name.len() + 4 + self.data.symbol.len() + 4 + self.data.uri.len();
            let mut ix_data = [0u8; 512]; // max reasonable metadata size
            if total_len > ix_data.len() {
                return Err(ProgramError::InvalidInstructionData);
            }
            ix_data[..8].copy_from_slice(&TOKEN_METADATA_INITIALIZE_DISC);
            let mut off = 8;

            // name (Borsh string: u32 LE length prefix + bytes)
            ix_data[off..off + 4].copy_from_slice(&(self.data.name.len() as u32).to_le_bytes());
            off += 4;
            ix_data[off..off + self.data.name.len()].copy_from_slice(self.data.name);
            off += self.data.name.len();

            ix_data[off..off + 4].copy_from_slice(&(self.data.symbol.len() as u32).to_le_bytes());
            off += 4;
            ix_data[off..off + self.data.symbol.len()].copy_from_slice(self.data.symbol);
            off += self.data.symbol.len();

            ix_data[off..off + 4].copy_from_slice(&(self.data.uri.len() as u32).to_le_bytes());
            off += 4;
            ix_data[off..off + self.data.uri.len()].copy_from_slice(self.data.uri);
            off += self.data.uri.len();

            // Accounts for TokenMetadataInitialize:
            //   0. [writable] metadata = mint (inline metadata)
            //   1. []         update_authority = mint_authority PDA
            //   2. []         mint
            //   3. [signer]   mint_authority (PDA signs via invoke_signed)
            //
            // Pinocchio requires a 1:1 mapping between account_views and
            // instruction account metas — duplicates must be repeated.
            let account_metas = [
                InstructionAccount::writable(self.accounts.mint.address()),
                InstructionAccount::readonly(self.accounts.mint_authority.address()),
                InstructionAccount::readonly(self.accounts.mint.address()),
                InstructionAccount::readonly_signer(self.accounts.mint_authority.address()),
            ];

            let ix = InstructionView {
                program_id: self.accounts.token_2022_program.address(),
                accounts: &account_metas,
                data: &ix_data[..off],
            };

            let mint_auth_bump_bytes = [mint_auth_bump];
            let mint_auth_seeds: [pinocchio::cpi::Seed; 3] = [
                pinocchio::cpi::Seed::from(MINT_AUTHORITY_SEED),
                pinocchio::cpi::Seed::from(mint_key.as_ref()),
                pinocchio::cpi::Seed::from(mint_auth_bump_bytes.as_ref()),
            ];
            let mint_auth_signer = Signer::from(&mint_auth_seeds);

            pinocchio::cpi::invoke_signed::<4>(
                &ix,
                &[
                    self.accounts.mint,
                    self.accounts.mint_authority,
                    self.accounts.mint,
                    self.accounts.mint_authority,
                ],
                &[mint_auth_signer],
            )?;
        }

        let config_bump_bytes = [config_bump];
        let config_seeds = token_config_seeds(mint_key.as_ref(), &config_bump_bytes);
        let config_signer = Signer::from(&config_seeds);

        create_pda_account(
            self.accounts.admin,
            self.accounts.config,
            TokenConfig::LEN,
            program_id,
            &[config_signer],
        )?;

        {
            let mut data = self.accounts.config.try_borrow_mut()?;
            let config = TokenConfig::from_bytes_mut_init(&mut data)?;
            config.bump = config_bump;
            config.paused = 0;
            config.mint_authority_bump = mint_auth_bump;
            config.permission_manager =
                Address::new_from_array(self.accounts.permission_manager.address().to_bytes());
            config.spl_mint = Address::new_from_array(mint_key.to_bytes());
            config.redemption_contract = Address::new_from_array([0u8; 32]); // not set initially
        }

        let event = TokenInitializedEvent::new(
            self.accounts.admin.address().clone(),
            self.accounts.mint.address().clone(),
        );
        spiko_events::emit_event(
            program_id,
            self.accounts.event_authority,
            self.accounts.self_program,
            &event.to_bytes(),
            crate::events::event_authority_pda::BUMP,
        )?;

        Ok(())
    }
}
