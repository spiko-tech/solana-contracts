# Spiko Solana Contracts — Build & Codegen Recipes

# Default recipe: show available commands
default:
    @just --list

# Install Node.js dependencies
install:
    pnpm install

# Generate Codama IDL JSON files from Rust source annotations
generate-idl:
    cargo check -p permission-manager --features idl
    cargo check -p spiko-token --features idl
    cargo check -p minter --features idl
    cargo check -p redemption --features idl
    cargo check -p spiko-transfer-hook --features idl
    cargo check -p custodial-gatekeeper --features idl

# Generate TypeScript clients from IDL files
generate-clients: generate-idl
    pnpm exec tsx codegen/generate-clients.ts

# Full build: generate IDL + clients + compile programs
build: generate-clients
    cargo-build-sbf

# Check Rust code (without building .so)
check:
    cargo check --workspace

# Format Rust code
fmt:
    cargo fmt --all

# Run clippy
clippy:
    cargo clippy --workspace -- -D warnings

# Run integration tests
integration-test:
    cargo test -p integration-tests
