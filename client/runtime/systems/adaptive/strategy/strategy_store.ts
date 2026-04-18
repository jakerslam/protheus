#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime (bridge) -> core/layer1/memory_runtime/adaptive (authoritative)

const { createAdaptiveMemoryEntrypoint } = require('../../../lib/adaptive_memory_entrypoint.ts');
const STORE_ID = 'strategy_store';
const MAX_ARGS = 64;
const MAX_ARG_LEN = 512;
const entrypoint = createAdaptiveMemoryEntrypoint(STORE_ID, {
  maxArgs: MAX_ARGS,
  maxArgLen: MAX_ARG_LEN
});

if (require.main === module) {
  entrypoint.runAsMain(process.argv.slice(2));
}

module.exports = {
  ...(entrypoint.target && typeof entrypoint.target === 'object' ? entrypoint.target : {}),
  run: entrypoint.run,
  storeId: STORE_ID,
  normalizeReceiptHash: entrypoint.normalizeReceiptHash
};

export {};
