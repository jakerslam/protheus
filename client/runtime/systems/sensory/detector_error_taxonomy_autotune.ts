#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/sensory/detector_error_taxonomy_autotune.js'
}, process.argv.slice(2));
