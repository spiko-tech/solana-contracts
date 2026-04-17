pub mod discriminators;
pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

use pinocchio::address::declare_id;
declare_id!("4yEpQ3wkwKkWq3ejgu95evdQUhkL1DNVpp4Ptg2HpetY");

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
