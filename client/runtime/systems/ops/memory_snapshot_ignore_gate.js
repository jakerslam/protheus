#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'memory_snapshot_ignore_gate', 'RUNTIME-SYSTEMS-OPS-MEMORY_SNAPSHOT_IGNORE_GATE');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
