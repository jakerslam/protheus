#!/usr/bin/env node
'use strict';

const path = require('node:path');

// Layer ownership: core/layer3/cognition + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const { createCognitionModule, runAsMain } = require(path.resolve(
  __dirname,
  '..',
  '..',
  '..',
  '..',
  'client',
  'cognition',
  'shared',
  'lib',
  'legacy_retired_wrapper.ts'
));
const mod = createCognitionModule(__dirname, 'proposal_template', 'COGNITION-SKILLS-MOLTBOOK-PROPOSAL_TEMPLATE');
if (require.main === module) runAsMain(mod, process.argv.slice(2));
module.exports = mod;
