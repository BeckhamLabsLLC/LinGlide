//! LinGlide Input - uinput-based input injection
//!
//! This crate provides virtual input device creation and event injection.

pub mod mouse;
pub mod stylus;
pub mod touch;
pub mod uinput;

pub use mouse::VirtualMouse;
pub use stylus::VirtualStylus;
pub use touch::VirtualTouchscreen;
pub use uinput::VirtualDevice;
