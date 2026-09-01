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
| Minter  | `8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw` |

### Mainnet

| Program | Address                                        |
| ------- | ---------------------------------------------- |
| Minter  | `8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw` |

The mainnet program is a [verified build](https://solscan.io/account/8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw#programVerification):
the deployed bytes reproduce byte-for-byte from this repository. Check at any time with

```bash
solana-verify get-program-hash -u mainnet-beta 8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw
curl -s https://verify.osec.io/status/8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw
```

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
| Minter Initiator | `F8ZugHpjAdtprW4HNUhEcXk72NBrcWNtaTXHLuJQeuGS` |
| Burn Authority   | `F8ZugHpjAdtprW4HNUhEcXk72NBrcWNtaTXHLuJQeuGS` |

### Mainnet

#### Squads

| Name                       | Multisig Account                               | Squad Vault                                    |
| -------------------------- | ---------------------------------------------- | ---------------------------------------------- |
| Minter Admin               | `CqexeLYpGafaLfX7Nb3tD8ShLmWP1mmap16iR9M59u1A` | `2duhrjMfZdX1TX3sKzc9ZPkdJ1TtnXVgCYcxToTq3Kv9` |
| Gate Authority             | `C3pBB4Rc8X5xT8hRA3q8WtAxdoErmzBQhFvPXr4hoTuZ` | `8Q62NJJnQuvnEVBg1Dw4XWxc4E4kPLtqDNv79EgmNRoY` |
| Permanent Delegate         | `2z8Xu665SJRSFa77Mbrw4f48VJVEfCYGjvyr9WEUhguC` | `EeJqADE9Xqkqoyjid6H1GmbwDZ4Bz1qyC7ySpmFq849m` |
| Metadata Pointer Authority | `GnnxydxK1YWBDwUkJs9rwJshxZH9W3L9Mbf3PZ6sxYga` | `E5pzf6p7n3UYP8B6q7LdG7sT8GfQcwbZ8beFDqZ26Y75` |
| Metadata Update Authority  | `B7rC3Qs7c4nnqiokNR2EfLV3Lz59cQpRK7MkomSAW2g2` | `9sM8XStBdZnXqNsKfNfqkHE8ggr7U1koDeEDqMXCFBLg` |
| Pause Authority            | `UDNfKHVduuhannhBtYmY727JnTpfNyk3AMfQ9mBdPG8`  | `3wVzYmWt79gBpPtWBK8GHvRfii1SARpP8SKqEKuzJsDY` |
| Mint Close Authority       | `m2Ez19erx2dkDPZYqyMUq4jiiCa5VbosP75cckcwVxT`  | `7ano9sb3YoEPkYc18uDphWVLYyxvVCNVz31QSAcf3B6C` |
| Token Acl Authority        | `2xT7Y4E8Cpp6TP153ACChxVjZoNxtfNCHBx4SmBvG3s2` | `63iE6ayimMdRQ3fSScC6zSYZXrtmmBct5BsvWoNXcMXJ` |

#### Operational Keypairs

| Name             | Address                                        |
| ---------------- | ---------------------------------------------- |
| Minter Initiator | `5CgAHs1K779jzqgB1K5kgTYozcexBGF9voi9zo2WCPMd` |
| Burn Authority   | `5CgAHs1K779jzqgB1K5kgTYozcexBGF9voi9zo2WCPMd` |

## Audit reports

| Auditor  | Date      | Link                                                                                                          |
| -------- | --------- | ------------------------------------------------------------------------------------------------------------- |
| OtterSec | June 2026 | https://github.com/AdevarLabs/audit-reports/blob/main/reports/2026-06-12_Spiko_Solana_Minter_audit_report.pdf |

## Setup

Prerequisites: Rust (stable), Solana CLI 3.1.10, Anchor CLI 1.0.2, Node.js 20+, pnpm.

```bash
pnpm install
```

The SPL Token 2022 program needed by the tests is vendored at
`programs/minter/tests/fixtures/spl_token_2022.so`, so no download is required. To refresh it
against the current mainnet deployment:

```bash
solana program dump TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb \
  programs/minter/tests/fixtures/spl_token_2022.so --url mainnet-beta
```

## Common Commands

```bash
anchor build -p minter --ignore-keys

cargo test -p minter

# Regenerate TypeScript clients (Codama)
pnpm generate-clients
```

## Deployment Scripts

Scripts target `devnet` or `mainnet-beta` only.

### 1. Setup Minter

First-time deploy + initialization. Nominates the Squads vault as config admin via a two-step transfer (nominate → accept). Upgrade authority stays with the deployer keypair.

- On **devnet**: automatically executes the `accept_admin` vault transaction via the Squads SDK.
- On **mainnet** (`mainnet-beta`): stops after `nominate_admin` and prints instruction details for manual proposal via the Squads web UI.

```bash
### Devnet
pnpm tsx scripts/setup-minter.ts \
  --cluster devnet \
  --keypair ./deployer.json \
  --minter-admin-squad-account 3ynDxXhWUe2e4qj35rEAXnzJZLMxYNhTkLmekSz3yZTv \
  --minter-admin-squad-vault DbvTDctFR9vg9Zr9B3AXwuijwaEG2CrQsAbVRJGDLXcd \
  --mint-initiator F8ZugHpjAdtprW4HNUhEcXk72NBrcWNtaTXHLuJQeuGS

### Mainnet
pnpm tsx scripts/setup-minter.ts \
  --cluster mainnet-beta \
  --keypair ./deployer.json \
  --minter-admin-squad-account CqexeLYpGafaLfX7Nb3tD8ShLmWP1mmap16iR9M59u1A \
  --minter-admin-squad-vault 2duhrjMfZdX1TX3sKzc9ZPkdJ1TtnXVgCYcxToTq3Kv9 \
  --mint-initiator 5CgAHs1K779jzqgB1K5kgTYozcexBGF9voi9zo2WCPMd
```

| Flag                           | Description                                      |
| ------------------------------ | ------------------------------------------------ |
| `--cluster`                    | `devnet` or `mainnet-beta`                       |
| `--keypair`                    | Deployer keypair (pays + initial admin)          |
| `--minter-admin-squad-account` | Squads multisig account pubkey                   |
| `--minter-admin-squad-vault`   | Squads vault PDA (derived, validated at runtime) |
| `--mint-initiator`             | Pubkey authorized to initiate mints              |

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

### Mainnet
pnpm tsx scripts/setup-acl.ts \
  --cluster mainnet-beta \
  --keypair ./deployer.json \
  --gate-authority 8Q62NJJnQuvnEVBg1Dw4XWxc4E4kPLtqDNv79EgmNRoY \
  --multisig-pubkey C3pBB4Rc8X5xT8hRA3q8WtAxdoErmzBQhFvPXr4hoTuZ \
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
  --decimals 5 \
  --minter-daily-limit 1000000 \
  --permanent-delegate HPiFoPhj9GBp4R36tZDgYx5EyvBiY7sKr6YQo3hNVKBF \
  --metadata-pointer-authority FocY7ZDCpZ6mBDtnmiQvKf5Cc1VvE4qmnH8Uh6KKAogq \
  --metadata-update-authority 4MmywJDnBM21o2VW4Eg5ycvVpD6HSbi8FESy4rECfYHq \
  --pause-authority 9fcM9RjgMbczEBShPfD7q424sbDSD7h8zXvq685kBFRj \
  --mint-close-authority 22fFELs3pQztY75UNzU27Q6j1Sv5tjuMSxANHMkP8yj2 \
  --burn-authority F8ZugHpjAdtprW4HNUhEcXk72NBrcWNtaTXHLuJQeuGS \
  --token-acl-authority 5HhTmXJwmg62wbtjbK9jjcsiz6btc37iQ5G753cAqTqi \
  --multisig-pubkey 3ynDxXhWUe2e4qj35rEAXnzJZLMxYNhTkLmekSz3yZTv \
  --vault-index 0

### Mainnet
pnpm tsx scripts/setup-token.ts \
  --cluster mainnet-beta \
  --keypair ./deployer.json \
  --symbol czkSAFO \
  --name "Spiko Amundi Overnight Swap Fund (CZK)" \
  --uri "https://spiko.io" \
  --decimals 5 \
  --minter-daily-limit 125000000 \
  --permanent-delegate EeJqADE9Xqkqoyjid6H1GmbwDZ4Bz1qyC7ySpmFq849m \
  --metadata-pointer-authority E5pzf6p7n3UYP8B6q7LdG7sT8GfQcwbZ8beFDqZ26Y75 \
  --metadata-update-authority 9sM8XStBdZnXqNsKfNfqkHE8ggr7U1koDeEDqMXCFBLg \
  --pause-authority 3wVzYmWt79gBpPtWBK8GHvRfii1SARpP8SKqEKuzJsDY \
  --mint-close-authority 7ano9sb3YoEPkYc18uDphWVLYyxvVCNVz31QSAcf3B6C \
  --burn-authority 5CgAHs1K779jzqgB1K5kgTYozcexBGF9voi9zo2WCPMd \
  --token-acl-authority 63iE6ayimMdRQ3fSScC6zSYZXrtmmBct5BsvWoNXcMXJ
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
| `--burn-authority`             | Burn authority (co-signs every burn, rotatable)   |
| `--token-acl-authority`        | Final Token ACL config authority                  |
| `--multisig-pubkey`            | Squads multisig account (optional, devnet only)   |
| `--vault-index`                | Vault index (default: `0`, optional, devnet only) |

After execution, the keypair has no remaining power.

- On **devnet** with `--multisig-pubkey`: automatically creates and executes the Squads vault transactions for **Minter setDailyLimit**.
- On **mainnet** (or without `--multisig-pubkey`): prints instruction details for manual execution via the Squads web UI.

**Output:** `deployments/{SYMBOL}-{cluster}.json` + mint keypair saved to `deployments/`

## Upgrading the Minter

`setup-minter.ts` is first-deploy only. Once the program is live, upgrades go through
`scripts/upgrade-minter.ts`, which builds reproducibly, deploys, and submits the program for
verification in one pass.

### Why not `anchor build`

The binary that gets deployed is the one produced by **`solana-verify build`** — a build inside
`solanafoundation/solana-verifiable-build:<agave-version>`, not a host build. Only that artifact is
reproducible by a third party, so deploying an `anchor build` output would leave the program
unverifiable. `anchor build` is still run first, but only to regenerate the IDL and the Codama
clients; `solana-verify build` then overwrites `target/deploy/minter.so` with the artifact that is
actually deployed.

The image tag comes from `Cargo.toml` `[workspace.metadata.cli] solana`, which the CI `pins` job
keeps equal to `Anchor.toml` `solana_version`. If those drift, the verified build stops
reproducing — that check is why it exists.

### Prerequisites

- Docker running (the verifiable build runs in a container)
- `cargo install solana-verify`
- The change committed **and pushed** — OtterSec clones the commit from GitHub, so an unpushed
  commit cannot be verified
- The upgrade-authority keypair
- A private RPC endpoint. `api.mainnet-beta.solana.com` will not reliably carry a ~300 KB deploy.

### Run it

```bash
# Dry run: preflight + reproducible build + hash comparison, no chain writes
pnpm tsx scripts/upgrade-minter.ts \
  --cluster mainnet-beta \
  --keypair ./deployer.json \
  --rpc-url <private-rpc> \
  --dry-run

# The real thing
pnpm tsx scripts/upgrade-minter.ts \
  --cluster mainnet-beta \
  --keypair ./deployer.json \
  --rpc-url <private-rpc>
```

| Flag            | Description                                                          |
| --------------- | -------------------------------------------------------------------- |
| `--cluster`     | `devnet` or `mainnet-beta` (default: `mainnet-beta`)                 |
| `--keypair`     | Upgrade authority keypair (asserted against the on-chain authority)  |
| `--rpc-url`     | RPC override; defaults to the public endpoint for the cluster        |
| `--repo-url`    | Repo to verify from (default: this repository)                       |
| `--dry-run`     | Stop before deploying                                                |
| `--skip-verify` | Deploy without submitting for verification                           |

The script refuses to run on a dirty tree, on an unpushed commit, if `clients/ts` is out of date
with the IDL, or if the keypair is not the current upgrade authority. It is idempotent: if the
on-chain hash already matches the build, it skips the deploy and goes straight to verification.

**Output:** updates `deployments/minter-<cluster>.json` with `deployedCommit`, `executableHash`,
`verifiableBuildImage`, `programDataAddress`, `verified`, and `lastUpgradeAt`.

### If the binary grew

The program account is sized at its last deploy. If the new binary is larger, the script aborts and
prints the fix:

```bash
solana program extend 8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw <additional-bytes> \
  -u <rpc> -k ./deployer.json
```

### If the deploy fails midway

Funds stay parked in an intermediate buffer account. Resume rather than restart:

```bash
solana program show --buffers --buffer-authority <deployer-pubkey> -u <rpc>
solana program deploy --buffer <BUFFER> \
  --program-id 8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw -u <rpc> -k ./deployer.json
# or reclaim the rent:
solana program close <BUFFER> -u <rpc> -k ./deployer.json
```

### Re-verifying without redeploying

If the deployed bytes are already correct and only the verification record is missing or stale:

Verification is two steps since `solana-verify` 0.5.x — the `--remote` flag is deprecated. The
upgrade authority first writes the verify PDA, then the remote build worker is queued against it:

```bash
# 1. Upload the verify PDA (signed by the upgrade authority)
solana-verify verify-from-repo https://github.com/spiko-tech/solana-contracts \
  --program-id 8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw \
  --library-name minter \
  --commit-hash <sha> \
  -u <rpc> -k ./deployer.json -y

# 2. Queue the remote job (uploader = the pubkey that signed step 1)
solana-verify remote submit-job \
  --program-id 8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw \
  --uploader GhRFGntEPkDi3ass8HYPqdqWMZQ4F4hs8a3QfNtT622h \
  -u <rpc>

# Status at any time
solana-verify remote get-status --program-id 8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw
```

### CI

`.github/workflows/verify.yml` runs the same reproducible build on every push to `main` and on PRs
touching `programs/`, `Cargo.*`, or `Anchor.toml`. It publishes the executable hash to the job
summary and emits a notice when `main` has drifted from what is deployed on mainnet-beta. Drift is
never a failure — a merged change is expected to differ until it is deployed.

## Security

Report vulnerabilities privately to **<security@spiko.io>**. Please do not open a
public GitHub issue. See [SECURITY.md](SECURITY.md).

Audit reports are listed in the [Audit reports](#audit-reports) section above.

## License

MIT — see [LICENSE](LICENSE).
