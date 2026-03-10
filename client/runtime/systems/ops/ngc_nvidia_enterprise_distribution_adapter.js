#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'ngc_nvidia_enterprise_distribution_adapter', 'RUNTIME-SYSTEMS-OPS-NGC_NVIDIA_ENTERPRISE_DISTRIBUTION_ADAPTER');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
