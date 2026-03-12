#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/autonomy/self_improvement_cadence_orchestrator.js'
}, process.argv.slice(2));
