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
pnpm generate-clients  # regenerate TypeScript clients (Codama)
anchor test        # build, deploy to localnet, and run tests
```

## Just Commands

| Command                | Description                     |
| ---------------------- | ------------------------------- |
| `just install`         | Install Node.js dependencies    |
| `just build`           | Build all programs (SBF + IDL)  |
| `just build-no-idl`    | Build without IDL generation    |
| `just check`           | Check Rust code (no .so output) |
| `just fmt`             | Format Rust code                |
| `just clippy`          | Run clippy                      |
| `just test`            | Run tests                       |
| `just test-skip-build` | Run tests without rebuilding    |

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
cd e2e && pnpm tsx e2e.ts
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
cd e2e && pnpm tsx e2e.ts
```

The test reads the RPC URL and admin keypair from `~/.config/solana/cli/config.yml`.

## Deployment Scripts

Two scripts handle program setup and token creation independently.

### 1. Setup Programs

Initializes all 4 programs and gates vault PDAs. Optionally builds and deploys the `.so` binaries. Idempotent — safe to re-run.

```bash
pnpm tsx scripts/setup-programs.ts \
  --cluster <localnet|devnet|mainnet-beta> \
  --keypair <path-to-admin-keypair.json> \
  --deploy <true|false> \
  --whitelist-authority <pubkey> \
  --mint-initiator <pubkey> \
  --redemption-authority <pubkey> \
  --gatekeeper-initiator <pubkey>
```

| Flag                     | Description                                                           |
| ------------------------ | --------------------------------------------------------------------- |
| `--cluster`              | Target network (`localnet`, `devnet`, `mainnet-beta`)                 |
| `--keypair`              | Path to the admin keypair JSON file                                   |
| `--deploy`               | `true` = run `anchor build` + `solana program deploy`; `false` = skip |
| `--whitelist-authority`  | Pubkey for Transfer Hook whitelist authority                          |
| `--mint-initiator`       | Pubkey for Minter initiator role                                      |
| `--redemption-authority` | Pubkey for Redemption authority role                                  |
| `--gatekeeper-initiator` | Pubkey for Custodial Gatekeeper initiator role                        |

**Output:** `deployments/programs-<cluster>.json`

### 2. Create Token

Creates a new Token-2022 mint with all Spiko extensions (TransferHook, PermanentDelegate, MetadataPointer), registers it, sets daily limits, and creates vault ATAs. Requires `setup-programs` to have been run first.

```bash
pnpm tsx scripts/create-token.ts \
  --cluster <localnet|devnet|mainnet-beta> \
  --keypair <path-to-admin-keypair.json> \
  --symbol EUTBL \
  --name "EU T-Bill Token" \
  --uri "https://spiko.finance/eutbl" \
  --decimals 6 \
  --minter-daily-limit 1000000 \
  --cg-daily-limit 500000
```

| Flag                   | Description                                           |
| ---------------------- | ----------------------------------------------------- |
| `--cluster`            | Target network                                        |
| `--keypair`            | Path to the admin keypair JSON file                   |
| `--symbol`             | Token symbol (e.g. `EUTBL`)                           |
| `--name`               | Token display name                                    |
| `--uri`                | Metadata URI                                          |
| `--decimals`           | Token decimals (e.g. `6`)                             |
| `--minter-daily-limit` | Minter daily limit in whole token units               |
| `--cg-daily-limit`     | Custodial Gatekeeper daily limit in whole token units |

**Output:** `deployments/<symbol>-<cluster>.json`

### Example: Full localnet setup

```bash
# Terminal 1: start validator
solana-test-validator \
  --bpf-program Fzyd28cVXwzaqoU9bqU8hLpcYqhQQtSyJVaGCgcGFEjq target/deploy/transfer_hook.so \
  --bpf-program Hygpx48FpJyDjW1uW8fykwb94Jmak4CaWvihRREsJyFX target/deploy/minter.so \
  --bpf-program B3ustaVazAzqwbgkxARcsL9KKKaNKT6o6FFQyo4b4EBr target/deploy/redemption.so \
  --bpf-program 5Y7mJuJRdBFTXBrXG3rCUZTjRtNKhrRjCA3vKnVX2Zb6 target/deploy/custodial_gatekeeper.so \
  --reset

# Terminal 2: setup + create tokens
ADMIN=$(solana address)

pnpm tsx scripts/setup-programs.ts \
  --cluster localnet --keypair ~/.config/solana/id.json --deploy false \
  --whitelist-authority $ADMIN --mint-initiator $ADMIN \
  --redemption-authority $ADMIN --gatekeeper-initiator $ADMIN

pnpm tsx scripts/create-token.ts \
  --cluster localnet --keypair ~/.config/solana/id.json \
  --symbol EUTBL --name "EU T-Bill Token" --uri "https://spiko.finance/eutbl" \
  --decimals 6 --minter-daily-limit 1000000 --cg-daily-limit 500000

pnpm tsx scripts/create-token.ts \
  --cluster localnet --keypair ~/.config/solana/id.json \
  --symbol USTBL --name "US T-Bill Token" --uri "https://spiko.finance/ustbl" \
  --decimals 6 --minter-daily-limit 1000000 --cg-daily-limit 500000
```
