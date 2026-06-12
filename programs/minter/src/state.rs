use anchor_lang::prelude::*;

use crate::errors::MinterError;

#[account]
#[derive(InitSpace)]
pub struct MinterConfig {
    pub admin: Pubkey,
    pub pending_admin: Option<Pubkey>,
    pub mint_initiator: Pubkey,
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
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PendingMintAuthorityTransfer {
    pub new_authority: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
}
