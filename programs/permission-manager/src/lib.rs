pub mod discriminators;
pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

use pinocchio::address::declare_id;
declare_id!("2Qhjh6NXiyQEPBP9tVCkzNtLWERHbggUjbbwje1Mpqsc");

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
