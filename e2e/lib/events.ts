/**
 * Event decoder for Spiko program events (v2).
 *
 * Uses Codama-generated FixedSizeDecoders for struct parsing.
 * Keeps the log-line extraction, discriminator matching, RPC retry,
 * and pretty-print logic as thin wrappers.
 *
 * Supports two emission modes:
 *   - Legacy: `sol_log_data` -> `"Program data: <base64>"` log lines.
 *     Payload: event_discriminator(8) + LE-packed fields.
 *   - Self-CPI: inner instruction to the program's own `EmitEvent` no-op.
 *     CPI data on wire: [255] + EVENT_IX_TAG_LE(8) + event_discriminator(8) + fields.
 *
 * `parseTransactionEvents` tries inner-instruction parsing first (new format),
 * then falls back to log-line parsing (old format) for backward compatibility.
 *
 * Discriminator = SHA256("event:<EventName>")[0..8] (Anchor convention),
 * computed dynamically at module load time using Node.js crypto.
 */

import nodeCrypto from "node:crypto";

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
  getPermissionManagerInitializedEventDecoder,
} from "../../clients/ts/permission-manager/src/generated/types/permissionManagerInitializedEvent.js";
import {
  getRoleGrantedEventDecoder,
} from "../../clients/ts/permission-manager/src/generated/types/roleGrantedEvent.js";
import {
  getRoleRemovedEventDecoder,
} from "../../clients/ts/permission-manager/src/generated/types/roleRemovedEvent.js";
import {
  getOwnershipTransferStartedEventDecoder,
} from "../../clients/ts/permission-manager/src/generated/types/ownershipTransferStartedEvent.js";
import {
  getOwnershipTransferredEventDecoder,
} from "../../clients/ts/permission-manager/src/generated/types/ownershipTransferredEvent.js";

// ── Spiko Token event decoders ────────────────────────────────
import {
  getTokenInitializedEventDecoder,
} from "../../clients/ts/spiko-token/src/generated/types/tokenInitializedEvent.js";
import {
  getMintEventDecoder,
} from "../../clients/ts/spiko-token/src/generated/types/mintEvent.js";
import {
  getBurnEventDecoder,
} from "../../clients/ts/spiko-token/src/generated/types/burnEvent.js";
import {
  getRedeemInitiatedEventDecoder,
} from "../../clients/ts/spiko-token/src/generated/types/redeemInitiatedEvent.js";
import {
  getTokenPausedEventDecoder,
} from "../../clients/ts/spiko-token/src/generated/types/tokenPausedEvent.js";
import {
  getTokenUnpausedEventDecoder,
} from "../../clients/ts/spiko-token/src/generated/types/tokenUnpausedEvent.js";
import {
  getRedemptionContractSetEventDecoder,
} from "../../clients/ts/spiko-token/src/generated/types/redemptionContractSetEvent.js";

// ── Transfer Hook event decoders ──────────────────────────────
import {
  getTransferEventDecoder,
} from "../../clients/ts/spiko-transfer-hook/src/generated/types/transferEvent.js";

// ── Minter event decoders ─────────────────────────────────────
import {
  getMinterInitializedEventDecoder,
} from "../../clients/ts/minter/src/generated/types/minterInitializedEvent.js";
import {
  getMintExecutedEventDecoder,
} from "../../clients/ts/minter/src/generated/types/mintExecutedEvent.js";
import {
  getMintBlockedEventDecoder,
} from "../../clients/ts/minter/src/generated/types/mintBlockedEvent.js";
import {
  getMintApprovedEventDecoder,
} from "../../clients/ts/minter/src/generated/types/mintApprovedEvent.js";
import {
  getMintCanceledEventDecoder,
} from "../../clients/ts/minter/src/generated/types/mintCanceledEvent.js";
import {
  getDailyLimitUpdatedEventDecoder,
} from "../../clients/ts/minter/src/generated/types/dailyLimitUpdatedEvent.js";
import {
  getMaxDelayUpdatedEventDecoder,
} from "../../clients/ts/minter/src/generated/types/maxDelayUpdatedEvent.js";

