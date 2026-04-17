#!/usr/bin/env tsx
// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::contract-check (authoritative contract validation route).

const TARGET_MODULE = '../../../../adapters/runtime/protheus_cli_modules.ts';
const TARGET_EXPORT = 'contractCheck';

function loadTarget() {
  try {
    const mod = require(TARGET_MODULE);
    const target = mod && mod[TARGET_EXPORT];
    if (!target || typeof target.run !== 'function') {
      return {
        ok: false,
        error: 'contract_check_target_missing_run',
      };
    }
    return target;
  } catch (error) {
    return {
      ok: false,
      error: 'contract_check_target_load_failed',
      detail: String(error && error.message ? error.message : error || 'unknown_error'),
    };
  }
}

const target = loadTarget();

function run(args = process.argv.slice(2)) {
  if (!target || target.ok === false) {
    process.stderr.write(JSON.stringify(target || { ok: false, error: 'contract_check_target_unavailable' }) + '\n');
    return 1;
  }
  return target.run(Array.isArray(args) ? args : []);
}

if (require.main === module) {
  const code = run(process.argv.slice(2));
  process.exit(Number.isFinite(Number(code)) ? Number(code) : 1);
}

module.exports = {
  ...(target && typeof target === 'object' ? target : {}),
  run,
};
