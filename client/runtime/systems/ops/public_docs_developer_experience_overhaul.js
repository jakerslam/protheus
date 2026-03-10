#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createLegacyRetiredModule, runAsMain } = require('../../lib/legacy_retired_wrapper.js');
const mod = createLegacyRetiredModule(__dirname, 'public_docs_developer_experience_overhaul', 'RUNTIME-SYSTEMS-OPS-PUBLIC_DOCS_DEVELOPER_EXPERIENCE_OVERHAUL');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
