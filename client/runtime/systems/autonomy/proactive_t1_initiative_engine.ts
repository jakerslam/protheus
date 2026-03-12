#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/autonomy/proactive_t1_initiative_engine.js'
}, process.argv.slice(2));
