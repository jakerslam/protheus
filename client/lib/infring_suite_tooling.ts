#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Shell role: thin CLI helper for suite command dispatch.
const { run } = require('../runtime/systems/compat/legacy_alias_adapter.ts');

const SUITE_LANE_ID = 'RUNTIME-CLI-INFRING-SUITE-TOOLING';

function runTool(tool, args = []) {
  return run([`--lane-id=${SUITE_LANE_ID}`, String(tool || '').trim(), ...args]);
}

module.exports = {
  runTool
};
