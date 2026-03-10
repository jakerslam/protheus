#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'control_plane_live_activation_shadow_exit_gate', 'RUNTIME-SYSTEMS-OPS-CONTROL_PLANE_LIVE_ACTIVATION_SHADOW_EXIT_GATE');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
