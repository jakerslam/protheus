#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'automated_compliance_mapping_evidence_engine', 'RUNTIME-SYSTEMS-OPS-AUTOMATED_COMPLIANCE_MAPPING_EVIDENCE_ENGINE');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
