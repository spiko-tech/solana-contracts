# Spiko Solana Contracts

Tokenized money market fund shares on Solana, built with Pinocchio and Token-2022.

## Program IDs

### Devnet

| Program             | Address                                        |
| ------------------- | ---------------------------------------------- |
| PermissionManager   | `2Qhjh6NXiyQEPBP9tVCkzNtLWERHbggUjbbwje1Mpqsc` |
| SpikoToken          | `3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd` |
| SpikoTransferHook   | `CKV53PkgjvoTmfpzdkbuQc9fMukqu7Qey7kLoSiTwYmY` |
| Minter              | `3pXknoeMQiY44nKBcnwtSSxzuh1uxUHPHggjXcuVLDT2` |
| Redemption          | `8opABJP3fzXuCVUnbzDZqYpnfxmCmeiXUQ49txf6BFWX` |
| CustodialGatekeeper | `4yEpQ3wkwKkWq3ejgu95evdQUhkL1DNVpp4Ptg2HpetY` |

### Mainnet

TBD

## Setup

Prerequisites: Rust (stable), Solana CLI 2.x, Node.js 20+, pnpm, just.

```bash
just install   # install Node.js dependencies
just build     # generate IDL + clients + compile programs
```

## Just Commands

| Command                 | Description                               |
| ----------------------- | ----------------------------------------- |
| `just install`          | Install Node.js dependencies              |
| `just build`            | Generate IDL + clients + compile programs |
| `just generate-idl`     | Generate Codama IDL JSON from Rust source |
| `just generate-clients` | Generate TypeScript clients from IDL      |
| `just check`            | Check Rust code (no .so output)           |
| `just fmt`              | Format Rust code                          |
| `just clippy`           | Run clippy                                |
| `just integration-test` | Run integration tests                     |

## E2E Tests

```bash
cd e2e
pnpm install
pnpm e2e
```

Requires a running Solana validator (devnet or local).
