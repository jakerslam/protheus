#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'data_retention_tiering', 'RUNTIME-SYSTEMS-OPS-DATA_RETENTION_TIERING');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
