# Spiko Token System — Solana Contracts

Tokenized money market fund shares (EUTBL, USTBL) on Solana, built with Pinocchio (zero-copy, no Anchor) and Token-2022 extensions.

Four programs enforce role-based access control, whitelist-gated transfers, controlled minting with daily limits, and admin-forced burns via PermanentDelegate. See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full system design.

## Overview

| Program               | Description                                                                                                                                          |
| --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| **PermissionManager** | Singleton authorization hub. All permissioned operations across the system are gated through it.                                                     |
| **SpikoToken**        | Singleton token program. Manages one Token-2022 mint per fund (EUTBL, USTBL), each with TransferHook (whitelist) and PermanentDelegate (admin burn). |
| **Minter**            | Singleton mint gateway. Enforces per-token daily limits and a two-phase approval flow for large mints.                                               |
| **Redemption**        | Singleton off-ramp gateway. Users deposit tokens to request redemption; operator confirms after off-chain settlement.                                |

## Program IDs (Devnet)

| Program           | Address                                        |
| ----------------- | ---------------------------------------------- |
| PermissionManager | `BTZTjmY3i1ZPFkUvZAwD3WzwQFxxLXeaCYYBNjHKuRoz` |
| SpikoToken        | `2LKr4wYMkx75hCbrmRCR2iESCWmeDViuSDLxZaZnC4aP` |
| Minter            | `6jbcB2eNfm1qLXRFd9jJes9yYEUWYafJDjfZ1dobSQ9z` |
| Redemption        | `GZEFPC74n1ifKrsH9vh67qntZ8bqpzpdDrBasGVCUPPo` |

## Prerequisites

