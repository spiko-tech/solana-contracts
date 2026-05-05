# Spiko Solana Contracts

Tokenized money market fund shares on Solana, built with Anchor and Token-2022.

## Program IDs

### Devnet

| Program             | Address                                        |
| ------------------- | ---------------------------------------------- |
| PermissionManager   | `G3KXsXdrTz85MjA7avs89fTHmQa4SkybRdRRNBYq5XZE` |
| SpikoToken          | `6amQsxSBnx64VVVgEueDFHPGkZ62VoUSQvhyLjKYbejZ` |
| SpikoTransferHook   | `21Qu5pfKsxFpmDpwrXq1ZjVxCDW5kA9jrtBuMeQCNh86` |
| Minter              | `13jYMgAoRQHSKVT6LakgRKFiyygFTN7LYsKym9Lv84MQ` |
| Redemption          | `F6P3cmm4xDxxZCF6vj3K9pbY2LFjVrYpEft6x6CXJxmu` |
| CustodialGatekeeper | `7raQ9TfCJkFWFDg2X2GsuPh3rso5n6jRS2WGa7enhtfg` |

### Mainnet

TBD

## Setup

Prerequisites: Rust (stable), Solana CLI 2.x, Anchor CLI 0.31.1, Node.js 20+, pnpm.

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

The E2E test is a self-contained multi-actor scenario that deploys all programs and runs through minting, transfers, custodial gatekeeper withdrawals, and redemptions.

### Prerequisites

- All 6 programs deployed (devnet or local validator)
- Solana CLI configured (`solana config set --url <rpc_url>`) with a funded admin keypair
- Node.js 20+, pnpm

### Running

```bash
# 1. Build programs (BPF binaries)
cargo build-sbf --tools-version v1.48

# 2. Deploy all programs
solana program deploy target/deploy/permission_manager.so --program-id target/deploy/permission_manager-keypair.json
solana program deploy target/deploy/spiko_transfer_hook.so --program-id target/deploy/spiko_transfer_hook-keypair.json
solana program deploy target/deploy/spiko_token.so --program-id target/deploy/spiko_token-keypair.json
solana program deploy target/deploy/minter.so --program-id target/deploy/minter-keypair.json
solana program deploy target/deploy/redemption.so --program-id target/deploy/redemption-keypair.json
solana program deploy target/deploy/custodial_gatekeeper.so --program-id target/deploy/custodial_gatekeeper-keypair.json

# 3. Run the e2e test
npx tsx e2e/e2e.ts
```

The test reads the RPC URL and admin keypair from `~/.config/solana/cli/config.yml`.
