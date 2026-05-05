/**
 * Event decoder for Spiko Anchor program events.
 *
 * Anchor `emit!()` events appear as `Program data: <base64>` log lines.
 * Format: 8-byte discriminator (SHA256("event:<StructName>")[0..8]) + Borsh-encoded fields.
 *
 * Uses Codama-generated FixedSizeDecoders for struct parsing after the discriminator.
 */

import {
  type Address,
  type Rpc,
  type SolanaRpcApi,
  type FixedSizeDecoder,
} from "@solana/kit";

import { ROLE_NAMES } from "./shared.js";

// ── Codama-generated event decoders ─────────────────────────

import { getConfigInitializedDecoder } from "../../clients/ts/permission-manager/src/generated/types/configInitialized.js";
import { getRoleGrantedDecoder } from "../../clients/ts/permission-manager/src/generated/types/roleGranted.js";
import { getRoleRevokedDecoder } from "../../clients/ts/permission-manager/src/generated/types/roleRevoked.js";
import { getAdminTransferRequestedDecoder } from "../../clients/ts/permission-manager/src/generated/types/adminTransferRequested.js";
import { getAdminTransferAcceptedDecoder } from "../../clients/ts/permission-manager/src/generated/types/adminTransferAccepted.js";

import { getTokenInitializedDecoder } from "../../clients/ts/spiko-token/src/generated/types/tokenInitialized.js";
import { getMintedDecoder } from "../../clients/ts/spiko-token/src/generated/types/minted.js";
import { getBurnedDecoder } from "../../clients/ts/spiko-token/src/generated/types/burned.js";
import { getPausedDecoder } from "../../clients/ts/spiko-token/src/generated/types/paused.js";
import { getUnpausedDecoder } from "../../clients/ts/spiko-token/src/generated/types/unpaused.js";

import { getHookInitializedDecoder } from "../../clients/ts/spiko-transfer-hook/src/generated/types/hookInitialized.js";
import { getTransferExecutedDecoder } from "../../clients/ts/spiko-transfer-hook/src/generated/types/transferExecuted.js";

import { getMinterInitializedDecoder } from "../../clients/ts/minter/src/generated/types/minterInitialized.js";
import { getMintInitiatedDecoder } from "../../clients/ts/minter/src/generated/types/mintInitiated.js";
import { getMintBlockedDecoder } from "../../clients/ts/minter/src/generated/types/mintBlocked.js";
import { getMintApprovedDecoder } from "../../clients/ts/minter/src/generated/types/mintApproved.js";
import { getMintCanceledDecoder } from "../../clients/ts/minter/src/generated/types/mintCanceled.js";
import { getDailyLimitUpdatedDecoder } from "../../clients/ts/minter/src/generated/types/dailyLimitUpdated.js";

import { getRedemptionInitializedDecoder } from "../../clients/ts/redemption/src/generated/types/redemptionInitialized.js";
import { getVaultCreatedDecoder } from "../../clients/ts/redemption/src/generated/types/vaultCreated.js";
import { getRedemptionInitiatedDecoder } from "../../clients/ts/redemption/src/generated/types/redemptionInitiated.js";
import { getRedemptionExecutedDecoder } from "../../clients/ts/redemption/src/generated/types/redemptionExecuted.js";
import { getRedemptionCanceledDecoder } from "../../clients/ts/redemption/src/generated/types/redemptionCanceled.js";

import { getGatekeeperInitializedDecoder } from "../../clients/ts/custodial-gatekeeper/src/generated/types/gatekeeperInitialized.js";
import { getWithdrawalInitiatedDecoder } from "../../clients/ts/custodial-gatekeeper/src/generated/types/withdrawalInitiated.js";
import { getWithdrawalApprovedDecoder } from "../../clients/ts/custodial-gatekeeper/src/generated/types/withdrawalApproved.js";
import { getWithdrawalCanceledDecoder } from "../../clients/ts/custodial-gatekeeper/src/generated/types/withdrawalCanceled.js";
import { getWithdrawalBlockedDecoder } from "../../clients/ts/custodial-gatekeeper/src/generated/types/withdrawalBlocked.js";
import { getDailyLimitUpdatedDecoder as getGkDailyLimitUpdatedDecoder } from "../../clients/ts/custodial-gatekeeper/src/generated/types/dailyLimitUpdated.js";

export interface DecodedEvent {
  name: string;
  program: string;
  fields: Record<string, string | bigint | number | Uint8Array>;
}

