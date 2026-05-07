use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;

pub use permission_manager::state::{has_role, is_admin, PermissionConfig, UserPermissions};

#[account]
#[derive(InitSpace)]
pub struct MinterConfig {
    pub max_delay: i64,
    pub permission_manager: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct MintDailyLimit {
    pub limit: u64,
    pub used_amount: u64,
    pub last_day: i64,
    pub bump: u8,
}

impl MintDailyLimit {
    pub fn reset_if_new_day(&mut self, current_day: i64) {
        if current_day != self.last_day {
            self.used_amount = 0;
            self.last_day = current_day;
        }
    }

    pub fn is_within_limit(&self, amount: u64) -> Result<bool> {
        let new_total = self
            .used_amount
            .checked_add(amount)
            .ok_or(MinterError::ArithmeticOverflow)?;
        Ok(new_total <= self.limit)
    }

    pub fn record_usage(&mut self, amount: u64) -> Result<()> {
        self.used_amount = self
            .used_amount
            .checked_add(amount)
            .ok_or(MinterError::ArithmeticOverflow)?;
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct MintOperation {
    pub status: u8,
    pub deadline: i64,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub bump: u8,
}

impl MintOperation {
    pub fn is_pending(&self) -> bool {
        self.status == STATUS_PENDING
    }

    pub fn validate_for_approval(
        &self,
        operation_id: &[u8; 32],
        mint: &Pubkey,
        recipient: &Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        crate::utils::verify_operation_id(recipient, mint, amount, salt, operation_id)?;
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp <= self.deadline,
            MinterError::DeadlinePassed
        );
        Ok(())
    }

    pub fn validate_for_cancellation(
        &self,
        operation_id: &[u8; 32],
        mint: &Pubkey,
        recipient: &Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        crate::utils::verify_operation_id(recipient, mint, amount, salt, operation_id)?;
        let clock = Clock::get()?;
        require!(
            clock.unix_timestamp > self.deadline,
            MinterError::DeadlineNotPassed
        );
        Ok(())
    }
}
