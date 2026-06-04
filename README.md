# Spiko Solana Contracts

Tokenized money market fund shares on Solana, built with Anchor and Token-2022.

## Architecture

- **Minter** — Custom Anchor program for controlled token minting (initiate → approve flow with daily limits)
- **Token ACL** (`TACLkU6CiCdkQN2MjoyDkVg2yAH9zkxiHDsiztQ52TP`) — Freeze authority delegation + permissionless thaw
- **ABL Gate** (`GATEzzqxhJnsWF6vHRsgtixxSB8PaQdcqGEVTEHWiULz`) — Allow/block list management (composite mode: allow for KYC, block for sanctions)

All new token accounts start **frozen** (DefaultAccountState). Users are thawed permissionlessly after being added to the allow list. Block list always wins.

## Program IDs

### Devnet

| Program | Address                                        |
| ------- | ---------------------------------------------- |
| Minter  | `Az2K7mBaAJpkH8ekiHq89zVAtUAEPG4ZhugbtHAPBHTc` |

## Authorities

### Devnet

#### Squads

| Name                       | Multisig Account                               | Squad Vault                                    |
| -------------------------- | ---------------------------------------------- | ---------------------------------------------- |
| Minter Admin               | `3ynDxXhWUe2e4qj35rEAXnzJZLMxYNhTkLmekSz3yZTv` | `DbvTDctFR9vg9Zr9B3AXwuijwaEG2CrQsAbVRJGDLXcd` |
| Gate Authority             | `2UPy4twDntnEGAtzPwSgbKFuH7JrPU6RXtGdCzPLnNok` | `4wMDSynaKhXyHThzhDugX61bcRQW1FQdPCQ3ap5e8vkN` |
| Permanent Delegate         | `9Nduu43LCQ6CCCZVezots6Yfrwx3BKPNBtGD583L8Q5A` | `HPiFoPhj9GBp4R36tZDgYx5EyvBiY7sKr6YQo3hNVKBF` |
| Metadata Pointer Authority | `9CDu7eu8ViFSozLgo2HfLcbwLNAw5uLJdojHxvcApFJU` | `FocY7ZDCpZ6mBDtnmiQvKf5Cc1VvE4qmnH8Uh6KKAogq` |
| Metadata Update Authority  | `5thvQzhm8cqeqRPvcDzr2Mp6brThsa8mwQFuDRRNRBQ`  | `4MmywJDnBM21o2VW4Eg5ycvVpD6HSbi8FESy4rECfYHq` |
| Pause Authority            | `8GLJnGqrtUYrzfNbXov3BcZvJaymhodRLcB5C136mAkj` | `9fcM9RjgMbczEBShPfD7q424sbDSD7h8zXvq685kBFRj` |
| Mint Close Authority       | `FE2zryGDUEPdwVernzNDLFV7L9rU4vHyfEwyFHJgqW4k` | `22fFELs3pQztY75UNzU27Q6j1Sv5tjuMSxANHMkP8yj2` |
| Token Acl Authority        | `6xACzLmDsZEEM9iXiwDmJmwkPRnMNk2g7DPq82sYeyiZ` | `5HhTmXJwmg62wbtjbK9jjcsiz6btc37iQ5G753cAqTqi` |

#### Operational Keypairs

| Name             | Address                                        |
| ---------------- | ---------------------------------------------- |
| Minter Initiator | `5kx1nLkKyqG2UyAMtb5yhWVurZ9mqUnUabrGYdtkZoNM` |

## Setup

Prerequisites: Rust (stable), Solana CLI 2.x, Anchor CLI 1.0.2, Node.js 20+, pnpm.

```bash
pnpm install

# Download SPL Token 2022 program (required for tests)
solana program dump TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb target/deploy/spl_token_2022.so --url mainnet-beta
```

## Common Commands

```bash
# Build
anchor build -p minter

# Run LiteSVM unit tests
cargo test -p minter

# Regenerate TypeScript clients (Codama)
pnpm generate-clients
```

## Deployment Scripts

Scripts target `devnet` or `mainnet-beta` only.

### 1. Setup Minter

First-time deploy + initialization. Transfers config admin to a multisig. Upgrade authority stays with the deployer keypair.

```bash
### Devnet
pnpm tsx scripts/setup-minter.ts \
  --cluster devnet \
  --keypair ./deployer.json \
  --minter-admin DbvTDctFR9vg9Zr9B3AXwuijwaEG2CrQsAbVRJGDLXcd \
  --mint-initiator 5kx1nLkKyqG2UyAMtb5yhWVurZ9mqUnUabrGYdtkZoNM
```

