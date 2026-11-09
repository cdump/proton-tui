//! Shared Proton API connection metadata.

pub const API_BASE: &str = "https://vpn-api.proton.me";

// Proton's Linux CLI uses the API core version, not the CLI package version.
// Keep this in sync with a supported release of:
// https://github.com/ProtonVPN/python-proton-vpn-api-core/blob/stable/versions.yml
pub const APP_VERSION: &str = "linux-vpn-cli@6.1.1+x86-64";

pub const USER_AGENT: &str = concat!("proton-tui/", env!("CARGO_PKG_VERSION"));
