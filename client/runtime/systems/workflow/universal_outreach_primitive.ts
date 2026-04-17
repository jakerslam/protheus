#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration; this file is a thin CLI bridge.

const TARGET = '../../../../surface/orchestration/scripts/universal_outreach_primitive.ts';

function loadTarget() {
  try {
    return require(TARGET);
  } catch (error) {
    return {
      ok: false,
      error: 'universal_outreach_primitive_target_load_failed',
      detail: String(error && error.message ? error.message : error || 'unknown_error')
    };
  }
}

const target = loadTarget();

function run(args = process.argv.slice(2)) {
  if (!target || target.ok === false) {
    process.stderr.write(`${JSON.stringify(target || { ok: false, error: 'universal_outreach_primitive_target_unavailable' })}\n`);
    return 1;
  }
  if (typeof target.run !== 'function') {
    process.stderr.write(`${JSON.stringify({ ok: false, error: 'universal_outreach_primitive_target_missing_run' })}\n`);
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
  run
};
