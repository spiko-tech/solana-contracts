use anchor_lang::prelude::*;

use crate::errors::GatekeeperError;

#[account]
#[derive(InitSpace)]
pub struct GatekeeperConfig {
    pub admin: Pubkey,
    pub gatekeeper_initiator: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VaultAuthority {
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct WithdrawalDailyLimit {
    pub limit: u64,
    pub used_amount: u64,
    pub last_day: i64,
    pub bump: u8,
}

impl WithdrawalDailyLimit {
    pub fn reset_if_new_day(&mut self, current_day: i64) {
        if current_day > self.last_day {
            self.used_amount = 0;
            self.last_day = current_day;
        }
    }

    pub fn is_within_limit(&self, amount: u64) -> Result<bool> {
        let new_total = self
            .used_amount
            .checked_add(amount)
            .ok_or(GatekeeperError::ArithmeticOverflow)?;
        Ok(new_total <= self.limit)
    }

    pub fn record_usage(&mut self, amount: u64) -> Result<()> {
        self.used_amount = self
            .used_amount
            .checked_add(amount)
            .ok_or(GatekeeperError::ArithmeticOverflow)?;
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct GatekeepOperation {
    pub status: u8,
    pub sender: Pubkey,
    pub destination: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub bump: u8,
}
