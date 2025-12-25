//! LinGlide Core - Shared types and protocol definitions
//!
//! This crate provides the foundational types used across all LinGlide components.

pub mod config;
pub mod error;
pub mod frame;
pub mod protocol;

pub use config::{Config, DisplayPosition};
pub use error::{Error, Result};
pub use frame::Frame;
pub use protocol::InputEvent;
