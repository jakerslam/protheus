#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer3/cognition + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createCognitionModule, runAsMain } = require('../../../lib/legacy_retired_wrapper.ts');
const mod = createCognitionModule(__dirname, 'publish', 'COGNITION-SKILLS-MOLTSTACK-SCRIPTS-PUBLISH');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
