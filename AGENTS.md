# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs` is the entry point and wires the TUI runtime.
- `src/app/` holds application state, navigation, search, and connection logic.
- `src/ui/` renders views and widgets for the TUI.
- `src/` top-level modules (`api.rs`, `auth.rs`, `login.rs`, `wireguard.rs`, etc.) handle Proton API, auth, and WireGuard integration.
- `assets/` stores the screenshot used in the README; `target/` is build output.

## Build, Test, and Development Commands
- `cargo build`: debug build.
- `cargo build --release`: optimized binary.
- `cargo run --release`: build and run the TUI locally.
- `cargo install --path .`: install the binary from source.
- `cargo test`: run Rust's built-in test harness (none currently defined).

## Coding Style & Naming Conventions
Use Rust 2021 defaults and `rustfmt` conventions (4-space indentation, trailing commas). Name modules and functions in `snake_case`, types in `CamelCase`, and constants in `SCREAMING_SNAKE_CASE`. Keep UI code in `src/ui/` and state/behavior in `src/app/` to avoid mixing rendering with business logic.

## Testing Guidelines
There are no unit or integration tests yet. When adding tests, prefer `#[cfg(test)] mod tests` near the code under test or `tests/` for integration tests; name test functions `test_*`. Run `cargo test` locally before opening a PR. No coverage targets are defined.

## Commit & Pull Request Guidelines
The current Git history contains only an `initial commit`, so no established message convention exists. Use concise, imperative subjects (e.g., `Add split-view search`) and include a brief body when behavior changes. PRs should include a summary, the commands you ran (e.g., `cargo test`), and link any related issues. For UI changes, update `assets/screenshot.png` if the interface is visibly different.

## Security & Configuration Notes
This project targets Linux and relies on `wireguard-tools` plus `sudo wg-quick`. Avoid committing credentials or generated configs; runtime configs are written to `/tmp/proton-tui0.conf` with restricted permissions.
