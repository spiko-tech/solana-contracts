use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::RedemptionEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct RedemptionInitializedEvent {
    pub admin: Address,
}

impl EventDiscriminator for RedemptionInitializedEvent {
    const DISCRIMINATOR: u8 = RedemptionEventDiscriminators::RedemptionInitialized as u8;
}

impl EventSerialize for RedemptionInitializedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.admin.as_ref());
        data
    }
}

impl RedemptionInitializedEvent {
    pub const DATA_LEN: usize = 32;

    #[inline(always)]
    pub fn new(admin: Address) -> Self {
        Self { admin }
    }
}
