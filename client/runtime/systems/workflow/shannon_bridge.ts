#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::shannon-bridge (authoritative shared workflow bridge).

const TARGET = '../../lib/shannon_bridge.ts';
const BRIDGE_PATH = 'client/runtime/lib/shannon_bridge.ts';
const BRIDGE_TARGET = 'adapters/runtime/shannon_bridge.ts';
const FRAMEWORK = 'shannon';

function loadImpl() {
  try {
    return require(TARGET);
  } catch (error) {
    return {
      ok: false,
      error: 'shannon_bridge_target_load_failed',
      detail: String(error && error.message ? error.message : error || 'unknown_error'),
    };
  }
}

function withBridgeMetadata(payload = {}) {
  return {
    bridge_path: BRIDGE_PATH,
    framework: FRAMEWORK,
    ...payload,
  };
}

const impl = loadImpl();
const exported =
  impl && typeof impl === 'object' && !Array.isArray(impl)
    ? {
        BRIDGE_PATH,
        BRIDGE_TARGET,
        FRAMEWORK,
        withBridgeMetadata,
        ...impl,
      }
    : {
        ok: false,
        error: 'shannon_bridge_target_invalid',
      };

if (require.main === module && exported && exported.ok === false) {
  process.stderr.write(JSON.stringify(exported) + '\n');
  process.exit(1);
}

module.exports = exported;
