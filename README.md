# Spiko Solana Contracts

Tokenized money market fund shares on Solana, built with Anchor and Token-2022.

## Program IDs

### Devnet

| Program             | Address                                        |
| ------------------- | ---------------------------------------------- |
| TransferHook        | `Fzyd28cVXwzaqoU9bqU8hLpcYqhQQtSyJVaGCgcGFEjq` |
| Minter              | `Hygpx48FpJyDjW1uW8fykwb94Jmak4CaWvihRREsJyFX` |
| Redemption          | `B3ustaVazAzqwbgkxARcsL9KKKaNKT6o6FFQyo4b4EBr` |
| CustodialGatekeeper | `5Y7mJuJRdBFTXBrXG3rCUZTjRtNKhrRjCA3vKnVX2Zb6` |

### Mainnet

TBD

## Setup

Prerequisites: Rust (stable), Solana CLI 2.x, Anchor CLI 1.0.2, Node.js 20+, pnpm.

```bash
pnpm install       # install Node.js dependencies
anchor build       # build all programs (SBF binaries + IDL)
anchor test        # build, deploy to localnet, and run tests
```

## Just Commands

| Command                  | Description                              |
| ------------------------ | ---------------------------------------- |
| `just install`           | Install Node.js dependencies             |
| `just build`             | Build all programs (SBF + IDL)           |
| `just build-no-idl`      | Build without IDL generation             |
| `just check`             | Check Rust code (no .so output)          |
| `just fmt`               | Format Rust code                         |
| `just clippy`            | Run clippy                               |
| `just test`              | Run tests                                |
| `just test-skip-build`   | Run tests without rebuilding             |

## E2E Tests

The E2E test is a self-contained multi-actor scenario that runs the full lifecycle: program initialization, minting (auto-approve + pending/approve/cancel), transfers (direct + via custodial gatekeeper with daily limits), and redemptions (burn from vault). It covers both EUTBL and USTBL tokens.

### Prerequisites

- Solana CLI 2.x, Anchor CLI 1.0.2, Node.js 20+, pnpm
- A funded admin keypair (the test reads from `~/.config/solana/cli/config.yml`)

### Running on local validator

```bash
# 1. Build all programs
anchor build --ignore-keys

# 2. Start a local validator with programs loaded (Terminal 1)
solana-test-validator \
  --bpf-program Fzyd28cVXwzaqoU9bqU8hLpcYqhQQtSyJVaGCgcGFEjq target/deploy/transfer_hook.so \
  --bpf-program Hygpx48FpJyDjW1uW8fykwb94Jmak4CaWvihRREsJyFX target/deploy/minter.so \
  --bpf-program B3ustaVazAzqwbgkxARcsL9KKKaNKT6o6FFQyo4b4EBr target/deploy/redemption.so \
  --bpf-program 5Y7mJuJRdBFTXBrXG3rCUZTjRtNKhrRjCA3vKnVX2Zb6 target/deploy/custodial_gatekeeper.so \
  --reset

# 3. Configure CLI for localnet (Terminal 2)
solana config set --url http://127.0.0.1:8899

# 4. Run the e2e test
cd e2e && npx tsx e2e.ts
```

### Running on devnet

```bash
# 1. Build all programs
anchor build --ignore-keys

# 2. Configure CLI for devnet
solana config set --url https://api.devnet.solana.com

# 3. Deploy all programs (ensure funded keypair)
solana program deploy target/deploy/transfer_hook.so --program-id target/deploy/transfer_hook-keypair.json
solana program deploy target/deploy/minter.so --program-id target/deploy/minter-keypair.json
solana program deploy target/deploy/redemption.so --program-id target/deploy/redemption-keypair.json
solana program deploy target/deploy/custodial_gatekeeper.so --program-id target/deploy/custodial_gatekeeper-keypair.json

# 4. Run the e2e test
cd e2e && npx tsx e2e.ts
```

The test reads the RPC URL and admin keypair from `~/.config/solana/cli/config.yml`.
