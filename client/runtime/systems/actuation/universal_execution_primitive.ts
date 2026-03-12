#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/actuation/universal_execution_primitive.js'
}, process.argv.slice(2));
