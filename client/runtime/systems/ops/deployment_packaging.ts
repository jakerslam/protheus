#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops::deployment-packaging (authoritative).

const { bindLegacyRetiredModuleSafe } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindLegacyRetiredModuleSafe(__filename, module);
