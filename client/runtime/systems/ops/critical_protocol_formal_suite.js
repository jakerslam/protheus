#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'critical_protocol_formal_suite', 'RUNTIME-SYSTEMS-OPS-CRITICAL_PROTOCOL_FORMAL_SUITE');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