- **Rust** — latest stable (`rustup update stable`)
- **Solana CLI 2.x** — install via [Solana docs](https://docs.solana.com/cli/install)
- **`cargo-build-sbf`** — included with Solana CLI (`solana-install update`)
- **Node.js 20+** and npm

## Build

Build all four programs:

```bash
cargo build-sbf
```

Build a single program:

```bash
cargo build-sbf -- -p permission-manager
cargo build-sbf -- -p spiko-token
cargo build-sbf -- -p minter
cargo build-sbf -- -p redemption
```

Outputs land in `target/deploy/*.so`.

## Test

Unit tests per program:

```bash
cargo test-sbf -p permission-manager   # 25 tests
cargo test-sbf -p spiko-token          # 12 tests (11 pass, 1 known Mollusk CPI limitation)
cargo test-sbf -p minter               # 11 tests
cargo test-sbf -p redemption           # 14 tests
```

Integration tests (end-to-end flows across all 4 programs):

```bash
cargo test-sbf -p integration-tests    # 14 tests
```

E2E tests:

```bash
cd scripts && pnpm install && pnpm e2e-test
```

Total: **76/77 pass**. The single skip is a Mollusk test-harness limitation with cross-program invocations — the actual on-chain behavior is verified on devnet.

## Deploy to Devnet

### 1. Configure Solana CLI

```bash
solana config set --url devnet
solana config set --keypair ~/.config/solana/id.json
solana airdrop 5   # fund the deployer if needed
```

### 2. Deploy the four programs

Each program's keypair in `target/deploy/` determines its on-chain address. Deploy order does not matter.

```bash
solana program deploy target/deploy/permission_manager.so
solana program deploy target/deploy/spiko_token.so
solana program deploy target/deploy/minter.so
solana program deploy target/deploy/redemption.so
```

### 3. Install script dependencies

```bash
cd scripts && npm install
```

### 4. Run the initialization script

```bash
npx tsx deploy.ts
```

This executes 8 phases in sequence (idempotent — safe to re-run):

1. **Initialize PermissionManager** — creates PermissionConfig PDA + admin UserPermissions PDA
2. **Initialize SpikoToken x2** — creates Token-2022 mints (TransferHook + PermanentDelegate) and TokenConfig PDAs for EUTBL and USTBL
3. **Initialize Minter** — creates MinterConfig PDA (max delay = 86400s)
4. **Initialize Redemption** — creates RedemptionConfig PDA
5. **Link programs** — sets redemption contract, daily limits (5M shares), and redemption minimums (1 share) for each mint
6. **Grant admin roles** — grants all 8 roles (bits 0-7) to the deployer wallet
7. **Grant MinterConfig PDA the MINTER role** — enables the Minter -> SpikoToken CPI chain
8. **Grant vault authority PDA roles** — grants WHITELISTED + BURNER to the vault authority PDA (required for the redemption flow)

Mint keypairs are read from `scripts/keys/eutbl-mint.json` and `scripts/keys/ustbl-mint.json`. On first run, generate them:

```bash
solana-keygen new -o scripts/keys/eutbl-mint.json --no-bip39-passphrase
solana-keygen new -o scripts/keys/ustbl-mint.json --no-bip39-passphrase
```

## Operational Scripts

All scripts run from the `scripts/` directory and use the deployer keypair at `~/.config/solana/id.json`.

### Whitelist a user

Grants `ROLE_WHITELISTED` so the user can hold and transfer tokens.

```bash
npx tsx whitelist.ts <USER_ADDRESS>
```

Example:

```bash
npx tsx whitelist.ts 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU
```

### Mint tokens

Mints tokens to a whitelisted recipient via the Minter program (enforces daily limits).

```bash
npx tsx mint.ts <RECIPIENT_ADDRESS> <AMOUNT> <TOKEN>
```

- `AMOUNT` is in whole shares (e.g., `100` = 100 shares = 10,000,000 base units at 5 decimals)
- `TOKEN` is `eutbl` or `ustbl`

Example — mint 100 EUTBL:

```bash
npx tsx mint.ts 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU 100 eutbl
```

### Burn tokens (admin-forced)

Burns tokens from any holder's account using the PermanentDelegate authority. Requires `ROLE_BURNER`.

```bash
npx tsx burn.ts <HOLDER_ADDRESS> <AMOUNT> <TOKEN>
```

Example — burn 50 USTBL from a holder:

```bash
npx tsx burn.ts 7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU 50 ustbl
```

### Redeem tokens (initiate)

Initiates a redemption: transfers tokens from the user to the vault and creates a PENDING redemption operation with a 14-day deadline. The user must be whitelisted.

```bash
npx tsx redeem.ts <eutbl|ustbl> <AMOUNT_IN_SHARES> [SALT]
```

Example — redeem 10 EUTBL:

```bash
npx tsx redeem.ts eutbl 10
```

The script prints the salt — save it, you'll need it for execute or cancel.

### Execute redemption (operator)

Burns the vault tokens for a pending redemption. Requires `ROLE_REDEMPTION_EXECUTOR`. Must be called before the 14-day deadline.

```bash
npx tsx execute-redemption.ts <eutbl|ustbl> <USER_ADDRESS> <AMOUNT_IN_SHARES> <SALT>
```

Example:

```bash
npx tsx execute-redemption.ts eutbl 3QpaUQVDFTKKK56pDRxmVcX1rMjEjXRcLECmeAaGs9Sx 10 12345
```

### Cancel redemption (after deadline)

Refunds tokens from the vault back to the user. Anyone can call, but only after the 14-day deadline has passed.

```bash
npx tsx cancel-redemption.ts <eutbl|ustbl> <USER_ADDRESS> <AMOUNT_IN_SHARES> <SALT>
```

Example:

```bash
npx tsx cancel-redemption.ts eutbl 3QpaUQVDFTKKK56pDRxmVcX1rMjEjXRcLECmeAaGs9Sx 10 12345
```

## Event System

All four programs emit **Anchor-compatible structured events** via the `sol_log_data` syscall. Events appear in transaction logs as `Program data: <base64>` entries and can be decoded by any Anchor-compatible tooling.

### Discriminator Computation

Each event is identified by an **8-byte discriminator** — the first 8 bytes of the SHA-256 hash of the string `"event:<EventName>"`:

```
discriminator = SHA256("event:<EventName>")[0..8]
```

For example, for the `RoleGranted` event:

```
SHA256("event:RoleGranted") = dcb759e48f3ff63a...
discriminator = [0xdc, 0xb7, 0x59, 0xe4, 0x8f, 0x3f, 0xf6, 0x3a]
```

This is the same convention used by Anchor programs, so standard Anchor event parsers work out of the box.

**Verify from the command line:**

```bash
echo -n "event:RoleGranted" | shasum -a 256
# dcb759e48f3ff63a... → first 16 hex chars = first 8 bytes
```

**Compute in Node.js / TypeScript:**

```ts
async function eventDiscriminator(eventName: string): Promise<Uint8Array> {
  const hash = await crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(`event:${eventName}`),
  );
  return new Uint8Array(hash).slice(0, 8);
}

// Example:
const disc = await eventDiscriminator("RoleGranted");
// → Uint8Array [0xdc, 0xb7, 0x59, 0xe4, 0x8f, 0x3f, 0xf6, 0x3a]
```

**Compute in Rust (build script or offline):**

```rust
use sha2::{Sha256, Digest};

let hash = Sha256::digest(b"event:RoleGranted");
let disc: [u8; 8] = hash[..8].try_into().unwrap();
// → [0xdc, 0xb7, 0x59, 0xe4, 0x8f, 0x3f, 0xf6, 0x3a]
```

### Event Payload Format

Each event payload is a byte buffer: `discriminator (8 bytes) + fields (LE-packed)`.

- **Addresses** are 32 bytes (raw public key bytes)
- **u64 / i64** are 8 bytes, little-endian
- **u8** is 1 byte

### Event Reference

#### Permission Manager (5 events)

| Event                          | Hash Input                           | Discriminator      | Fields                                                   |
| ------------------------------ | ------------------------------------ | ------------------ | -------------------------------------------------------- |
| `PermissionManagerInitialized` | `event:PermissionManagerInitialized` | `cf1e6038fda9c50f` | `admin: pubkey(32)`                                      |
| `RoleGranted`                  | `event:RoleGranted`                  | `dcb759e48f3ff63a` | `caller: pubkey(32), target: pubkey(32), role_id: u8(1)` |
| `RoleRevoked`                  | `event:RoleRevoked`                  | `a7b734e57ece3e3d` | `caller: pubkey(32), target: pubkey(32), role_id: u8(1)` |
| `OwnershipTransferStarted`     | `event:OwnershipTransferStarted`     | `b7fdeff68cb38569` | `admin: pubkey(32), new_admin: pubkey(32)`               |
| `OwnershipTransferred`         | `event:OwnershipTransferred`         | `ac3dcdb7fa322662` | `new_admin: pubkey(32)`                                  |

#### SpikoToken (8 events)

| Event                   | Hash Input                    | Discriminator      | Fields                                                                                              |
| ----------------------- | ----------------------------- | ------------------ | --------------------------------------------------------------------------------------------------- |
| `TokenInitialized`      | `event:TokenInitialized`      | `4d46e97cec5ccc00` | `admin: pubkey(32), mint: pubkey(32)`                                                               |
| `TokensMinted`          | `event:TokensMinted`          | `cfd480c2af364018` | `caller: pubkey(32), mint: pubkey(32), recipient_ata: pubkey(32), amount: u64(8)`                   |
| `TokensBurned`          | `event:TokensBurned`          | `e6ff2271e235e309` | `caller: pubkey(32), mint: pubkey(32), source_ata: pubkey(32), amount: u64(8)`                      |
| `TokensTransferred`     | `event:TokensTransferred`     | `8c566a26559dcafa` | `sender: pubkey(32), mint: pubkey(32), source: pubkey(32), destination: pubkey(32), amount: u64(8)` |
| `RedeemInitiated`       | `event:RedeemInitiated`       | `47dc92b90bdcf513` | `user: pubkey(32), mint: pubkey(32), amount: u64(8), salt: u64(8)`                                  |
| `TokenPaused`           | `event:TokenPaused`           | `7e364ca17d97943b` | `caller: pubkey(32), config: pubkey(32)`                                                            |
| `TokenUnpaused`         | `event:TokenUnpaused`         | `e1114451818691a9` | `caller: pubkey(32), config: pubkey(32)`                                                            |
| `RedemptionContractSet` | `event:RedemptionContractSet` | `bdb31c22e363f63a` | `caller: pubkey(32), config: pubkey(32), contract: pubkey(32)`                                      |

#### Minter (7 events)

| Event               | Hash Input                | Discriminator      | Fields                                                                                   |
| ------------------- | ------------------------- | ------------------ | ---------------------------------------------------------------------------------------- |
| `MinterInitialized` | `event:MinterInitialized` | `b18962b316ce37c0` | `admin: pubkey(32), max_delay: i64(8)`                                                   |
| `MintExecuted`      | `event:MintExecuted`      | `37876c4905beed2c` | `caller: pubkey(32), user: pubkey(32), mint: pubkey(32), amount: u64(8), salt: u64(8)`   |
| `MintBlocked`       | `event:MintBlocked`       | `7eee83cdfd6ef523` | `caller: pubkey(32), user: pubkey(32), mint: pubkey(32), amount: u64(8), salt: u64(8)`   |
| `MintApproved`      | `event:MintApproved`      | `0244e91866416823` | `approver: pubkey(32), user: pubkey(32), mint: pubkey(32), amount: u64(8), salt: u64(8)` |
| `MintCanceled`      | `event:MintCanceled`      | `a84a139d4addc019` | `caller: pubkey(32), user: pubkey(32), mint: pubkey(32), amount: u64(8), salt: u64(8)`   |
| `DailyLimitUpdated` | `event:DailyLimitUpdated` | `4108e7add7b647c9` | `caller: pubkey(32), mint: pubkey(32), limit: u64(8)`                                    |
| `MaxDelayUpdated`   | `event:MaxDelayUpdated`   | `8151911a62d2a00c` | `caller: pubkey(32), max_delay: i64(8)`                                                  |

#### Redemption (5 events)

| Event                   | Hash Input                    | Discriminator      | Fields                                                                                   |
| ----------------------- | ----------------------------- | ------------------ | ---------------------------------------------------------------------------------------- |
| `RedemptionInitialized` | `event:RedemptionInitialized` | `6ac86472946426cb` | `admin: pubkey(32)`                                                                      |
| `RedemptionCreated`     | `event:RedemptionCreated`     | `13123e3c8d46a96f` | `user: pubkey(32), mint: pubkey(32), amount: u64(8), salt: u64(8), deadline: i64(8)`     |
| `RedemptionExecuted`    | `event:RedemptionExecuted`    | `aeda0538242e35d4` | `operator: pubkey(32), user: pubkey(32), mint: pubkey(32), amount: u64(8), salt: u64(8)` |
| `RedemptionCanceled`    | `event:RedemptionCanceled`    | `bdf4d0e83c68e7a4` | `caller: pubkey(32), user: pubkey(32), mint: pubkey(32), amount: u64(8), salt: u64(8)`   |
| `TokenMinimumUpdated`   | `event:TokenMinimumUpdated`   | `eb3c994761d4706e` | `caller: pubkey(32), mint: pubkey(32), minimum: u64(8)`                                  |
