# Repository Guidelines

## Project Structure & Module Organization

`termfoto` is a Rust terminal image browser. The binary entry point is `src/main.rs`, which handles CLI parsing, terminal setup, the event loop, and grid sizing. Core state, navigation, caching, background image loading, fullscreen render scheduling, and zoom/pan logic live in `src/app.rs`. Directory scanning is in `src/scanner.rs`, language strings are in `src/lang.rs`, and UI widgets are under `src/ui/` (`browser.rs`, `preview.rs`, `search.rs`, `mod.rs`). Static docs assets live in `assets/`. The `npm/` directory is a thin distribution wrapper. GitHub Actions workflows are in `.github/workflows/`.

## Build, Test, and Development Commands

- `cargo build` compiles the default local build.
- `cargo run -- <path>` runs the app against a directory or image file.
- `cargo test` runs all unit tests.
- `cargo test <name>` runs tests matching a name filter.
- `cargo clippy -- -D warnings` matches CI lint strictness.
- `cargo fmt` formats Rust sources before review.
- `cargo build --release` builds the optimized release profile.
- `cargo build --release --no-default-features --features chafa-static` matches the release workflow binary build.

CI installs `libchafa-dev` and `libglib2.0-dev`, then runs build, test, and clippy.

## Coding Style & Naming Conventions

Use Rust 2021 style and `rustfmt` defaults. Keep modules focused on their current responsibilities; avoid moving UI, scanning, and app-state logic across boundaries without a clear reason. Prefer descriptive snake_case for functions and variables, PascalCase for types and enum variants, and SCREAMING_SNAKE_CASE for constants such as `MIN_CELL`. Keep CLI behavior lightweight and avoid adding startup indexing or recursive scanning unless explicitly required.

## Testing Guidelines

Tests are inline `#[cfg(test)]` modules in the source files they cover. Existing coverage focuses on navigation, search scoring, language selection, scanner behavior, fullscreen rendering, loader routing, and cache behavior. Add focused unit tests near changed logic, especially for keyboard behavior, search results, supported image extensions, cache limits, render quality selection, thumbnail request order, and path handling. Run `cargo test` before submitting changes.

## Commit & Pull Request Guidelines

Use English commit messages with Conventional Commit prefixes such as `fix:`, `docs:`, and `chore:`. Keep commits scoped and imperative, for example `fix: handle empty image directory`. Pull requests should include a short summary, test results, linked issues when applicable, and screenshots or terminal captures for visible UI changes. Mention release-impacting changes to `Cargo.toml`, `Cargo.lock`, `npm/package.json`, or workflow files.

## Agent-Specific Instructions

Follow repository-specific instructions before editing. In this environment, shell commands should be prefixed with `rtk`.
