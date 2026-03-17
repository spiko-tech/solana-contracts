use pinocchio::{account::AccountView, address::Address, error::ProgramError, ProgramResult};

use crate::helpers::{require_admin, verify_pda};
use crate::state::{PermissionConfig, PERMISSION_CONFIG_SEED};

/// Transfer ownership — sets a pending admin who must accept.
///
/// Accounts:
///   0. `[signer]`   Current admin
///   1. `[writable]` PermissionConfig PDA
///
/// Data:
///   [0..32] new_admin address (32 bytes)
pub struct TransferOwnership<'a> {
    pub admin: &'a AccountView,
    pub config: &'a AccountView,
    pub new_admin: Address,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for TransferOwnership<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [admin, config, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if data.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mut addr_bytes = [0u8; 32];
        addr_bytes.copy_from_slice(&data[0..32]);
        let new_admin = Address::new_from_array(addr_bytes);

        Ok(Self {
            admin,
            config,
            new_admin,
        })
    }
}

impl<'a> TransferOwnership<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        // 1. Verify PermissionConfig PDA
        verify_pda(self.config, &[PERMISSION_CONFIG_SEED], program_id)?;

        // 2. Check caller is admin
        require_admin(self.admin, self.config, program_id)?;

        // 3. Set pending_admin
        {
            let mut data = self.config.try_borrow_mut()?;
            let config = PermissionConfig::from_bytes_mut(&mut data)?;
            config.pending_admin = Address::new_from_array(self.new_admin.to_bytes());
        }

        pinocchio_log::log!("OwnershipTransferStarted");

        Ok(())
    }
}
