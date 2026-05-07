# Spiko Solana Contracts

Tokenized money market fund shares on Solana, built with Anchor and Token-2022.

## Program IDs

### Devnet

| Program             | Address                                        |
| ------------------- | ---------------------------------------------- |
| PermissionManager   | `7Kn4rpdRjcPZSPgR4h1VU97DviDdZsBEd284BfSpUbMD` |
| SpikoToken          | `F8sDrPvNHJCaB8EBKj5fJc2jt4FpxfAVW7Y2pqsHqcEN` |
| SpikoTransferHook   | `7DXckwPHM1ktduwLXWxsn87hWrmyUVKDNNst5ycAj8VU` |
| Minter              | `9SwnGKZtV54CRsFd8eocmBNH5WzxCiG7bBb1B3romQSj` |
| Redemption          | `2MJeRdtRSUu9UJkuuVzWHKc8rgQpTfYEuKevpoM1Uv1D` |
| CustodialGatekeeper | `9z86yHHZEojd2HoGBviCKf7kWbbZJqWzRgQQm3bKCBh5` |

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

The E2E test is a self-contained multi-actor scenario that deploys all programs and runs through role grants, minting, transfers, custodial gatekeeper withdrawals, and redemptions. It verifies final balances and decodes all CPI events.

### Prerequisites

- Solana CLI 2.x, Anchor CLI 1.0.2, Node.js 20+, pnpm
- A funded admin keypair (the test reads from `~/.config/solana/cli/config.yml`)

### Running on local validator

```bash
# 1. Build all programs
anchor build

# 2. Start a local validator (reset state)
solana-test-validator --reset -q &

# 3. Deploy all programs
for f in target/deploy/*.so; do
  [[ "$f" == *spl_token* ]] && continue
  solana program deploy "$f"
done

# 4. Run the e2e test
cd e2e && npx tsx e2e.ts
```

### Running on devnet

```bash
# 1. Build all programs
anchor build

# 2. Configure CLI for devnet
solana config set --url https://api.devnet.solana.com

# 3. Deploy all programs (ensure funded keypair)
solana program deploy target/deploy/permission_manager.so
solana program deploy target/deploy/spiko_transfer_hook.so
solana program deploy target/deploy/spiko_token.so
solana program deploy target/deploy/minter.so
solana program deploy target/deploy/redemption.so
solana program deploy target/deploy/custodial_gatekeeper.so

# 4. Run the e2e test
cd e2e && npx tsx e2e.ts
```

### Expected output

All 13 steps should pass with all events decoded:

```
Steps:          13
Events decoded: 13/13
All expected events were found!

--- Final Balances ---
  User1: 4 shares
  User2: 5 shares
  User3: 1 shares
  Vault: 0 shares
  CG Vault: 0 shares
```

The test reads the RPC URL and admin keypair from `~/.config/solana/cli/config.yml`.
