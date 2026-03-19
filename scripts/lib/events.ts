/**
 * Event decoder for Spiko program events.
 *
 * Decodes Anchor-compatible structured events emitted via `sol_log_data`.
 * Each event payload: discriminator(8) + LE-packed fields.
 * Discriminator = SHA256("event:<EventName>")[0..8].
 */

import {
  type Address,
  type Rpc,
  type SolanaRpcApi,
  getAddressDecoder,
} from "@solana/kit";

import { ROLE_NAMES, TOKEN_DECIMALS } from "./constants.js";

// =================================================================
// Field types for event decoding
// =================================================================

type FieldType = "address" | "u64" | "i64" | "u8";

interface FieldDef {
  name: string;
  type: FieldType;
}

interface EventDef {
  name: string;
  program: string;
  disc: Uint8Array;
  fields: FieldDef[];
}

export interface DecodedEvent {
  name: string;
  program: string;
  fields: Record<string, string | bigint | number>;
}

// =================================================================
// Address decoder (shared)
// =================================================================

const addressDecoder = getAddressDecoder();

// =================================================================
// All 25 event definitions
// =================================================================

const EVENT_DEFS: EventDef[] = [
  // ── Permission Manager (5) ──────────────────────────────────
  {
    // SHA256("event:PermissionManagerInitialized")[0..8]
    name: "PermissionManagerInitialized",
    program: "PermissionManager",
    disc: new Uint8Array([0xcf, 0x1e, 0x60, 0x38, 0xfd, 0xa9, 0xc5, 0x0f]),
    fields: [{ name: "admin", type: "address" }],
  },
  {
    // SHA256("event:RoleGranted")[0..8]
    name: "RoleGranted",
    program: "PermissionManager",
    disc: new Uint8Array([0xdc, 0xb7, 0x59, 0xe4, 0x8f, 0x3f, 0xf6, 0x3a]),
    fields: [
      { name: "caller", type: "address" },
      { name: "target", type: "address" },
      { name: "role_id", type: "u8" },
    ],
  },
  {
    // SHA256("event:RoleRemoved")[0..8]
    name: "RoleRemoved",
    program: "PermissionManager",
    disc: new Uint8Array([0x85, 0x23, 0xd6, 0xea, 0xcb, 0x9d, 0xcb, 0x35]),
    fields: [
      { name: "caller", type: "address" },
      { name: "target", type: "address" },
      { name: "role_id", type: "u8" },
    ],
  },
  {
    // SHA256("event:OwnershipTransferStarted")[0..8]
    name: "OwnershipTransferStarted",
    program: "PermissionManager",
    disc: new Uint8Array([0xb7, 0xfd, 0xef, 0xf6, 0x8c, 0xb3, 0x85, 0x69]),
    fields: [
      { name: "admin", type: "address" },
      { name: "new_admin", type: "address" },
    ],
  },
  {
    // SHA256("event:OwnershipTransferred")[0..8]
    name: "OwnershipTransferred",
    program: "PermissionManager",
    disc: new Uint8Array([0xac, 0x3d, 0xcd, 0xb7, 0xfa, 0x32, 0x26, 0x62]),
    fields: [{ name: "new_admin", type: "address" }],
  },

  // ── SpikoToken (7) ─────────────────────────────────────────
  {
    // SHA256("event:TokenInitialized")[0..8]
    name: "TokenInitialized",
    program: "SpikoToken",
    disc: new Uint8Array([0x4d, 0x46, 0xe9, 0x7c, 0xec, 0x5c, 0xcc, 0x00]),
    fields: [
      { name: "admin", type: "address" },
      { name: "mint", type: "address" },
    ],
  },
  {
    // SHA256("event:Mint")[0..8]
    name: "Mint",
    program: "SpikoToken",
    disc: new Uint8Array([0x3f, 0x0b, 0xd5, 0x86, 0x94, 0xc2, 0x18, 0xcb]),
    fields: [
      { name: "caller", type: "address" },
      { name: "mint", type: "address" },
      { name: "recipient_ata", type: "address" },
      { name: "amount", type: "u64" },
    ],
  },
  {
    // SHA256("event:Burn")[0..8]
    name: "Burn",
    program: "SpikoToken",
    disc: new Uint8Array([0xb8, 0x0d, 0x41, 0xce, 0xce, 0xaa, 0x33, 0x55]),
    fields: [
      { name: "caller", type: "address" },
      { name: "mint", type: "address" },
      { name: "source_ata", type: "address" },
      { name: "amount", type: "u64" },
    ],
  },
  {
    // SHA256("event:RedeemInitiated")[0..8]
    name: "RedeemInitiated",
    program: "SpikoToken",
    disc: new Uint8Array([0x47, 0xdc, 0x92, 0xb9, 0x0b, 0xdc, 0xf5, 0x13]),
    fields: [
      { name: "user", type: "address" },
      { name: "mint", type: "address" },
      { name: "amount", type: "u64" },
      { name: "salt", type: "u64" },
    ],
  },
  {
    // SHA256("event:TokenPaused")[0..8]
    name: "TokenPaused",
    program: "SpikoToken",
    disc: new Uint8Array([0x7e, 0x36, 0x4c, 0xa1, 0x7d, 0x97, 0x94, 0x3b]),
    fields: [
      { name: "caller", type: "address" },
      { name: "config", type: "address" },
    ],
  },
  {
    // SHA256("event:TokenUnpaused")[0..8]
    name: "TokenUnpaused",
    program: "SpikoToken",
    disc: new Uint8Array([0xe1, 0x11, 0x44, 0x51, 0x81, 0x86, 0x91, 0xa9]),
    fields: [
      { name: "caller", type: "address" },
      { name: "config", type: "address" },
    ],
  },
  {
    // SHA256("event:RedemptionContractSet")[0..8]
    name: "RedemptionContractSet",
    program: "SpikoToken",
    disc: new Uint8Array([0xbd, 0xb3, 0x1c, 0x22, 0xe3, 0x63, 0xf6, 0x3a]),
    fields: [
      { name: "caller", type: "address" },
      { name: "config", type: "address" },
      { name: "contract", type: "address" },
    ],
  },

  // ── Transfer Hook (1) ──────────────────────────────────────
  {
    // SHA256("event:Transfer")[0..8]
    name: "Transfer",
    program: "TransferHook",
    disc: new Uint8Array([0x19, 0x12, 0x17, 0x07, 0xac, 0x74, 0x82, 0x1c]),
    fields: [
      { name: "sender", type: "address" },
      { name: "mint", type: "address" },
      { name: "source", type: "address" },
      { name: "destination", type: "address" },
      { name: "amount", type: "u64" },
    ],
  },

  // ── Minter (7) ─────────────────────────────────────────────
  {
    // SHA256("event:MinterInitialized")[0..8]
    name: "MinterInitialized",
    program: "Minter",
    disc: new Uint8Array([0xb1, 0x89, 0x62, 0xb3, 0x16, 0xce, 0x37, 0xc0]),
    fields: [
      { name: "admin", type: "address" },
      { name: "max_delay", type: "i64" },
    ],
  },
  {
    // SHA256("event:MintExecuted")[0..8]
    name: "MintExecuted",
    program: "Minter",
    disc: new Uint8Array([0x37, 0x87, 0x6c, 0x49, 0x05, 0xbe, 0xed, 0x2c]),
    fields: [
      { name: "caller", type: "address" },
      { name: "user", type: "address" },
      { name: "mint", type: "address" },
      { name: "amount", type: "u64" },
      { name: "salt", type: "u64" },
    ],
  },
  {
    // SHA256("event:MintBlocked")[0..8]
    name: "MintBlocked",
    program: "Minter",
    disc: new Uint8Array([0x7e, 0xee, 0x83, 0xcd, 0xfd, 0x6e, 0xf5, 0x23]),
    fields: [
      { name: "caller", type: "address" },
      { name: "user", type: "address" },
      { name: "mint", type: "address" },
      { name: "amount", type: "u64" },
      { name: "salt", type: "u64" },
    ],
  },
  {
    // SHA256("event:MintApproved")[0..8]
    name: "MintApproved",
    program: "Minter",
    disc: new Uint8Array([0x02, 0x44, 0xe9, 0x18, 0x66, 0x41, 0x68, 0x23]),
    fields: [
      { name: "approver", type: "address" },
      { name: "user", type: "address" },
      { name: "mint", type: "address" },
      { name: "amount", type: "u64" },
      { name: "salt", type: "u64" },
    ],
  },
  {
    // SHA256("event:MintCanceled")[0..8]
    name: "MintCanceled",
    program: "Minter",
    disc: new Uint8Array([0xa8, 0x4a, 0x13, 0x9d, 0x4a, 0xdd, 0xc0, 0x19]),
    fields: [
      { name: "caller", type: "address" },
      { name: "user", type: "address" },
      { name: "mint", type: "address" },
      { name: "amount", type: "u64" },
      { name: "salt", type: "u64" },
    ],
  },
  {
    // SHA256("event:DailyLimitUpdated")[0..8]
    name: "DailyLimitUpdated",
    program: "Minter",
    disc: new Uint8Array([0x41, 0x08, 0xe7, 0xad, 0xd7, 0xb6, 0x47, 0xc9]),
    fields: [
      { name: "caller", type: "address" },
      { name: "mint", type: "address" },
      { name: "limit", type: "u64" },
    ],
  },
  {
    // SHA256("event:MaxDelayUpdated")[0..8]
    name: "MaxDelayUpdated",
    program: "Minter",
    disc: new Uint8Array([0x81, 0x51, 0x91, 0x1a, 0x62, 0xd2, 0xa0, 0x0c]),
    fields: [
      { name: "caller", type: "address" },
      { name: "max_delay", type: "i64" },
    ],
  },

  // ── Redemption (5) ─────────────────────────────────────────
  {
    // SHA256("event:RedemptionInitialized")[0..8]
    name: "RedemptionInitialized",
    program: "Redemption",
    disc: new Uint8Array([0x6a, 0xc8, 0x64, 0x72, 0x94, 0x64, 0x26, 0xcb]),
    fields: [{ name: "admin", type: "address" }],
  },
  {
    // SHA256("event:RedemptionInitiated")[0..8]
    name: "RedemptionInitiated",
    program: "Redemption",
    disc: new Uint8Array([0x55, 0xfe, 0xeb, 0x0e, 0xdd, 0x88, 0x60, 0xde]),
    fields: [
      { name: "user", type: "address" },
      { name: "mint", type: "address" },
      { name: "amount", type: "u64" },
      { name: "salt", type: "u64" },
      { name: "deadline", type: "i64" },
    ],
  },
  {
    // SHA256("event:RedemptionExecuted")[0..8]
    name: "RedemptionExecuted",
    program: "Redemption",
    disc: new Uint8Array([0xae, 0xda, 0x05, 0x38, 0x24, 0x2e, 0x35, 0xd4]),
    fields: [
      { name: "operator", type: "address" },
      { name: "user", type: "address" },
      { name: "mint", type: "address" },
      { name: "amount", type: "u64" },
      { name: "salt", type: "u64" },
    ],
  },
  {
    // SHA256("event:RedemptionCanceled")[0..8]
    name: "RedemptionCanceled",
    program: "Redemption",
    disc: new Uint8Array([0xbd, 0xf4, 0xd0, 0xe8, 0x3c, 0x68, 0xe7, 0xa4]),
    fields: [
      { name: "caller", type: "address" },
      { name: "user", type: "address" },
      { name: "mint", type: "address" },
      { name: "amount", type: "u64" },
      { name: "salt", type: "u64" },
    ],
  },
  {
    // SHA256("event:TokenMinimumUpdated")[0..8]
    name: "TokenMinimumUpdated",
    program: "Redemption",
    disc: new Uint8Array([0xeb, 0x3c, 0x99, 0x47, 0x61, 0xd4, 0x70, 0x6e]),
    fields: [
      { name: "caller", type: "address" },
      { name: "mint", type: "address" },
      { name: "minimum", type: "u64" },
    ],
  },
];

