#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'protheus_core_runtime_envelope', 'RUNTIME-SYSTEMS-OPS-PROTHEUS_CORE_RUNTIME_ENVELOPE');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
