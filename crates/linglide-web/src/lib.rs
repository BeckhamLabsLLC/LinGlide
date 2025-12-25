//! LinGlide Web - Embedded web assets
//!
//! This crate embeds the web viewer assets into the binary.

use rust_embed::Embed;

#[derive(Embed)]
#[folder = "www/"]
pub struct Assets;
