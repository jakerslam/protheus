#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'chaos_self_healing_automation', 'RUNTIME-SYSTEMS-OPS-CHAOS_SELF_HEALING_AUTOMATION');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
