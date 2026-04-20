import { createFromJson } from "codama";
import { renderVisitor as renderJavaScriptVisitor } from "@codama/renderers-js";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.join(__dirname, "..");
const idlDir = path.join(projectRoot, "idl");
const clientsDir = path.join(projectRoot, "clients", "ts");

const programs: [string, string][] = [
  ["permission_manager.json", "permission-manager"],
  ["spiko_token.json", "spiko-token"],
  ["minter.json", "minter"],
  ["redemption.json", "redemption"],
  ["spiko_transfer_hook.json", "spiko-transfer-hook"],
  ["custodial_gatekeeper.json", "custodial-gatekeeper"],
];

/**
 * Preserves config files (package.json, tsconfig.json, etc.) during client generation.
 * The Codama renderers use `deleteFolderBeforeRendering: true` which would destroy these.
 */
function preserveConfigFiles(clientDir: string) {
  const filesToPreserve = ["package.json", "tsconfig.json", ".npmignore"];
  const preservedFiles = new Map<string, string>();

  for (const filename of filesToPreserve) {
    const filePath = path.join(clientDir, filename);
    const tempPath = path.join(clientDir, `${filename}.temp`);

    if (fs.existsSync(filePath)) {
      fs.copyFileSync(filePath, tempPath);
      preservedFiles.set(filename, tempPath);
    }
  }

  return {
    restore: () => {
      for (const [filename, tempPath] of preservedFiles) {
        try {
          const filePath = path.join(clientDir, filename);
          if (fs.existsSync(tempPath)) {
            fs.copyFileSync(tempPath, filePath);
            fs.unlinkSync(tempPath);
          }
        } catch (error) {
          console.warn(
            `Warning: Failed to restore ${filename}:`,
            (error as Error).message,
          );
        }
      }
    },
  };
}

for (const [idlFile, clientDir] of programs) {
  const idlPath = path.join(idlDir, idlFile);

  if (!fs.existsSync(idlPath)) {
    console.warn(`Warning: IDL not found at ${idlPath}, skipping ${clientDir}`);
    continue;
  }

  console.log(`Generating TypeScript client for ${clientDir}...`);

  const idl = JSON.parse(fs.readFileSync(idlPath, "utf-8"));
  const codama = createFromJson(JSON.stringify(idl));

  const tsClientDir = path.join(clientsDir, clientDir);
  const generatedDir = path.join(tsClientDir, "src", "generated");

  fs.mkdirSync(generatedDir, { recursive: true });

  const configPreserver = preserveConfigFiles(tsClientDir);

  codama.accept(
    renderJavaScriptVisitor(generatedDir, {
      deleteFolderBeforeRendering: true,
      formatCode: true,
    }),
  );

  configPreserver.restore();

  console.log(`  -> ${generatedDir}`);
}

console.log("Done generating all TypeScript clients.");
