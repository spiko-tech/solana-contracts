use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::GatekeeperEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct GatekeeperInitializedEvent {
    pub admin: Address,
}

impl EventDiscriminator for GatekeeperInitializedEvent {
    const DISCRIMINATOR: u8 = GatekeeperEventDiscriminators::GatekeeperInitialized as u8;
}

impl EventSerialize for GatekeeperInitializedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.admin.as_ref());
        data
    }
}

impl GatekeeperInitializedEvent {
    pub const DATA_LEN: usize = 32;

    #[inline(always)]
    pub fn new(admin: Address) -> Self {
        Self { admin }
    }
}
