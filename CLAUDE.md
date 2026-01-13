# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

```bash
# Build and run
cargo run --release

# Install globally
cargo install --path .

# Check code (fast compilation check)
cargo check

# Format code (required by CI)
cargo fmt

# Lint code (CI fails on warnings)
cargo clippy -- -D warnings
```

## Architecture Overview

Proton TUI is a Rust terminal application for connecting to ProtonVPN via WireGuard. It uses async/await throughout with Tokio runtime.

### Core Modules

**Authentication (`auth.rs`)** - Implements SRP-6a (Secure Remote Password) protocol for ProtonVPN authentication. Handles the multi-step flow: fetch SRP parameters, verify PGP signature on modulus, compute client proofs, submit authentication, and handle 2FA if required.

**API Client (`api.rs`)** - `ProtonClient` wraps reqwest for ProtonVPN API calls. Main endpoints: `/vpn/logicals` (server list), `/vpn/v1/certificate` (WireGuard cert registration).

**Token Management (`tokens.rs`)** - Persists auth tokens to `~/.config/proton-tui/tokens.json` with 0600 permissions.

**WireGuard (`wireguard.rs`)** - Generates WireGuard config files. Uses Ed25519 key generation with X25519 derivation for the WireGuard key exchange.

### Application State (`app/`)

- `mod.rs` - Main `App` struct holding server list, UI state, connection status
- `state.rs` - Enums: `InputMode`, `SplitFocus`, `DisplayItem`, `ConnectionStatus`
- `connection.rs` - WireGuard management via `sudo wg-quick up/down`, traffic stats from `/proc/net/dev`
- `navigation.rs` - Tree view navigation (expand/collapse countries, selection)
- `search.rs` - Incremental server filtering with Emacs-style keybindings
- `split_view.rs` - Dual-pane view logic (countries left, servers right)

### UI Layer (`ui/mod.rs`)

Ratatui-based rendering with two view modes:
- **Tree View**: Hierarchical country → server list with expandable sections
- **Split View**: Dual-pane with country list (35%) and server list (65%)

### Event Loop (`main.rs`)

1. Initialize crossterm terminal
2. Load tokens or show login form
3. Fetch server list
4. Poll keyboard events (100ms), update state, render with ratatui
5. TUI suspends/resumes around sudo prompts for WireGuard operations

### Key Technical Details

- Interface name: `proton-tui0`
- Config path: `/tmp/proton-tui0.conf` (mode 600)
- Linux only (requires `wg-quick`)
- Requires sudo for VPN operations
- DNS is not modified (relies on system DNS)

## Dependencies

Key crates: `ratatui` (TUI), `crossterm` (terminal), `reqwest` (HTTP), `tokio` (async), `ed25519-dalek`/`sha2`/`bcrypt`/`num-bigint` (SRP crypto).
