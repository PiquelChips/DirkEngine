# Repository Guidelines

## Project Structure & Module Organization

DirkEngine is a Rust 2024 workspace for a Vulkan game engine. The root package (`src/`) provides the `dirkengine` binary and top-level engine wiring. Engine modules live in `crates/`, including `dirk_renderer`, `dirk_platform`, `dirk_assets`, `dirk_universe`, and `dirk_events`. Shader compilation support is in `shaders/`, with GLSL inputs under `shaders/shaders/`. Runtime and sample model assets are under `assets/models/`. Design notes live in `docs/`.

Tests are colocated with crates as `src/tests.rs` for unit coverage and `tests/*.rs` for integration coverage.

## Build, Test, and Development Commands

- `cargo build --workspace`: build every workspace crate.
- `cargo run`: run the default `dirkengine` binary. This may require a working Vulkan-capable environment and available assets.
- `cargo test --workspace`: run all unit and integration tests.
- `cargo fmt --all --check`: verify Rust formatting.
- `cargo clippy --workspace --all-targets --all-features`: run lint checks using the workspace lint policy.
- `cargo deny check`: validate dependency policy from `deny.toml` when `cargo-deny` is installed.

## Coding Style & Naming Conventions

Use standard `rustfmt` formatting with 4-space indentation. Follow Rust naming conventions: crates, modules, functions, and variables use `snake_case`; types and traits use `PascalCase`; constants use `SCREAMING_SNAKE_CASE`.

Workspace lints forbid unsafe code, warn on missing documentation, enable `clippy::pedantic`, and warn on `unwrap_used`. Prefer explicit error propagation with `Result`, `thiserror`, or `anyhow` as appropriate for the crate boundary.

## Testing Guidelines

Add unit tests in `src/tests.rs` for internal behavior and integration tests in `tests/*.rs` for public APIs. Test names should describe behavior, such as `buffered_spawns_are_applied_on_tick_and_components_are_readable`. Keep tests deterministic and avoid GPU or window-system state unless the crate requires it.

Run `cargo test --workspace` before submitting changes. For renderer or asset changes, also run the relevant crate tests directly, for example `cargo test -p dirk_renderer`.

## Commit & Pull Request Guidelines

Recent history uses short, imperative, lowercase commit subjects, with occasional conventional prefixes such as `fix:` or `refactor:`. Examples: `skip models that haven't been loaded yet when rendering` and `fix comment on BeginFrame`.

Pull requests should include a concise description, the affected crates or systems, test results, and linked issues when applicable. Include screenshots or capture notes for visible renderer, windowing, or asset-loading changes.

## Agent-Specific Instructions

Keep changes scoped to the relevant crate. Do not modify generated build output, `target/`, or saved local runtime state. Preserve assets and licenses when adding or replacing files under `assets/`.
