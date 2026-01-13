# Proton TUI - Project Context

## Project Overview
**Proton TUI** is an unofficial, terminal-based user interface for ProtonVPN, written in Rust. It allows users to browse servers, authenticate securely, and manage WireGuard connections directly from the CLI.

**Disclaimer:** This is an unofficial tool and is **not** affiliated with Proton AG.

## Key Technologies
*   **Language:** Rust
*   **TUI Framework:** `ratatui` (formerly `tui-rs`)
*   **Async Runtime:** `tokio`
*   **HTTP Client:** `reqwest`
*   **System Integration:** `wireguard-tools` (via `sudo wg-quick`)
*   **Authentication:** Custom SRP (Secure Remote Password) implementation (`num-bigint`, `sha2`, `bcrypt`)

## Architecture

### Directory Structure
*   **`src/main.rs`**: Application entry point. Handles the initial authentication loop, server fetching, and the main event loop.
*   **`src/app/`**: Core application logic.
    *   `mod.rs`: Defines the `App` struct and central state management.
    *   `state.rs`: Enums for `InputMode`, `SplitFocus`, etc.
    *   `connection.rs`, `navigation.rs`, `search.rs`: Feature-specific logic.
*   **`src/ui/`**: Rendering logic. Translates application state into terminal widgets.
*   **`src/api.rs`**: Interacts with the ProtonVPN HTTP API.
*   **`src/auth.rs`**: Implements the SRP authentication protocol.
*   **`src/wireguard.rs`**: Wrapper around system calls to `wg-quick` for managing network interfaces.
*   **`src/tokens.rs`**: Manages secure storage/retrieval of session tokens.

### Key Workflows
1.  **Startup & Auth:** The app checks for stored tokens. If missing or invalid, it enters a TUI-based login flow (SRP).
2.  **Server Fetch:** Fetches the list of logical servers from Proton API.
3.  **Event Loop:** Uses `crossterm` to poll for keyboard events. Updates state in `App` and redraws the UI.
4.  **Connection:**
    *   Generates a temporary WireGuard config (`/tmp/proton-tui0.conf`).
    *   Executes `sudo wg-quick up ...`.
    *   Monitors `/proc/net/dev` for traffic stats.

## Build & Run

### Prerequisites
*   Rust Toolchain
*   `wireguard-tools` (must be installed on the system)
*   Root privileges (required for `wg-quick` execution)

### Commands
*   **Run (Dev):** `cargo run`
*   **Build (Release):** `cargo build --release`
*   **Install:** `cargo install --path .`

## Development Conventions
*   **Async/Await:** Heavy use of `tokio` for non-blocking I/O (network requests, key polling).
*   **Error Handling:** Uses `anyhow::Result` for simplified error propagation.
*   **State Management:** The `App` struct is the single source of truth for the UI. Logic is split into trait extensions or helper modules within `src/app/`.
*   **UI Refresh:** The UI is reactive; `app.should_redraw` triggers a render pass.

## Notes
*   **Security:** The app handles credentials. Ensure strict review of any changes to `auth.rs` or `tokens.rs`.
*   **Platform:** Currently Linux-only due to `wireguard-tools` and `/proc` filesystem usage.