interface EventEntry {
  name: string;
  program: string;
  decoder: FixedSizeDecoder<Record<string, any>>;
}

/**
 * Compute the Anchor event discriminator: SHA256("event:<StructName>")[0..8].
 * Uses synchronous SubtleCrypto workaround via pre-computed values.
 */
async function anchorEventDiscriminator(eventName: string): Promise<Uint8Array> {
  const data = new TextEncoder().encode(`event:${eventName}`);
  const hash = await crypto.subtle.digest("SHA-256", data);
  return new Uint8Array(hash).slice(0, 8);
}

/** Map from 8-byte hex discriminator to EventEntry. */
const EVENT_MAP = new Map<string, EventEntry>();

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, "0")).join("");
}

/**
 * Register all events. Must be called once at startup (async due to SHA256).
 */
async function registerAllEvents(): Promise<void> {
  const entries: Array<{ structName: string } & EventEntry> = [
    // Permission Manager
    { structName: "ConfigInitialized", name: "ConfigInitialized", program: "PermissionManager", decoder: getConfigInitializedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "RoleGranted", name: "RoleGranted", program: "PermissionManager", decoder: getRoleGrantedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "RoleRevoked", name: "RoleRevoked", program: "PermissionManager", decoder: getRoleRevokedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "AdminTransferRequested", name: "AdminTransferRequested", program: "PermissionManager", decoder: getAdminTransferRequestedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "AdminTransferAccepted", name: "AdminTransferAccepted", program: "PermissionManager", decoder: getAdminTransferAcceptedDecoder() as FixedSizeDecoder<Record<string, any>> },

    // Spiko Token
    { structName: "TokenInitialized", name: "TokenInitialized", program: "SpikoToken", decoder: getTokenInitializedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "Minted", name: "Minted", program: "SpikoToken", decoder: getMintedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "Burned", name: "Burned", program: "SpikoToken", decoder: getBurnedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "Paused", name: "Paused", program: "SpikoToken", decoder: getPausedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "Unpaused", name: "Unpaused", program: "SpikoToken", decoder: getUnpausedDecoder() as FixedSizeDecoder<Record<string, any>> },

    // Transfer Hook
    { structName: "HookInitialized", name: "HookInitialized", program: "TransferHook", decoder: getHookInitializedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "TransferExecuted", name: "TransferExecuted", program: "TransferHook", decoder: getTransferExecutedDecoder() as FixedSizeDecoder<Record<string, any>> },

    // Minter
    { structName: "MinterInitialized", name: "MinterInitialized", program: "Minter", decoder: getMinterInitializedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "MintInitiated", name: "MintInitiated", program: "Minter", decoder: getMintInitiatedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "MintBlocked", name: "MintBlocked", program: "Minter", decoder: getMintBlockedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "MintApproved", name: "MintApproved", program: "Minter", decoder: getMintApprovedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "MintCanceled", name: "MintCanceled", program: "Minter", decoder: getMintCanceledDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "DailyLimitUpdated", name: "DailyLimitUpdated", program: "Minter", decoder: getDailyLimitUpdatedDecoder() as FixedSizeDecoder<Record<string, any>> },

    // Redemption
    { structName: "RedemptionInitialized", name: "RedemptionInitialized", program: "Redemption", decoder: getRedemptionInitializedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "VaultCreated", name: "VaultCreated", program: "Redemption", decoder: getVaultCreatedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "RedemptionInitiated", name: "RedemptionInitiated", program: "Redemption", decoder: getRedemptionInitiatedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "RedemptionExecuted", name: "RedemptionExecuted", program: "Redemption", decoder: getRedemptionExecutedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "RedemptionCanceled", name: "RedemptionCanceled", program: "Redemption", decoder: getRedemptionCanceledDecoder() as FixedSizeDecoder<Record<string, any>> },

    // Custodial Gatekeeper
    { structName: "GatekeeperInitialized", name: "GatekeeperInitialized", program: "CustodialGatekeeper", decoder: getGatekeeperInitializedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "WithdrawalInitiated", name: "WithdrawalInitiated", program: "CustodialGatekeeper", decoder: getWithdrawalInitiatedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "WithdrawalApproved", name: "WithdrawalApproved", program: "CustodialGatekeeper", decoder: getWithdrawalApprovedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "WithdrawalCanceled", name: "WithdrawalCanceled", program: "CustodialGatekeeper", decoder: getWithdrawalCanceledDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "WithdrawalBlocked", name: "WithdrawalBlocked", program: "CustodialGatekeeper", decoder: getWithdrawalBlockedDecoder() as FixedSizeDecoder<Record<string, any>> },
    { structName: "DailyLimitUpdated", name: "DailyLimitUpdated", program: "CustodialGatekeeper", decoder: getGkDailyLimitUpdatedDecoder() as FixedSizeDecoder<Record<string, any>> },
  ];

  for (const { structName, ...entry } of entries) {
    const disc = await anchorEventDiscriminator(structName);
    const key = toHex(disc);
    // If there's a collision (same event name in different programs), last wins.
    // For DailyLimitUpdated, both Minter and CG have the same discriminator.
    // We'll handle this by trying both decoders.
    if (EVENT_MAP.has(key)) {
      // Store under a different key to avoid collision
      EVENT_MAP.set(key + "_" + entry.program, entry);
    }
    EVENT_MAP.set(key, entry);
  }
}

