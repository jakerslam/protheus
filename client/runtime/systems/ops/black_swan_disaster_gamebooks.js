#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'black_swan_disaster_gamebooks', 'RUNTIME-SYSTEMS-OPS-BLACK_SWAN_DISASTER_GAMEBOOKS');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