// ── Redemption event decoders ─────────────────────────────────
import {
  getRedemptionInitializedEventDecoder,
} from "../../clients/ts/redemption/src/generated/types/redemptionInitializedEvent.js";
import {
  getRedemptionInitiatedEventDecoder,
} from "../../clients/ts/redemption/src/generated/types/redemptionInitiatedEvent.js";
import {
  getRedemptionExecutedEventDecoder,
} from "../../clients/ts/redemption/src/generated/types/redemptionExecutedEvent.js";
import {
  getRedemptionCanceledEventDecoder,
} from "../../clients/ts/redemption/src/generated/types/redemptionCanceledEvent.js";
import {
  getTokenMinimumUpdatedEventDecoder,
} from "../../clients/ts/redemption/src/generated/types/tokenMinimumUpdatedEvent.js";

// =================================================================
// Public types
// =================================================================

export interface DecodedEvent {
  name: string;
  program: string;
  fields: Record<string, string | bigint | number>;
}

// =================================================================
// Discriminator computation
// =================================================================

/**
 * Compute event discriminator: SHA256("event:<EventName>")[0..8].
 * Uses Node.js sync crypto — no async needed.
 */
function eventDiscriminator(name: string): Uint8Array {
  const hash = nodeCrypto.createHash("sha256").update(`event:${name}`).digest();
  return new Uint8Array(hash.buffer, hash.byteOffset, 8);
}

// =================================================================
// Event registry: discriminator -> name + program + decoder
// =================================================================

interface EventEntry {
  name: string;
  program: string;
  decoder: FixedSizeDecoder<Record<string, any>>;
}

/**
 * All 25 events with Codama-generated decoders.
 * Discriminators are computed dynamically from event names.
 */
const EVENT_ENTRIES: EventEntry[] = [
  // ── Permission Manager (5) ──
  { name: "PermissionManagerInitialized", program: "PermissionManager", decoder: getPermissionManagerInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "RoleGranted",                  program: "PermissionManager", decoder: getRoleGrantedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "RoleRemoved",                  program: "PermissionManager", decoder: getRoleRemovedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "OwnershipTransferStarted",     program: "PermissionManager", decoder: getOwnershipTransferStartedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "OwnershipTransferred",         program: "PermissionManager", decoder: getOwnershipTransferredEventDecoder() as FixedSizeDecoder<Record<string, any>> },

  // ── Spiko Token (7) ──
  { name: "TokenInitialized",       program: "SpikoToken", decoder: getTokenInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "Mint",                   program: "SpikoToken", decoder: getMintEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "Burn",                   program: "SpikoToken", decoder: getBurnEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "RedeemInitiated",        program: "SpikoToken", decoder: getRedeemInitiatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "TokenPaused",            program: "SpikoToken", decoder: getTokenPausedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "TokenUnpaused",          program: "SpikoToken", decoder: getTokenUnpausedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "RedemptionContractSet",  program: "SpikoToken", decoder: getRedemptionContractSetEventDecoder() as FixedSizeDecoder<Record<string, any>> },

  // ── Transfer Hook (1) ──
  { name: "Transfer", program: "TransferHook", decoder: getTransferEventDecoder() as FixedSizeDecoder<Record<string, any>> },

  // ── Minter (7) ──
  { name: "MinterInitialized",  program: "Minter", decoder: getMinterInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "MintExecuted",       program: "Minter", decoder: getMintExecutedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "MintBlocked",        program: "Minter", decoder: getMintBlockedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "MintApproved",       program: "Minter", decoder: getMintApprovedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "MintCanceled",       program: "Minter", decoder: getMintCanceledEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "DailyLimitUpdated",  program: "Minter", decoder: getDailyLimitUpdatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "MaxDelayUpdated",    program: "Minter", decoder: getMaxDelayUpdatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },

  // ── Redemption (5) ──
  { name: "RedemptionInitialized",  program: "Redemption", decoder: getRedemptionInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "RedemptionInitiated",    program: "Redemption", decoder: getRedemptionInitiatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "RedemptionExecuted",     program: "Redemption", decoder: getRedemptionExecutedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "RedemptionCanceled",     program: "Redemption", decoder: getRedemptionCanceledEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { name: "TokenMinimumUpdated",    program: "Redemption", decoder: getTokenMinimumUpdatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
];

// =================================================================
// Discriminator lookup map (built dynamically from event names)
// =================================================================

function discKey(disc: Uint8Array): string {
  return Array.from(disc.slice(0, 8))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

const DISC_MAP = new Map<string, EventEntry>();
for (const entry of EVENT_ENTRIES) {
  DISC_MAP.set(discKey(eventDiscriminator(entry.name)), entry);
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