// =================================================================
// Discriminator lookup map
// =================================================================

/** Convert 8-byte discriminator to hex string key for fast lookup. */
function discKey(disc: Uint8Array): string {
  return Array.from(disc.slice(0, 8))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

const DISC_MAP = new Map<string, EventDef>();
for (const def of EVENT_DEFS) {
  DISC_MAP.set(discKey(def.disc), def);
}

// =================================================================
// Field decoders
// =================================================================

function decodeAddress(data: Uint8Array, offset: number): [Address, number] {
  const bytes = data.slice(offset, offset + 32);
  return [addressDecoder.decode(bytes), offset + 32];
}

function decodeU64(data: Uint8Array, offset: number): [bigint, number] {
  const view = new DataView(data.buffer, data.byteOffset + offset, 8);
  return [view.getBigUint64(0, true), offset + 8];
}

function decodeI64(data: Uint8Array, offset: number): [bigint, number] {
  const view = new DataView(data.buffer, data.byteOffset + offset, 8);
  return [view.getBigInt64(0, true), offset + 8];
}

function decodeU8(data: Uint8Array, offset: number): [number, number] {
  return [data[offset], offset + 1];
}

// =================================================================
// Core decoder
// =================================================================

/**
 * Decode a single event from raw bytes.
 * Returns null if the discriminator is not recognized.
 */
export function decodeEvent(data: Uint8Array): DecodedEvent | null {
  if (data.length < 8) return null;

  const key = discKey(data);
  const def = DISC_MAP.get(key);
  if (!def) return null;

  const fields: Record<string, string | bigint | number> = {};
  let offset = 8; // skip discriminator

  for (const field of def.fields) {
    switch (field.type) {
      case "address": {
        const [val, next] = decodeAddress(data, offset);
        fields[field.name] = val;
        offset = next;
        break;
      }
      case "u64": {
        const [val, next] = decodeU64(data, offset);
        fields[field.name] = val;
        offset = next;
        break;
      }
      case "i64": {
        const [val, next] = decodeI64(data, offset);
        fields[field.name] = val;
        offset = next;
        break;
      }
      case "u8": {
        const [val, next] = decodeU8(data, offset);
        fields[field.name] = val;
        offset = next;
        break;
      }
    }
  }

  return { name: def.name, program: def.program, fields };
}

/**
 * Extract and decode all Spiko events from transaction log messages.
 *
 * Scans for `"Program data: <base64>"` lines, base64-decodes them,
 * and matches against the 25 known event discriminators.
 */
export function decodeEventsFromLogs(logs: string[]): DecodedEvent[] {
  const events: DecodedEvent[] = [];

  for (const line of logs) {
    const match = line.match(/^Program data: (.+)$/);
    if (!match) continue;

    const base64 = match[1];
    const data = Buffer.from(base64, "base64");

    const event = decodeEvent(new Uint8Array(data));
    if (event) {
      events.push(event);
    }
  }

  return events;
}

/**
 * Fetch a confirmed transaction by signature and decode its events.
 */
export async function parseTransactionEvents(
  rpc: Rpc<SolanaRpcApi>,
  signature: string,
  maxRetries = 5,
  retryDelayMs = 2000,
): Promise<DecodedEvent[]> {
  // Retry loop — devnet RPC may need time to index the transaction
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    const tx = await rpc
      .getTransaction(signature as any, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
        encoding: "json",
      })
      .send();

    if (tx?.meta?.logMessages) {
      return decodeEventsFromLogs(tx.meta.logMessages as string[]);
    }

    if (attempt < maxRetries) {
      await new Promise((r) => setTimeout(r, retryDelayMs));
    }
  }

  return [];
}

