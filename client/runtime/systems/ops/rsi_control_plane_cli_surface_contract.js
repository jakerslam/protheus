#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'rsi_control_plane_cli_surface_contract', 'RUNTIME-SYSTEMS-OPS-RSI_CONTROL_PLANE_CLI_SURFACE_CONTRACT');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
