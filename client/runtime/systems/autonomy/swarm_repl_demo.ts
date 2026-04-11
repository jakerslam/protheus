#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::swarm-repl-demo (authoritative bridge-driven demo shell).

const { bindCompatibilityBridgeModule } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindCompatibilityBridgeModule('../../../adapters/runtime/swarm_repl_demo.ts', module);
