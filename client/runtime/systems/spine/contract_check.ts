#!/usr/bin/env tsx
// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::contract-check (authoritative contract validation route).

const { createCompatTargetBridge } = require('../../lib/compat_target_bridge.ts');
const MAX_ARGS = 64;
const MAX_ARG_LEN = 512;
const bridge = createCompatTargetBridge({
  scriptDir: __dirname,
  targetRelativePath: '../../../../adapters/runtime/infring_cli_modules.ts',
  targetExport: 'contractCheck',
  loadError: 'contract_check_target_load_failed',
  unavailableError: 'contract_check_target_unavailable',
  missingExportError: 'contract_check_target_missing_export',
  missingRunError: 'contract_check_target_missing_run',
  maxArgs: MAX_ARGS,
  maxArgLen: MAX_ARG_LEN
});

if (require.main === module) {
  bridge.runAsMain(process.argv.slice(2));
}

module.exports = {
  ...(bridge.target && typeof bridge.target === 'object' ? bridge.target : {}),
  run: bridge.run,
  normalizeReceiptHash: bridge.normalizeReceiptHash
};
