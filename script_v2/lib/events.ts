/**
 * Event decoder for Spiko program events (v2).
 *
 * Uses Codama-generated FixedSizeDecoders for struct parsing.
 * Keeps the log-line extraction, discriminator matching, RPC retry,
 * and pretty-print logic as thin wrappers.
 *
 * Supports two emission modes:
 *   - Legacy: `sol_log_data` → `"Program data: <base64>"` log lines.
 *     Payload: event_discriminator(8) + LE-packed fields.
 *   - Self-CPI: inner instruction to the program's own `EmitEvent` no-op.
 *     CPI data on wire: [255] + EVENT_IX_TAG_LE(8) + event_discriminator(8) + fields.
 *
 * `parseTransactionEvents` tries inner-instruction parsing first (new format),
 * then falls back to log-line parsing (old format) for backward compatibility.
 *
 * Discriminator = SHA256("event:<EventName>")[0..8] (Anchor convention).
 */

import {
  type Address,
  type Rpc,
  type SolanaRpcApi,
  type FixedSizeDecoder,
} from "@solana/kit";

import { ROLE_NAMES } from "./shared.js";

// ─── Self-CPI event constants ────────────────────────────────────────────────

/** EmitEvent instruction discriminator (first byte of CPI data). */
const EMIT_EVENT_DISCRIMINATOR = 0xff;

/** SHA256("anchor:event")[0..8] — little-endian tag prepended to CPI event data. */
const EVENT_IX_TAG_LE = new Uint8Array([
  0x1d, 0x9a, 0xcb, 0x51, 0x2e, 0xa5, 0x45, 0xe4,
]);

// ── Permission Manager event decoders ─────────────────────────
import {
  getPermissionManagerInitializedDecoder,
  type PermissionManagerInitialized,
} from "../../clients/ts/permission-manager/types/permissionManagerInitialized.js";
import {
  getRoleGrantedDecoder,
  type RoleGranted,
} from "../../clients/ts/permission-manager/types/roleGranted.js";
import {
  getRoleRemovedDecoder,
  type RoleRemoved,
} from "../../clients/ts/permission-manager/types/roleRemoved.js";
import {
  getOwnershipTransferStartedDecoder,
  type OwnershipTransferStarted,
} from "../../clients/ts/permission-manager/types/ownershipTransferStarted.js";
import {
  getOwnershipTransferredDecoder,
  type OwnershipTransferred,
} from "../../clients/ts/permission-manager/types/ownershipTransferred.js";

// ── Spiko Token event decoders ────────────────────────────────
import {
  getTokenInitializedDecoder,
  type TokenInitialized,
} from "../../clients/ts/spiko-token/types/tokenInitialized.js";
import {
  getMintDecoder,
  type Mint,
} from "../../clients/ts/spiko-token/types/mint.js";
import {
  getBurnDecoder,
  type Burn,
} from "../../clients/ts/spiko-token/types/burn.js";
import {
  getRedeemInitiatedDecoder,
  type RedeemInitiated,
} from "../../clients/ts/spiko-token/types/redeemInitiated.js";
import {
  getTokenPausedDecoder,
  type TokenPaused,
} from "../../clients/ts/spiko-token/types/tokenPaused.js";
import {
  getTokenUnpausedDecoder,
  type TokenUnpaused,
} from "../../clients/ts/spiko-token/types/tokenUnpaused.js";
import {
  getRedemptionContractSetDecoder,
  type RedemptionContractSet,
} from "../../clients/ts/spiko-token/types/redemptionContractSet.js";

// ── Transfer Hook event decoders ──────────────────────────────
import {
  getTransferDecoder,
  type Transfer,
} from "../../clients/ts/spiko-transfer-hook/types/transfer.js";

// ── Minter event decoders ─────────────────────────────────────
import {
  getMinterInitializedDecoder,
  type MinterInitialized,
} from "../../clients/ts/minter/types/minterInitialized.js";
import {
  getMintExecutedDecoder,
  type MintExecuted,
} from "../../clients/ts/minter/types/mintExecuted.js";
import {
  getMintBlockedDecoder,
  type MintBlocked,
} from "../../clients/ts/minter/types/mintBlocked.js";
import {
  getMintApprovedDecoder,
  type MintApproved,
} from "../../clients/ts/minter/types/mintApproved.js";
import {
  getMintCanceledDecoder,
  type MintCanceled,
} from "../../clients/ts/minter/types/mintCanceled.js";
import {
  getDailyLimitUpdatedDecoder,
  type DailyLimitUpdated,
} from "../../clients/ts/minter/types/dailyLimitUpdated.js";
import {
  getMaxDelayUpdatedDecoder,
  type MaxDelayUpdated,
} from "../../clients/ts/minter/types/maxDelayUpdated.js";

