#!/usr/bin/env node
// @ts-nocheck
'use strict';

// Layer ownership: core/layer2/runtime + core/layer0/ops::legacy-retired-lane (authoritative)
// TypeScript compatibility shim only.
const mod = require('./detector_rollback_migration_safety_contract.ts');
if (require.main === module) mod.run(process.argv.slice(2));
module.exports = mod;
