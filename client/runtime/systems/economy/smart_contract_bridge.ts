#!/usr/bin/env node
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

// Layer ownership: adapters/economy/smart_contract_bridge.ts (authoritative)
// TypeScript compatibility shim only.
const require = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const mod = require(path.resolve(__dirname, '../../../../adapters/economy/smart_contract_bridge.ts'));
export const mintTitheReceipt = mod.mintTitheReceipt;
export default mod;
