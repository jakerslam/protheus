#!/usr/bin/env node
'use strict';

const WRAPPER = '../../lib/legacy_retired_wrapper.ts';

function loadBoundModule() {
  try {
    const runtime = require(WRAPPER);
    if (!runtime || typeof runtime.bindLegacyRetiredModule !== 'function') {
      return {
        ok: false,
        error: 'helix_controller_target_missing_bind',
      };
    }
    return runtime.bindLegacyRetiredModule(__filename, module);
  } catch (error) {
    return {
      ok: false,
      error: 'helix_controller_target_load_failed',
      detail: String(error && error.message ? error.message : error || 'unknown_error'),
    };
  }
}

const bound = loadBoundModule();

if (require.main === module && bound && bound.ok === false) {
  process.stderr.write(JSON.stringify(bound) + '\n');
  process.exit(1);
}

module.exports = bound;
