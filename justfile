# Spiko Solana Contracts — Anchor Build Recipes

# Default recipe: show available commands
default:
    @just --list

# Install Node.js dependencies
install:
    pnpm install

# Build all programs (SBF binaries + IDL)
build:
    anchor build

# Build without IDL generation
build-no-idl:
    anchor build --no-idl

# Generate TypeScript clients from Anchor IDLs
generate-clients:
    pnpm run generate-clients

# Check Rust code (without building .so)
check:
    cargo check --workspace

# Format Rust code
fmt:
    cargo fmt --all

# Run clippy
clippy:
    cargo clippy --workspace -- -D warnings

# Run Anchor integration tests (TypeScript)
test:
    anchor test

# Run tests without rebuilding
test-skip-build:
    anchor test --skip-build

# Run Rust BPF unit tests for a specific program
test-sbf program:
    cargo test-sbf -p {{program}} --tools-version v1.48

# Run all Rust BPF unit tests
test-sbf-all:
    cargo test-sbf --tools-version v1.48
