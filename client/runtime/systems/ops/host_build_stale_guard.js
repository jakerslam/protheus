#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'host_build_stale_guard', 'RUNTIME-SYSTEMS-OPS-HOST_BUILD_STALE_GUARD');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
