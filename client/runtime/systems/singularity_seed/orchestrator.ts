#!/usr/bin/env node
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';

// Layer ownership: apps/examples/singularity-seed-demo/orchestrator.ts (authoritative)
// TypeScript compatibility shim only.
const require = createRequire(import.meta.url);
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const mod = require(path.resolve(__dirname, '../../../../apps/examples/singularity-seed-demo/orchestrator.ts'));
export const runSovereigntyGuardedCycle = mod.runSovereigntyGuardedCycle;
export const DRIFT_THRESHOLD_PCT = mod.DRIFT_THRESHOLD_PCT;
export default mod;
