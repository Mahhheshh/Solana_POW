#![no_std]
use pinocchio::{entrypoint, nostd_panic_handler};

pub mod processor;
pub use processor::process_instruction;

pub mod instructions;
pub mod state;

pinocchio_pubkey::declare_id!("22222222222222222222222222222222222222222222");

entrypoint!(process_instruction);
nostd_panic_handler!();
