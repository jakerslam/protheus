#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::contract-check (authoritative).

const { bindCompatibilityBridgeModule } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindCompatibilityBridgeModule('./contract_check.ts', module);
