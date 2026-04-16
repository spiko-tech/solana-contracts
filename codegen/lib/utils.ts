import fs from 'fs';
import path from 'path';

interface ConfigPreserver {
    restore: () => void;
}

/**
 * Preserves config files (package.json, tsconfig.json, etc.) during client generation.
 * The Codama renderers use `deleteFolderBeforeRendering: true` which would destroy these.
 */
export function preserveConfigFiles(clientDir: string): ConfigPreserver {
    const filesToPreserve = ['package.json', 'tsconfig.json', '.npmignore'];
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
