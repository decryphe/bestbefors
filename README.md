# Bestbefors

Bestbefors is a Rust/Loco web application for tracking inventory items that expire or require recurring inspections. It lets you define item kinds, intervals, expiries, and checklists, manage actual inventory records, and log executed checklist runs with per-step outcomes and notes. Every page is server-rendered via Tera templates, with minimal vanilla JavaScript for form submissions.

![Bestbefors Logo](/assets/static/logo.svg)

The source code is provided under MIT or Apache 2.0 licenses. Use as you please.

## Host Requirements

- Rust 1.75+ with `cargo`
- SQLite 3 (default development database stored in `bestbefors_development.sqlite`)
- OpenSSL/LibreSSL headers (on Debian/Ubuntu install `libssl-dev`; on macOS `brew install openssl@3`)

## Getting Started

```bash
# Install dependencies and compile
cargo build

# Seed the database with some useful starting data, e.g. basic expiries or intervals
cargo loco db seed --reset

# Run migrations + start dev server (auto-migrates SQLite)
cargo loco start
```

Open `http://localhost:5150` to access the dashboard. Configuration files live under `config/*.yaml`; the development profile points at SQLite, while production may be switched to Postgres or another SeaORM-supported backend.

## Key Features

- Inventory item CRUD with item kinds, intervals, expiries, serial numbers
- Checklist management plus executed check history per item
- Per-step results and notes for every executed checklist
- Authentication endpoints (JWT, password, magic link) with login/register/logout pages
- Bootstrap 5 UI backed by Tera templates; assets in `assets/views/` and `assets/static/`

## Common Commands

- `cargo fmt` / `cargo clippy -- -D warnings` – formatting and linting
- `cargo test` – unit and integration tests
- `cargo loco start` – run application with auto-migrate

## Project Layout

- `src/controllers/` – request handlers (auth, inventory, checklists, users, etc.)
- `assets/views/` – Tera templates (home, inventory, auth)
- `migration/` – SeaORM migrations
- `tests/requests/` – API-level smoke tests
- `AGENTS.md` – contributor guidelines and workflow expectations

For additional framework details see the [Loco documentation](https://loco.rs/docs/). Contributions and bug reports are welcome.***
