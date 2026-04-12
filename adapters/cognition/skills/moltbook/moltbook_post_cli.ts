#!/usr/bin/env node
'use strict';

// Compatibility shim: keep the legacy post CLI entrypoint, but execute it
// with the same sync/async-safe main contract as every other retired wrapper.
const { runAsMain } = require('../../../../client/runtime/lib/legacy_retired_wrapper.ts');
const mod = require('./moltbook_publish_guard.ts');

if (require.main === module) runAsMain(mod, process.argv.slice(2));

module.exports = mod;