// ── Redemption event decoders ─────────────────────────────────
import {
  getRedemptionInitializedDecoder,
  type RedemptionInitialized,
} from "../../clients/ts/redemption/types/redemptionInitialized.js";
import {
  getRedemptionInitiatedDecoder,
  type RedemptionInitiated,
} from "../../clients/ts/redemption/types/redemptionInitiated.js";
import {
  getRedemptionExecutedDecoder,
  type RedemptionExecuted,
} from "../../clients/ts/redemption/types/redemptionExecuted.js";
import {
  getRedemptionCanceledDecoder,
  type RedemptionCanceled,
} from "../../clients/ts/redemption/types/redemptionCanceled.js";
import {
  getTokenMinimumUpdatedDecoder,
  type TokenMinimumUpdated,
} from "../../clients/ts/redemption/types/tokenMinimumUpdated.js";

// =================================================================
// Public types
// =================================================================

export interface DecodedEvent {
  name: string;
  program: string;
  fields: Record<string, string | bigint | number>;
}

// =================================================================
// Event registry: discriminator -> name + program + decoder
// =================================================================

interface EventEntry {
  name: string;
  program: string;
  disc: Uint8Array;
  decoder: FixedSizeDecoder<Record<string, any>>;
}

/**
 * All 25 events with their 8-byte SHA256("event:<Name>")[0..8] discriminators
 * and Codama-generated decoders.
 */
