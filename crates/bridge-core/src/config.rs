use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::token::generate_token;

/// Runtime configuration for the bridge.
///
/// `root` is the single directory the bridge is allowed to serve. Every file
/// operation is resolved relative to (and confined within) this directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    /// The only directory the bridge may read from or write to.
    pub root: PathBuf,
    /// Bearer token required on every request.
    pub token: String,
    /// Address the HTTP server binds to. Defaults to loopback.
    pub host: IpAddr,
    /// TCP port. `0` lets the OS choose a free port.
    pub port: u16,
    /// When false, write/mkdir/delete endpoints are rejected.
    pub allow_write: bool,
    /// Maximum requests per second. `0` disables rate limiting.
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u64,
}

fn default_rate_limit() -> u64 {
    100
}

impl BridgeConfig {
    /// Build a read/write config for `root` with a freshly generated token,
    /// bound to loopback on an OS-assigned port.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            token: generate_token(40),
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 0,
            allow_write: true,
            rate_limit: default_rate_limit(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = token.into();
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_host(mut self, host: IpAddr) -> Self {
        self.host = host;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.allow_write = false;
        self
    }

    pub fn with_rate_limit(mut self, rps: u64) -> Self {
        self.rate_limit = rps;
        self
    }
}
