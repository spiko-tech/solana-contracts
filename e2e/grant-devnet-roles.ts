/**
 * Grant roles to the Spiko devnet wallets.
 *
 * Prerequisites:
 *   - Permission manager program deployed and initialized on devnet
 *   - Solana CLI configured for devnet with the admin keypair
 *
 * Usage:
 *   cd e2e && npx tsx grant-devnet-roles.ts
 */

import { type Address, address } from "@solana/kit";

import { getGrantRoleInstructionAsync } from "../clients/ts/permission-manager/src/generated/instructions/grantRole.js";

import {
  setup,
  sendTx,
  ROLE_MINTER,
  ROLE_WHITELISTER,
  ROLE_REDEMPTION_EXECUTOR,
  ROLE_NAMES,
} from "./lib/shared.js";

const ROLE_ASSIGNMENTS: { target: Address; roleId: number }[] = [
  {
    target: address("5kx1nLkKyqG2UyAMtb5yhWVurZ9mqUnUabrGYdtkZoNM"),
    roleId: ROLE_MINTER,
  },
  {
    target: address("6ZfG1QWrKxDPFkAG8Xo41QPtQCPGNFHf76wrffd6zPmb"),
    roleId: ROLE_REDEMPTION_EXECUTOR,
  },
  {
    target: address("4qNAY287J2HdArf1Hc6w6qN76SAdHCsrumZSeZutG2mN"),
    roleId: ROLE_WHITELISTER,
  },
];

async function main() {
  const { rpc, rpcSub, admin } = await setup();

  console.log("Granting devnet roles...\n");

  for (const { target, roleId } of ROLE_ASSIGNMENTS) {
    const label = `Grant ${ROLE_NAMES[roleId]} to ${target.slice(0, 8)}...`;
    const ix = await getGrantRoleInstructionAsync({
      admin,
      user: target,
      role: roleId,
    });
    await sendTx(rpc, rpcSub, admin, [ix], label);
  }

  console.log("\nAll roles granted.");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
