//! Bootstrap configuration module.
//!
//! This module provides configuration types for the bootstrap subcommand,
//! organized into focused submodules by domain.

mod core;
mod modules;
mod mount;
mod network;
mod security;
mod supervise;
mod system;

pub use self::{
    core::{BootstrapConfig, HandoffMode},
    supervise::{BootScriptConfig, BootstrapSuperviseConfig},
};
