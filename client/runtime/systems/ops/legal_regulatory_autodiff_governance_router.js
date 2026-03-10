#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'legal_regulatory_autodiff_governance_router', 'RUNTIME-SYSTEMS-OPS-LEGAL_REGULATORY_AUTODIFF_GOVERNANCE_ROUTER');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