let initPromise: Promise<void> | null = null;
async function ensureInitialized(): Promise<void> {
  if (!initPromise) {
    initPromise = registerAllEvents();
  }
  return initPromise;
}

/**
 * Decode a single Anchor event from raw bytes.
 * The payload format is: [8-byte discriminator] + [Borsh-encoded fields].
 */
export function decodeEvent(data: Uint8Array): DecodedEvent | null {
  if (data.length < 8) return null;

  const discHex = toHex(data.slice(0, 8));
  const entry = EVENT_MAP.get(discHex);
  if (!entry) return null;

  const body = data.slice(8);
  try {
    const decoded = entry.decoder.decode(body);
    const fields: Record<string, string | bigint | number | Uint8Array> = {};
    for (const [k, v] of Object.entries(decoded)) {
      fields[k] = v as string | bigint | number | Uint8Array;
    }
    return { name: entry.name, program: entry.program, fields };
  } catch {
    return null;
  }
}

/**
 * Extract and decode all Anchor events from transaction log messages.
 *
 * Anchor `emit!()` events appear as `Program data: <base64>` log lines.
 * The data is base64-encoded: 8-byte discriminator + Borsh fields.
 */
export function decodeEventsFromLogs(logMessages: string[]): DecodedEvent[] {
  const events: DecodedEvent[] = [];

  for (const line of logMessages) {
    const match = line.match(/^Program data: (.+)$/);
    if (!match) continue;

    const base64Data = match[1];
    let raw: Uint8Array;
    try {
      raw = Uint8Array.from(atob(base64Data), c => c.charCodeAt(0));
    } catch {
      continue;
    }

    const event = decodeEvent(raw);
    if (event) {
      events.push(event);
    }
  }

  return events;
}

/**
 * Fetch a confirmed transaction by signature and decode its Anchor events.
 */
export async function parseTransactionEvents(
  rpc: Rpc<SolanaRpcApi>,
  signature: string,
  maxRetries = 5,
  retryDelayMs = 2000
): Promise<DecodedEvent[]> {
  await ensureInitialized();

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    const tx = await rpc
      .getTransaction(signature as any, {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
        encoding: "json",
      })
      .send();

    if (tx?.meta) {
      const logMessages = (tx.meta as any).logMessages;
      if (logMessages && Array.isArray(logMessages) && logMessages.length > 0) {
        const events = decodeEventsFromLogs(logMessages);
        if (events.length > 0) return events;
      }
    }

    if (attempt < maxRetries) {
      await new Promise((r) => setTimeout(r, retryDelayMs));
    }
  }

  return [];
}

export function formatEvent(event: DecodedEvent, decimals: number = 5): string {
  const lines: string[] = [];
  lines.push(`  [${event.program}] ${event.name}`);

  for (const [key, value] of Object.entries(event.fields)) {
    let display: string;
    if (value instanceof Uint8Array) {
      display = toHex(value);
    } else if (typeof value === "bigint") {
      if (key === "amount" || key === "limit" || key === "minimum") {
        const shares = Number(value) / 10 ** decimals;
        display = `${value} (${shares} shares)`;
      } else if (key === "salt") {
        display = `${value}`;
      } else if (key === "maxDelay" || key === "deadline" || key === "deadlineDelay") {
        display = `${value}`;
        if (key === "deadline") {
          display += ` (${new Date(Number(value) * 1000).toISOString()})`;
        }
      } else {
        display = `${value}`;
      }
    } else if (typeof value === "number") {
      if (key === "role") {
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
