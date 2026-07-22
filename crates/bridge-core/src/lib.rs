//! Platform-independent core for **Devin Desktop**.
//!
//! This crate implements the security-sensitive parts of the bridge so they
//! can be unit-tested in isolation, independent of the Tauri shell:
//!
//! - [`config::BridgeConfig`] — the allowed root, access token and bind addr.
//! - [`fs::ScopedFs`] — file operations confined to a single root directory.
//! - [`server`] — an [`axum`] HTTP API guarded by a bearer token.
//!
//! The Tauri application (`src-tauri`) embeds this crate, manages the lifecycle
//! of the server and renders the macOS control-panel UI.

pub mod config;
pub mod error;
pub mod fs;
pub mod server;
pub mod token;

pub use config::BridgeConfig;
pub use error::{BridgeError, Result};
pub use fs::{Entry, ScopedFs, SearchHit};
pub use server::{router, serve, serve_with_shutdown, AppState};
pub use token::generate_token;
