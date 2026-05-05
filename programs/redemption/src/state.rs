use anchor_lang::prelude::*;

pub use permission_manager::state::{has_role, is_admin, PermissionConfig, UserPermissions};

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::utils::verify_operation_id;

#[account]
#[derive(InitSpace)]
pub struct RedemptionConfig {
    pub deadline_delay: i64,
    pub permission_manager: Pubkey,
}

#[account]
#[derive(InitSpace)]
pub struct VaultAuthority {
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct RedemptionOperation {
    pub status: u8,
    pub deadline: i64,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

impl RedemptionOperation {
    pub fn is_pending(&self) -> bool {
        self.status == STATUS_PENDING
    }

    pub fn validate_for_execution(
        &self,
        operation_id: &[u8; 32],
        mint: &Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        verify_operation_id(&self.user, mint, amount, salt, operation_id)?;
        require!(self.is_pending(), RedemptionError::NotPending);
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp <= self.deadline,
            RedemptionError::DeadlinePassed
        );
        Ok(())
    }

    pub fn validate_for_cancellation(
        &self,
        operation_id: &[u8; 32],
        mint: &Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        verify_operation_id(&self.user, mint, amount, salt, operation_id)?;
        require!(self.is_pending(), RedemptionError::NotPending);
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp > self.deadline,
            RedemptionError::DeadlineNotPassed
        );
        Ok(())
    }
}
