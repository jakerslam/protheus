#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: apps/agent-holo-viz (authoritative)

/**
 * Compatibility launcher.
 *
 * Canonical visualizer server now lives in:
 *   client/runtime/local/workspaces/agent-holo-viz/server/system_visualizer_server.js
 */
const fs = require('fs');
const path = require('path');

function resolveSidecarMain() {
  const override = process.env.INFRING_VISUALIZER_SIDECAR_MAIN;
  if (override && String(override).trim()) {
    return path.resolve(String(override).trim());
  }
  return path.join(
    __dirname,
    '..',
    '..',
    'local',
    'workspaces',
    'agent-holo-viz',
    'server',
    'system_visualizer_server.js'
  );
}

function run() {
  const sidecarMain = resolveSidecarMain();
  if (!fs.existsSync(sidecarMain)) {
    process.stderr.write(
      '[visualizer] sidecar repo not found at client/runtime/local/workspaces/agent-holo-viz/. ' +
      'Create/populate $INFRING_WORKSPACE/client/runtime/local/workspaces/agent-holo-viz first.\n'
    );
    return 1;
  }
  const mod = require(sidecarMain);
  if (mod && typeof mod.main === 'function') {
    mod.main();
    return 0;
  }
  process.stderr.write('[visualizer] invalid sidecar server module (missing main export).\n');
  return 1;
}

if (require.main === module) {
  process.exit(run());
}

module.exports = {
  resolveSidecarMain,
  run,
};
