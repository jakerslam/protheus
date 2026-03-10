#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'guard_check_registry', 'RUNTIME-SYSTEMS-OPS-GUARD_CHECK_REGISTRY');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
