#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/redteam/quantum_security_primitive_synthesis.js'
}, process.argv.slice(2));
