/**
 * Generates TypeScript clients from the Codama IDL files for all Spiko programs.
 */

import { renderVisitor as renderJavaScriptVisitor } from '@codama/renderers-js';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

import { createSpikoCodamaBuilder } from './lib/spiko-codama-builder.js';
import { preserveConfigFiles } from './lib/utils.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectRoot = path.join(__dirname, '..');
const idlDir = path.join(projectRoot, 'idl');
const clientsDir = path.join(projectRoot, 'clients', 'ts');

/** Program IDL configs: [idl filename, client package directory name] */
const programs: [string, string][] = [
    ['permission_manager.json', 'permission-manager'],
    ['spiko_token.json', 'spiko-token'],
    ['minter.json', 'minter'],
    ['redemption.json', 'redemption'],
    ['spiko_transfer_hook.json', 'spiko-transfer-hook'],
    ['custodial_gatekeeper.json', 'custodial-gatekeeper'],
];

for (const [idlFile, clientDir] of programs) {
    const idlPath = path.join(idlDir, idlFile);

    if (!fs.existsSync(idlPath)) {
        console.warn(`Warning: IDL not found at ${idlPath}, skipping ${clientDir}`);
        continue;
    }

    console.log(`Generating TypeScript client for ${clientDir}...`);

    const idl = JSON.parse(fs.readFileSync(idlPath, 'utf-8'));
    const codama = createSpikoCodamaBuilder(idl).appendAccountVersion().build();

    const tsClientDir = path.join(clientsDir, clientDir);
    const generatedDir = path.join(tsClientDir, 'src', 'generated');

    // Ensure output directory exists
    fs.mkdirSync(generatedDir, { recursive: true });

    // Preserve config files during generation
    const configPreserver = preserveConfigFiles(tsClientDir);

    // Generate TypeScript client
    codama.accept(
        renderJavaScriptVisitor(generatedDir, {
            deleteFolderBeforeRendering: true,
            formatCode: true,
        }),
    );

    // Restore config files
    configPreserver.restore();

    console.log(`  -> ${generatedDir}`);
}

console.log('Done generating all TypeScript clients.');
