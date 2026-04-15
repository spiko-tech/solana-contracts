/**
 * Whitelist a user address by granting ROLE_WHITELISTED.
 *
 * Usage:
 *   npx tsx scripts/whitelist.ts <USER_ADDRESS>
 */

import { address } from "@solana/kit";
import { ROLE_WHITELISTED, ROLE_NAMES } from "./lib/constants.js";
import { permissionConfigPda, userPermissionsPda, permissionManagerEventAuthorityPda } from "./lib/pda.js";
import { grantRole } from "./lib/instructions.js";
import { setup, sendTx } from "./lib/shared.js";

async function main() {
  const userArg = process.argv[2];
  if (!userArg) {
    console.error("Usage: npx tsx scripts/whitelist.ts <USER_ADDRESS>");
    process.exit(1);
  }

  const userAddr = address(userArg);
  console.log(`=== Whitelist User ===\n`);
  console.log(`Target: ${userAddr}\n`);

  const { rpc, rpcSub, admin } = await setup();

  // Derive PDAs
  const [permConfigAddr] = await permissionConfigPda();
  const [adminPermsAddr] = await userPermissionsPda(admin.address);
  const [targetPermsAddr] = await userPermissionsPda(userAddr);
  const [pmEventAuth] = await permissionManagerEventAuthorityPda();

  console.log(`PermissionConfig: ${permConfigAddr}`);
  console.log(`Admin UserPerms:  ${adminPermsAddr}`);
  console.log(`Target UserPerms: ${targetPermsAddr}\n`);

  // Grant ROLE_WHITELISTED
  console.log(`Granting ${ROLE_NAMES[ROLE_WHITELISTED]} (bit ${ROLE_WHITELISTED})...`);

  const ix = grantRole(
    admin,
    permConfigAddr,
    targetPermsAddr,
    userAddr,
    adminPermsAddr,
    ROLE_WHITELISTED,
    pmEventAuth,
  );

  await sendTx(rpc, rpcSub, admin, [ix], `GrantRole(${ROLE_NAMES[ROLE_WHITELISTED]})`);
  console.log("\nDone. User is now whitelisted.");
}

main().catch((err) => {
  console.error("\nFailed:", err);
  process.exit(1);
});
