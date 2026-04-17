#!/usr/bin/env node
'use strict';

const TARGET = '../../lib/google_adk_bridge.ts';

function loadTarget() {
  try {
    return require(TARGET);
  } catch (error) {
    return {
      ok: false,
      error: 'google_adk_bridge_target_load_failed',
      detail: String(error && error.message ? error.message : error || 'unknown_error'),
    };
  }
}

const target = loadTarget();
const exported =
  target && typeof target === 'object' && !Array.isArray(target)
    ? target
    : {
        ok: false,
        error: 'google_adk_bridge_target_invalid',
      };

if (require.main === module && exported && exported.ok === false) {
  process.stderr.write(JSON.stringify(exported) + '\n');
  process.exit(1);
}

module.exports = exported;
