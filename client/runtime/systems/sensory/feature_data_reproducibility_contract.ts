#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/sensory/feature_data_reproducibility_contract.js'
}, process.argv.slice(2));
