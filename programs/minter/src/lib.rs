pub mod discriminators;
pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

use pinocchio::address::declare_id;
declare_id!("3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2");

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
