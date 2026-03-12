#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/autogenesis/trace_habit_autogenesis.js',
  target_rel: 'systems/ops/trace_habit_autogenesis.js'
});
