# Repository Guidelines

## Project Overview
Bestbefors is a Loco-based Rust web app for tracking expiration dates, whether they are fixed intervals (recurring inspections) or one-off deadlines for arbitrary items. Typical uses range from monitoring pantry goods to logging chemical storage shelf lives or other equipment with mandatory refresh cycles. The UI favors server-side rendering through the Tera template engine; avoid JavaScript entirely unless it is absolutely required, and stick to vanilla scripts when no server-side alternative exists.

## Project Structure & Module Organization
`src/` holds all Rust modules (controllers, services, domain). Keep each feature in its own module and re-export via `mod.rs` for discoverability. Static files or templates sit in `assets/`, environment overlays live in `config/*.yaml` (start with `config/development.yaml`), and database changes belong to `migration/`. `examples/` contains runnable snippets, `tests/` hosts integration specs, and `initial_db.bash` seeds SQLite. Ignore `target/`; it only contains build artifacts.

## Build, Test & Development Commands
- `cargo loco start` — compiles and runs the dev server with the default SQLite database.
- `cargo build --release` — produces the optimized binary used for staging/prod.
- `cargo test` — runs unit tests (`src/*`) and integration tests (`tests/*`).
- `cargo fmt && cargo clippy -- -D warnings` — enforces formatting and lints; required before opening a PR.

## Coding Style & Naming Conventions
Use 4-space indentation and snake_case file names (`src/services/user_service.rs`). Public types, enums, and traits are UpperCamelCase; private helpers remain snake_case. Favor `anyhow::Result<T>` for plumbing code and custom error enums for domain-level validation. Keep controller methods thin by delegating to service modules, and return HTTP-safe errors with meaningful messages. Document non-obvious logic with short comments rather than long narratives.

## Testing Guidelines
Colocate fast unit tests in the same file under `#[cfg(test)] mod tests`. Use `tests/*.rs` for end-to-end flows that boot the Loco app or hit the database. Every feature should add a happy-path integration test plus unit coverage for edge conditions (validation, error mapping, timeouts). Name tests after behavior (`returns_401_for_expired_token`). When touching migrations, assert schema expectations in a fresh SQLite database to avoid regressions.

## Commit & Pull Request Guidelines
Commits should have a concise, imperative subject (`Add shelf-life reminder job`) and optional wrapped body describing context or follow-ups. Keep unrelated work out of the same commit; it simplifies review and `git bisect`. Pull requests need: summary, testing log (commands + outcomes), linked issue, and screenshots or cURL transcripts for API/UI changes. Draft PRs are fine for feedback but must already pass fmt/clippy/test.

## Security & Configuration Tips
Do not commit real secrets; load them via environment variables referenced in the YAML configs. Rotate JWT signing keys in `config/development.yaml` after demos and never reuse them in production. When adding new configuration knobs, document default values and required env vars in the PR so deploy scripts can be updated promptly.
