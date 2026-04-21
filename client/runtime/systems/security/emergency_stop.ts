#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer1/security::emergency-stop (authoritative).

const { bindCompatibilityBridgeModule } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindCompatibilityBridgeModule('../../../lib/emergency_stop.ts', module);
