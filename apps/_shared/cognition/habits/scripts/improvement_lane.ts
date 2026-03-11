#!/usr/bin/env node
/**
 * Compatibility wrapper.
 * Moved to: client/runtime/systems/autonomy/improvement_orchestrator.ts
 */

const path = require('path');
const { spawnSync } = require('child_process');

const target = path.resolve(__dirname, '..', '..', 'systems', 'autonomy', 'improvement_orchestrator.js');

function usage() {
  process.stdout.write('Usage: improvement_lane.js propose|start-next|evaluate-open [options]\n');
}

if (require.main === module) {
  const first = String(process.argv[2] || '').trim().toLowerCase();
  if (!first || first === '--help' || first === '-h' || first === 'help') {
    usage();
    process.exit(0);
  }
  const r = spawnSync(process.execPath, [target, ...process.argv.slice(2)], {
    stdio: 'inherit',
    env: process.env
  });
  process.exit(r.status == null ? 1 : r.status);
}

module.exports = require(target);
