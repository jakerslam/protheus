#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer1/security::directive-hierarchy-controller (authoritative).

const { bindCompatibilityBridgeModule } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindCompatibilityBridgeModule('../../lib/directive_resolver.ts', module);
