#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/symbiosis/deep_symbiosis_understanding_layer.js'
}, process.argv.slice(2));