const EVENT_ENTRIES: EventEntry[] = [
  // ── Permission Manager (5) ──
  {
    name: "PermissionManagerInitialized",
    program: "PermissionManager",
    disc: new Uint8Array([0xcf, 0x1e, 0x60, 0x38, 0xfd, 0xa9, 0xc5, 0x0f]),
    decoder: getPermissionManagerInitializedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "RoleGranted",
    program: "PermissionManager",
    disc: new Uint8Array([0xdc, 0xb7, 0x59, 0xe4, 0x8f, 0x3f, 0xf6, 0x3a]),
    decoder: getRoleGrantedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "RoleRemoved",
    program: "PermissionManager",
    disc: new Uint8Array([0x85, 0x23, 0xd6, 0xea, 0xcb, 0x9d, 0xcb, 0x35]),
    decoder: getRoleRemovedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "OwnershipTransferStarted",
    program: "PermissionManager",
    disc: new Uint8Array([0xb7, 0xfd, 0xef, 0xf6, 0x8c, 0xb3, 0x85, 0x69]),
    decoder: getOwnershipTransferStartedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "OwnershipTransferred",
    program: "PermissionManager",
    disc: new Uint8Array([0xac, 0x3d, 0xcd, 0xb7, 0xfa, 0x32, 0x26, 0x62]),
    decoder: getOwnershipTransferredDecoder() as FixedSizeDecoder<Record<string, any>>,
  },

  // ── Spiko Token (7) ──
  {
    name: "TokenInitialized",
    program: "SpikoToken",
    disc: new Uint8Array([0x4d, 0x46, 0xe9, 0x7c, 0xec, 0x5c, 0xcc, 0x00]),
    decoder: getTokenInitializedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "Mint",
    program: "SpikoToken",
    disc: new Uint8Array([0x3f, 0x0b, 0xd5, 0x86, 0x94, 0xc2, 0x18, 0xcb]),
    decoder: getMintDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "Burn",
    program: "SpikoToken",
    disc: new Uint8Array([0xb8, 0x0d, 0x41, 0xce, 0xce, 0xaa, 0x33, 0x55]),
    decoder: getBurnDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "RedeemInitiated",
    program: "SpikoToken",
    disc: new Uint8Array([0x47, 0xdc, 0x92, 0xb9, 0x0b, 0xdc, 0xf5, 0x13]),
    decoder: getRedeemInitiatedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "TokenPaused",
    program: "SpikoToken",
    disc: new Uint8Array([0x7e, 0x36, 0x4c, 0xa1, 0x7d, 0x97, 0x94, 0x3b]),
    decoder: getTokenPausedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "TokenUnpaused",
    program: "SpikoToken",
    disc: new Uint8Array([0xe1, 0x11, 0x44, 0x51, 0x81, 0x86, 0x91, 0xa9]),
    decoder: getTokenUnpausedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "RedemptionContractSet",
    program: "SpikoToken",
    disc: new Uint8Array([0xbd, 0xb3, 0x1c, 0x22, 0xe3, 0x63, 0xf6, 0x3a]),
    decoder: getRedemptionContractSetDecoder() as FixedSizeDecoder<Record<string, any>>,
  },

  // ── Transfer Hook (1) ──
  {
    name: "Transfer",
    program: "TransferHook",
    disc: new Uint8Array([0x19, 0x12, 0x17, 0x07, 0xac, 0x74, 0x82, 0x1c]),
    decoder: getTransferDecoder() as FixedSizeDecoder<Record<string, any>>,
  },

  // ── Minter (7) ──
  {
    name: "MinterInitialized",
    program: "Minter",
    disc: new Uint8Array([0xb1, 0x89, 0x62, 0xb3, 0x16, 0xce, 0x37, 0xc0]),
    decoder: getMinterInitializedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "MintExecuted",
    program: "Minter",
    disc: new Uint8Array([0x37, 0x87, 0x6c, 0x49, 0x05, 0xbe, 0xed, 0x2c]),
    decoder: getMintExecutedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "MintBlocked",
    program: "Minter",
    disc: new Uint8Array([0x7e, 0xee, 0x83, 0xcd, 0xfd, 0x6e, 0xf5, 0x23]),
    decoder: getMintBlockedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "MintApproved",
    program: "Minter",
    disc: new Uint8Array([0x02, 0x44, 0xe9, 0x18, 0x66, 0x41, 0x68, 0x23]),
    decoder: getMintApprovedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "MintCanceled",
    program: "Minter",
    disc: new Uint8Array([0xa8, 0x4a, 0x13, 0x9d, 0x4a, 0xdd, 0xc0, 0x19]),
    decoder: getMintCanceledDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "DailyLimitUpdated",
    program: "Minter",
    disc: new Uint8Array([0x41, 0x08, 0xe7, 0xad, 0xd7, 0xb6, 0x47, 0xc9]),
    decoder: getDailyLimitUpdatedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "MaxDelayUpdated",
    program: "Minter",
    disc: new Uint8Array([0x81, 0x51, 0x91, 0x1a, 0x62, 0xd2, 0xa0, 0x0c]),
    decoder: getMaxDelayUpdatedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },

  // ── Redemption (5) ──
  {
    name: "RedemptionInitialized",
    program: "Redemption",
    disc: new Uint8Array([0x6a, 0xc8, 0x64, 0x72, 0x94, 0x64, 0x26, 0xcb]),
    decoder: getRedemptionInitializedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "RedemptionInitiated",
    program: "Redemption",
    disc: new Uint8Array([0x55, 0xfe, 0xeb, 0x0e, 0xdd, 0x88, 0x60, 0xde]),
    decoder: getRedemptionInitiatedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "RedemptionExecuted",
    program: "Redemption",
    disc: new Uint8Array([0xae, 0xda, 0x05, 0x38, 0x24, 0x2e, 0x35, 0xd4]),
    decoder: getRedemptionExecutedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "RedemptionCanceled",
    program: "Redemption",
    disc: new Uint8Array([0xbd, 0xf4, 0xd0, 0xe8, 0x3c, 0x68, 0xe7, 0xa4]),
    decoder: getRedemptionCanceledDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
  {
    name: "TokenMinimumUpdated",
    program: "Redemption",
    disc: new Uint8Array([0xeb, 0x3c, 0x99, 0x47, 0x61, 0xd4, 0x70, 0x6e]),
    decoder: getTokenMinimumUpdatedDecoder() as FixedSizeDecoder<Record<string, any>>,
  },
];

// =================================================================
// Discriminator lookup map
// =================================================================

function discKey(disc: Uint8Array): string {
  return Array.from(disc.slice(0, 8))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

const DISC_MAP = new Map<string, EventEntry>();
for (const entry of EVENT_ENTRIES) {
  DISC_MAP.set(discKey(entry.disc), entry);
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
  const entry = DISC_MAP.get(key);
  if (!entry) return null;

  // Strip 8-byte discriminator, decode struct fields with Codama decoder
  const body = data.slice(8);
  const decoded = entry.decoder.decode(body);

  // Flatten to Record<string, string | bigint | number>
  const fields: Record<string, string | bigint | number> = {};
  for (const [k, v] of Object.entries(decoded)) {
    fields[k] = v as string | bigint | number;
  }

  return { name: entry.name, program: entry.program, fields };
}

/**
 * Extract and decode all Spiko events from transaction log messages (legacy mode).
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
 * Check whether raw CPI data bytes match the self-CPI event envelope:
 *   [255] + EVENT_IX_TAG_LE(8) + event_disc(8) + fields
 *
 * Returns the event payload starting at the event discriminator (offset 9),
 * or null if the data does not match.
 */
function extractCpiEventPayload(data: Uint8Array): Uint8Array | null {
  // Minimum: 1 (emit disc) + 8 (tag) + 8 (event disc) = 17 bytes
  if (data.length < 17) return null;
  if (data[0] !== EMIT_EVENT_DISCRIMINATOR) return null;

  // Verify EVENT_IX_TAG_LE at bytes 1..9
  for (let i = 0; i < 8; i++) {
    if (data[1 + i] !== EVENT_IX_TAG_LE[i]) return null;
  }

  // Return bytes from offset 9 onward (event_disc + fields)
  return data.slice(9);
}

/**
 * Extract and decode all Spiko events from inner instructions (self-CPI mode).
 *
 * Each event is emitted as a CPI call to the program's own `EmitEvent`
 * instruction. The CPI data layout on the wire is:
 *   [255] + EVENT_IX_TAG_LE(8) + event_discriminator(8) + LE-packed fields
 */
export function decodeEventsFromCpiInstructions(
  innerInstructions: Array<{
    index: number;
    instructions: Array<{
      programIdIndex: number;
      data: string; // base58-encoded
      accounts: number[];
    }>;
  }>,
): DecodedEvent[] {
  const events: DecodedEvent[] = [];

  for (const group of innerInstructions) {
    for (const ix of group.instructions) {
      let raw: Uint8Array;
      try {
        raw = decodeBase58(ix.data);
      } catch {
        continue;
      }

      const payload = extractCpiEventPayload(raw);
      if (!payload) continue;

      const event = decodeEvent(payload);
      if (event) {
        events.push(event);
      }
    }
  }

  return events;
}

// ── base58 decoder ───────────────────────────────────────────────────────────

const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

function decodeBase58(str: string): Uint8Array {
  const bytes: number[] = [];
  for (const c of str) {
    const idx = BASE58_ALPHABET.indexOf(c);
    if (idx < 0) throw new Error(`Invalid base58 character: ${c}`);
    let carry = idx;
    for (let j = 0; j < bytes.length; j++) {
      carry += bytes[j] * 58;
      bytes[j] = carry & 0xff;
      carry >>= 8;
    }
    while (carry > 0) {
      bytes.push(carry & 0xff);
      carry >>= 8;
    }
  }
  // Leading '1's become leading zeros
  for (const c of str) {
    if (c !== "1") break;
    bytes.push(0);
  }
  return new Uint8Array(bytes.reverse());
}

/**
 * Fetch a confirmed transaction by signature and decode its events.
 *
 * Tries self-CPI inner instruction parsing first (new format), then falls
 * back to log-line parsing (legacy format) for backward compatibility.
 */
export async function parseTransactionEvents(
  rpc: Rpc<SolanaRpcApi>,
  signature: string,
  maxRetries = 5,
  retryDelayMs = 2000
): Promise<DecodedEvent[]> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    const tx = await rpc
      .getTransaction(signature as any, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
        encoding: "json",
      })
      .send();

    if (tx?.meta) {
      // Try inner instructions first (self-CPI events)
      const innerIxs = (tx.meta as any).innerInstructions;
      if (innerIxs && Array.isArray(innerIxs) && innerIxs.length > 0) {
        const events = decodeEventsFromCpiInstructions(innerIxs);
        if (events.length > 0) return events;
      }

      // Fall back to log-line parsing (legacy sol_log_data events)
      if (tx.meta.logMessages) {
        const events = decodeEventsFromLogs(tx.meta.logMessages as string[]);
        if (events.length > 0) return events;
      }
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

export function formatEvent(event: DecodedEvent, decimals: number = 5): string {
  const lines: string[] = [];
  lines.push(`  [${event.program}] ${event.name}`);

  for (const [key, value] of Object.entries(event.fields)) {
    let display: string;
    if (typeof value === "bigint") {
      if (key === "amount" || key === "limit" || key === "minimum") {
        const shares = Number(value) / 10 ** decimals;
        display = `${value} (${shares} shares)`;
      } else if (key === "salt") {
        display = `${value}`;
      } else if (key === "maxDelay" || key === "deadline") {
        display = `${value}`;
        if (key === "deadline") {
          display += ` (${new Date(Number(value) * 1000).toISOString()})`;
        }
      } else {
        display = `${value}`;
      }
    } else if (typeof value === "number") {
      if (key === "roleId") {
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

export function formatEvents(events: DecodedEvent[]): string {
  if (events.length === 0) return "  (no events decoded)";
  return events.map(formatEvent).join("\n\n");
}
