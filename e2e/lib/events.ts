/**
 * Event decoder for Spiko program events (v2).
 *
 * Uses Codama-generated FixedSizeDecoders for struct parsing.
 * Keeps the log-line extraction, discriminator matching, RPC retry,
 * and pretty-print logic as thin wrappers.
 *
 * Supports self-CPI event emission:
 *   Inner instruction to the program's own `EmitEvent` no-op.
 *   CPI data on wire: [255] + EVENT_IX_TAG_LE(8) + event_discriminator(1) + LE-packed fields.
 *
 * `parseTransactionEvents` decodes events from inner instructions, using the
 * program ID from the CPI target to disambiguate 1-byte discriminators.
 *
 * Discriminators are 1-byte sequential enums per program, defined in each
 * program's `discriminators/event.rs`.
 */

import {
  type Address,
  type Rpc,
  type SolanaRpcApi,
  type FixedSizeDecoder,
} from "@solana/kit";

import { ROLE_NAMES } from "./shared.js";

const EMIT_EVENT_DISCRIMINATOR = 0xff;

/** SHA256("anchor:event")[0..8] — little-endian tag prepended to CPI event data. */
const EVENT_IX_TAG_LE = new Uint8Array([
  0x1d, 0x9a, 0xcb, 0x51, 0x2e, 0xa5, 0x45, 0xe4,
]);

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

import {
  getTransferEventDecoder,
} from "../../clients/ts/spiko-transfer-hook/src/generated/types/transferEvent.js";

import {
  getMinterInitializedEventDecoder,
} from "../../clients/ts/minter/src/generated/types/minterInitializedEvent.js";
import {
  getMintInitiatedEventDecoder,
} from "../../clients/ts/minter/src/generated/types/mintInitiatedEvent.js";
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

import {
  getGatekeeperInitializedEventDecoder,
} from "../../clients/ts/custodial-gatekeeper/src/generated/types/gatekeeperInitializedEvent.js";
import {
  getWithdrawalInitiatedEventDecoder,
} from "../../clients/ts/custodial-gatekeeper/src/generated/types/withdrawalInitiatedEvent.js";
import {
  getWithdrawalApprovedEventDecoder,
} from "../../clients/ts/custodial-gatekeeper/src/generated/types/withdrawalApprovedEvent.js";
import {
  getWithdrawalCanceledEventDecoder,
} from "../../clients/ts/custodial-gatekeeper/src/generated/types/withdrawalCanceledEvent.js";
import {
  getWithdrawalBlockedEventDecoder,
} from "../../clients/ts/custodial-gatekeeper/src/generated/types/withdrawalBlockedEvent.js";
import {
  getDailyLimitUpdatedEventDecoder as getGkDailyLimitUpdatedEventDecoder,
} from "../../clients/ts/custodial-gatekeeper/src/generated/types/dailyLimitUpdatedEvent.js";

import { PERMISSION_MANAGER_PROGRAM_ADDRESS } from "../../clients/ts/permission-manager/src/generated/programs/index.js";
import { SPIKO_TOKEN_PROGRAM_ADDRESS } from "../../clients/ts/spiko-token/src/generated/programs/index.js";
import { MINTER_PROGRAM_ADDRESS } from "../../clients/ts/minter/src/generated/programs/index.js";
import { REDEMPTION_PROGRAM_ADDRESS } from "../../clients/ts/redemption/src/generated/programs/index.js";
import { SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS } from "../../clients/ts/spiko-transfer-hook/src/generated/programs/index.js";
import { CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS } from "../../clients/ts/custodial-gatekeeper/src/generated/programs/index.js";

export interface DecodedEvent {
  name: string;
  program: string;
  fields: Record<string, string | bigint | number>;
}

interface EventEntry {
  name: string;
  program: string;
  decoder: FixedSizeDecoder<Record<string, any>>;
}

/**
 * Per-program event discriminator maps.
 * Keys are the 1-byte discriminator value from the enum.
 */
const PROGRAM_EVENT_MAPS = new Map<string, Map<number, EventEntry>>();

function registerEvents(programAddress: Address, entries: Array<{ disc: number } & EventEntry>) {
  const map = new Map<number, EventEntry>();
  for (const { disc, ...entry } of entries) {
    map.set(disc, entry);
  }
  PROGRAM_EVENT_MAPS.set(programAddress as string, map);
}

registerEvents(PERMISSION_MANAGER_PROGRAM_ADDRESS, [
  { disc: 0, name: "PermissionManagerInitialized", program: "PermissionManager", decoder: getPermissionManagerInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 1, name: "RoleGranted",                  program: "PermissionManager", decoder: getRoleGrantedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 2, name: "RoleRemoved",                  program: "PermissionManager", decoder: getRoleRemovedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 3, name: "OwnershipTransferStarted",     program: "PermissionManager", decoder: getOwnershipTransferStartedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 4, name: "OwnershipTransferred",         program: "PermissionManager", decoder: getOwnershipTransferredEventDecoder() as FixedSizeDecoder<Record<string, any>> },
]);

registerEvents(SPIKO_TOKEN_PROGRAM_ADDRESS, [
  { disc: 0, name: "TokenInitialized",       program: "SpikoToken", decoder: getTokenInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 1, name: "Mint",                   program: "SpikoToken", decoder: getMintEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 2, name: "Burn",                   program: "SpikoToken", decoder: getBurnEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 3, name: "RedeemInitiated",        program: "SpikoToken", decoder: getRedeemInitiatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 4, name: "TokenPaused",            program: "SpikoToken", decoder: getTokenPausedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 5, name: "TokenUnpaused",          program: "SpikoToken", decoder: getTokenUnpausedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 6, name: "RedemptionContractSet",  program: "SpikoToken", decoder: getRedemptionContractSetEventDecoder() as FixedSizeDecoder<Record<string, any>> },
]);

