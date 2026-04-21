#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: core/layer1/security::startup-attestation (authoritative).

const { bindLegacyRetiredModuleSafe } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindLegacyRetiredModuleSafe(__filename, module);
