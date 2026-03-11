#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer3/cognition + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createCognitionModule, runAsMain } = require('../../lib/legacy_retired_wrapper.ts');
const mod = createCognitionModule(__dirname, 'trust_add_habit', 'COGNITION-HABITS-SCRIPTS-TRUST_ADD_HABIT');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
