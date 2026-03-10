#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'state_stream_policy_check', 'RUNTIME-SYSTEMS-OPS-STATE_STREAM_POLICY_CHECK');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