// =================================================================
// Pretty-print helpers
// =================================================================

/**
 * Format a decoded event as a human-readable string block.
 */
export function formatEvent(event: DecodedEvent): string {
  const lines: string[] = [];
  lines.push(`  [${event.program}] ${event.name}`);

  for (const [key, value] of Object.entries(event.fields)) {
    let display: string;
    if (typeof value === "bigint") {
      // For amount-like fields, also show human-readable shares
      if (key === "amount" || key === "limit" || key === "minimum") {
        const shares = Number(value) / 10 ** TOKEN_DECIMALS;
        display = `${value} (${shares} shares)`;
      } else if (key === "salt") {
        display = `${value}`;
      } else if (key === "max_delay" || key === "deadline") {
        display = `${value}`;
        if (key === "deadline") {
          display += ` (${new Date(Number(value) * 1000).toISOString()})`;
        }
      } else {
        display = `${value}`;
      }
    } else if (typeof value === "number") {
      // u8 fields — for role_id, show role name
      if (key === "role_id") {
        const roleName = ROLE_NAMES[value] || `UNKNOWN(${value})`;
        display = `${value} (${roleName})`;
      } else {
        display = `${value}`;
      }
    } else {
      display = value;
    }

    lines.push(`    ${key}: ${display}`);
  }

  return lines.join("\n");
}

/**
 * Format multiple events.
 */
export function formatEvents(events: DecodedEvent[]): string {
  if (events.length === 0) return "  (no events decoded)";
  return events.map(formatEvent).join("\n\n");
}
