#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::shannon-desktop-shell (authoritative workflow desktop bridge).

const TARGET = '../../../../adapters/runtime/shannon_desktop_shell.ts';

function loadTarget() {
  try {
    return require(TARGET);
  } catch (error) {
    return {
      ok: false,
      error: 'shannon_desktop_shell_target_load_failed',
      detail: String(error && error.message ? error.message : error || 'unknown_error')
    };
  }
}

const target = loadTarget();
module.exports = target;

if (require.main === module && target && target.ok === false) {
  process.stderr.write(`${JSON.stringify(target)}\n`);
  process.exit(1);
}
