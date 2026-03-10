#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'federated_sovereign_mesh_runtime', 'RUNTIME-SYSTEMS-OPS-FEDERATED_SOVEREIGN_MESH_RUNTIME');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
