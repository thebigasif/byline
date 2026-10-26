# Common tasks. `just` is the one tool to learn. CI runs the same recipes.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Everything CI checks, in the same order.
check:
    ./scripts/check.sh

# Format all Rust and TOML.
fmt:
    cargo fmt --all
    taplo fmt

# Clippy with warnings as errors on every target and feature.
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Tests with nextest, then doc tests.
test *ARGS:
    cargo nextest run --workspace --all-features {{ARGS}}
    cargo test --workspace --doc

# Docs with warnings as errors.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features

# Dependency audit: advisories, licences, bans, sources.
deny:
    cargo deny check

# Prove every lint in lints/lints.toml fires.
lints:
    ./scripts/check-lints.sh

# Prove that removing any one lint from the table makes `just lints` fail.
prove-lints:
    ./scripts/prove-lint-table.sh

# Benchmarks against the baseline.
bench:
    ./scripts/perf.sh

# The demo site through the validator, Lighthouse and axe at six widths, with
# scripts on and off. Runs on a server or in CI, never on a laptop.
demo-check:
    ./scripts/demo-check.sh

# Line coverage against the floor.
coverage:
    cargo llvm-cov nextest --workspace --all-features --summary-only

# Refresh .sqlx/, the checked query shapes CI compiles against, after changing
# a query or adding a migration. Needs sqlx-cli with the sqlite feature.
db-prepare:
    rm -f target/sqlx-prepare.db
    DATABASE_URL=sqlite:target/sqlx-prepare.db sqlx database create
    DATABASE_URL=sqlite:target/sqlx-prepare.db sqlx migrate run --source crates/khalid-db/migrations
    DATABASE_URL=sqlite:target/sqlx-prepare.db cargo sqlx prepare --workspace -- --all-targets --all-features

# Run the binary in a scratch site with a demo fixture.
dev:
    cargo run -p khalid -- serve --site fixtures/demo-site
