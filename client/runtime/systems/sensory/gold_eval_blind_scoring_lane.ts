#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/sensory/gold_eval_blind_scoring_lane.js'
}, process.argv.slice(2));