| Flag               | Description                             |
| ------------------ | --------------------------------------- |
| `--cluster`        | `devnet` or `mainnet-beta`              |
| `--keypair`        | Deployer keypair (pays + initial admin) |
| `--minter-admin`   | Final config admin (Squads multisig)    |
| `--mint-initiator` | Pubkey authorized to initiate mints     |

**Output:** `deployments/minter-<cluster>.json`

### 2. Setup ACL

One-time creation of the shared ABL Gate allow/block lists. The gate-authority becomes the immutable list owner.

- On **devnet** with `--multisig-pubkey`: automatically creates and executes the vault transactions via the Squads SDK.
- On **mainnet** (or without `--multisig-pubkey`): prints raw instruction details for manual execution via the Squads web UI.

```bash
### Devnet
pnpm tsx scripts/setup-acl.ts \
  --cluster devnet \
  --keypair ./deployer.json \
  --gate-authority 4wMDSynaKhXyHThzhDugX61bcRQW1FQdPCQ3ap5e8vkN \
  --multisig-pubkey 2UPy4twDntnEGAtzPwSgbKFuH7JrPU6RXtGdCzPLnNok \
  --vault-index 0
```

| Flag                | Description                                       |
| ------------------- | ------------------------------------------------- |
| `--cluster`         | `devnet` or `mainnet-beta`                        |
| `--keypair`         | Payer keypair (devnet: also the proposer/member)  |
| `--gate-authority`  | ABL list authority (immutable, e.g. Squads vault) |
| `--multisig-pubkey` | Squads multisig account (optional, devnet only)   |
| `--vault-index`     | Vault index (default: `0`, optional, devnet only) |

**Output:** `deployments/acl-<cluster>.json`

### 3. Setup Token

Creates a Token-2022 mint with all extensions, configures Token ACL, and transfers authorities. Requires `setup-minter` and `setup-acl` to have been run first.

```bash
### Devnet
pnpm tsx scripts/setup-token.ts \
  --cluster devnet \
  --keypair ./deployer.json \
  --symbol EUTBL \
  --name "EU T-Bill" \
  --uri "https://spiko.finance/eutbl" \
  --decimals 6 \
  --minter-daily-limit 1000000 \
  --permanent-delegate HPiFoPhj9GBp4R36tZDgYx5EyvBiY7sKr6YQo3hNVKBF \
  --metadata-pointer-authority FocY7ZDCpZ6mBDtnmiQvKf5Cc1VvE4qmnH8Uh6KKAogq \
  --metadata-update-authority 4MmywJDnBM21o2VW4Eg5ycvVpD6HSbi8FESy4rECfYHq \
  --pause-authority 9fcM9RjgMbczEBShPfD7q424sbDSD7h8zXvq685kBFRj \
  --mint-close-authority 22fFELs3pQztY75UNzU27Q6j1Sv5tjuMSxANHMkP8yj2 \
  --token-acl-authority 5HhTmXJwmg62wbtjbK9jjcsiz6btc37iQ5G753cAqTqi \
  --multisig-pubkey 3ynDxXhWUe2e4qj35rEAXnzJZLMxYNhTkLmekSz3yZTv \
  --vault-index 0
```

| Flag                           | Description                                       |
| ------------------------------ | ------------------------------------------------- |
| `--cluster`                    | `devnet` or `mainnet-beta`                        |
| `--keypair`                    | Payer/temp authority keypair                      |
| `--symbol`                     | Token symbol (e.g. `EUTBL`)                       |
| `--name`                       | Token display name                                |
| `--uri`                        | Metadata URI                                      |
| `--decimals`                   | Token decimals (default: 6)                       |
| `--minter-daily-limit`         | Daily limit in whole token units                  |
| `--permanent-delegate`         | Permanent delegate (immutable!)                   |
| `--metadata-pointer-authority` | Metadata pointer authority (set at init)          |
| `--metadata-update-authority`  | Final metadata update authority                   |
| `--pause-authority`            | Pause authority (set at init)                     |
| `--mint-close-authority`       | Mint close authority (set at init)                |
| `--token-acl-authority`        | Final Token ACL config authority                  |
| `--multisig-pubkey`            | Squads multisig account (optional, devnet only)   |
| `--vault-index`                | Vault index (default: `0`, optional, devnet only) |

After execution, the keypair has no remaining power.

- On **devnet** with `--multisig-pubkey`: automatically creates and executes the Squads vault transactions for **Minter setDailyLimit**.
- On **mainnet** (or without `--multisig-pubkey`): prints instruction details for manual execution via the Squads web UI.

**Output:** `deployments/{SYMBOL}-{cluster}.json` + mint keypair saved to `deployments/`
