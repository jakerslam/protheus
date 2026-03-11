#!/usr/bin/env node
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

// Layer ownership: adapters/importers/generic_json_importer.ts (authoritative)
// TypeScript compatibility shim only.
const require = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const mod = require(path.resolve(__dirname, '../../../../../adapters/importers/generic_json_importer.ts'));
export const engine = mod.engine;
export const importPayload = mod.importPayload;
export default mod;
