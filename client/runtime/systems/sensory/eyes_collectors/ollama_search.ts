#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: adapters/cognition/collectors (authoritative)

const { bindCompatibilityBridgeModule } = require('../../../lib/legacy_retired_wrapper.ts');

module.exports = bindCompatibilityBridgeModule('../../../../../adapters/cognition/collectors/ollama_search.ts', module);