registerEvents(SPIKO_TRANSFER_HOOK_PROGRAM_ADDRESS, [
  { disc: 0, name: "Transfer", program: "TransferHook", decoder: getTransferEventDecoder() as FixedSizeDecoder<Record<string, any>> },
]);

registerEvents(MINTER_PROGRAM_ADDRESS, [
  { disc: 0, name: "MinterInitialized",  program: "Minter", decoder: getMinterInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 1, name: "MintInitiated",      program: "Minter", decoder: getMintInitiatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 2, name: "MintApproved",       program: "Minter", decoder: getMintApprovedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 3, name: "MintCanceled",       program: "Minter", decoder: getMintCanceledEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 4, name: "MintBlocked",        program: "Minter", decoder: getMintBlockedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 5, name: "DailyLimitUpdated",  program: "Minter", decoder: getDailyLimitUpdatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 6, name: "MaxDelayUpdated",    program: "Minter", decoder: getMaxDelayUpdatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
]);

registerEvents(REDEMPTION_PROGRAM_ADDRESS, [
  { disc: 0, name: "RedemptionInitialized",  program: "Redemption", decoder: getRedemptionInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 1, name: "RedemptionInitiated",    program: "Redemption", decoder: getRedemptionInitiatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 2, name: "RedemptionExecuted",     program: "Redemption", decoder: getRedemptionExecutedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 3, name: "RedemptionCanceled",     program: "Redemption", decoder: getRedemptionCanceledEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 4, name: "TokenMinimumUpdated",    program: "Redemption", decoder: getTokenMinimumUpdatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
]);

registerEvents(CUSTODIAL_GATEKEEPER_PROGRAM_ADDRESS, [
  { disc: 0, name: "GatekeeperInitialized", program: "CustodialGatekeeper", decoder: getGatekeeperInitializedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 1, name: "WithdrawalInitiated",   program: "CustodialGatekeeper", decoder: getWithdrawalInitiatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 2, name: "WithdrawalApproved",    program: "CustodialGatekeeper", decoder: getWithdrawalApprovedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 3, name: "WithdrawalCanceled",    program: "CustodialGatekeeper", decoder: getWithdrawalCanceledEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 4, name: "WithdrawalBlocked",     program: "CustodialGatekeeper", decoder: getWithdrawalBlockedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
  { disc: 5, name: "DailyLimitUpdated",     program: "CustodialGatekeeper", decoder: getGkDailyLimitUpdatedEventDecoder() as FixedSizeDecoder<Record<string, any>> },
]);

/**
 * Decode a single event from raw bytes, given the program that emitted it.
 * The payload format is: [1-byte discriminator] + [LE-packed fields].
 * Returns null if the discriminator is not recognized for the given program.
 */
export function decodeEvent(data: Uint8Array, programAddress: string): DecodedEvent | null {
  if (data.length < 1) return null;

  const disc = data[0];
  const programMap = PROGRAM_EVENT_MAPS.get(programAddress);
  if (!programMap) return null;

  const entry = programMap.get(disc);
  if (!entry) return null;

  const body = data.slice(1);
  const decoded = entry.decoder.decode(body);

  const fields: Record<string, string | bigint | number> = {};
  for (const [k, v] of Object.entries(decoded)) {
    fields[k] = v as string | bigint | number;
  }

  return { name: entry.name, program: entry.program, fields };
}

/**
 * Check whether raw CPI data bytes match the self-CPI event envelope:
 *   [255] + EVENT_IX_TAG_LE(8) + event_disc(1) + fields
 *
 * Returns the event payload starting at the event discriminator (offset 9),
 * or null if the data does not match.
 */
function extractCpiEventPayload(data: Uint8Array): Uint8Array | null {
  // Minimum: 1 (emit disc) + 8 (tag) + 1 (event disc) = 10 bytes
  if (data.length < 10) return null;
  if (data[0] !== EMIT_EVENT_DISCRIMINATOR) return null;

  for (let i = 0; i < 8; i++) {
    if (data[1 + i] !== EVENT_IX_TAG_LE[i]) return null;
  }

  return data.slice(9);
}

/**
 * Extract and decode all Spiko events from inner instructions (self-CPI mode).
 *
 * Each event is emitted as a CPI call to the program's own `EmitEvent`
 * instruction. The CPI data layout on the wire is:
 *   [255] + EVENT_IX_TAG_LE(8) + event_discriminator(1) + LE-packed fields
 *
 * The `programIdIndex` in each inner instruction identifies the target program
 * via the transaction's `accountKeys`, enabling disambiguation of 1-byte
 * discriminators across programs.
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
  accountKeys: string[],
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

      const programAddress = accountKeys[ix.programIdIndex];
      if (!programAddress) continue;

      const event = decodeEvent(payload, programAddress);
      if (event) {
        events.push(event);
      }
    }
  }

  return events;
}

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
 * Uses self-CPI inner instruction parsing with program ID context
 * for 1-byte discriminator resolution.
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
      const message = (tx as any).transaction?.message;
      const accountKeys: string[] = message?.accountKeys ?? [];

      const innerIxs = (tx.meta as any).innerInstructions;
      if (innerIxs && Array.isArray(innerIxs) && innerIxs.length > 0) {
        const events = decodeEventsFromCpiInstructions(innerIxs, accountKeys);
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
