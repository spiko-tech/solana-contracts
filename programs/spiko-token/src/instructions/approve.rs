use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use crate::events::emit_delegate_approved;

/// Approve a spender to transfer tokens on behalf of the owner.
///
/// This is a simple passthrough to Token-2022 Approve. No permission
/// check is needed since any token holder can approve a delegate.
///
/// Accounts:
///   0. `[signer]`   Owner of the token account
///   1. `[writable]` Owner's token account
///   2. `[]`         Spender (delegate)
///   3. `[]`         Token-2022 program
///
/// Data:
///   [0..8] amount (u64, little-endian)
pub struct ApproveToken<'a> {
    pub owner: &'a AccountView,
    pub token_account: &'a AccountView,
    pub delegate: &'a AccountView,
    pub token_2022_program: &'a AccountView,
    pub amount: u64,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for ApproveToken<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [owner, token_account, delegate, token_2022_program, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !owner.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.len() < 8 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let amount = u64::from_le_bytes(data[0..8].try_into().unwrap());

        Ok(Self {
            owner,
            token_account,
            delegate,
            token_2022_program,
            amount,
        })
    }
}

impl<'a> ApproveToken<'a> {
    pub fn process(&self, _program_id: &Address) -> ProgramResult {
        // Straight CPI to Token-2022: Approve
        pinocchio_token_2022::instructions::Approve {
            source: self.token_account,
            delegate: self.delegate,
            authority: self.owner,
            amount: self.amount,
            token_program: self.token_2022_program.address(),
        }
        .invoke()?;

        pinocchio_log::log!("Approved delegate");
        emit_delegate_approved(
            &self.owner.address().to_bytes(),
            &self.token_account.address().to_bytes(),
            &self.delegate.address().to_bytes(),
            self.amount,
        );

        Ok(())
    }
}
