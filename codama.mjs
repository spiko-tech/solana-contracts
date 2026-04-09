/**
 * Codama code generation script.
 *
 * Reads Anchor v01-style IDLs from idl/ and generates Kit-friendly
 * TypeScript clients for each program into clients/ts/<program>/.
 *
 * Usage:
 *   node codama.mjs
 */

import { createFromRoot } from "codama";
import { rootNodeFromAnchor } from "@codama/nodes-from-anchor";
import { renderVisitor } from "@codama/renderers-js";
import { readFileSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

const programs = [
  "permission_manager",
  "spiko_token",
  "minter",
  "redemption",
  "spiko_transfer_hook",
];

for (const name of programs) {
  const idlPath = join(__dirname, "idl", `${name}.json`);
  const idl = JSON.parse(readFileSync(idlPath, "utf-8"));

  console.log(`Processing ${name}...`);

  // Convert Anchor v01 IDL to Codama root node
  const rootNode = rootNodeFromAnchor(idl);
  const codama = createFromRoot(rootNode);

  // Determine output directory
  const outDir = join(__dirname, "clients", "ts", name.replace(/_/g, "-"));

  // Render Kit-friendly TypeScript client
  codama.accept(
    renderVisitor(outDir, {
      dependencyMap: {
        // Map well-known program addresses to npm packages
        "11111111111111111111111111111111": "@solana-program/system",
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb": "@solana-program/token-2022",
      },
    })
  );

  console.log(`  -> ${outDir}`);
}

console.log("\nDone! Generated clients for all programs.");
