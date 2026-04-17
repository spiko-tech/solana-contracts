use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::MinterEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct MinterInitializedEvent {
    pub admin: Address,
    pub max_delay: i64,
}

impl EventDiscriminator for MinterInitializedEvent {
    const DISCRIMINATOR: u8 = MinterEventDiscriminators::MinterInitialized as u8;
}

impl EventSerialize for MinterInitializedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.admin.as_ref());
        data.extend_from_slice(&self.max_delay.to_le_bytes());
        data
    }
}

impl MinterInitializedEvent {
    pub const DATA_LEN: usize = 32 + 8;

    #[inline(always)]
    pub fn new(admin: Address, max_delay: i64) -> Self {
        Self { admin, max_delay }
    }
}
