import { type Codama, createFromJson } from 'codama';
import { appendAccountVersion } from './updates/index.js';

/**
 * Builder for applying Codama IDL transformations before client generation.
 */
export class SpikoCodamaBuilder {
    private codama: Codama;

    constructor(idl: any) {
        const idlJson = typeof idl === 'string' ? idl : JSON.stringify(idl);
        this.codama = createFromJson(idlJson);
    }

    appendAccountVersion(): this {
        this.codama = appendAccountVersion(this.codama);
        return this;
    }

    build(): Codama {
        return this.codama;
    }
}

export function createSpikoCodamaBuilder(idl: any): SpikoCodamaBuilder {
    return new SpikoCodamaBuilder(idl);
}
