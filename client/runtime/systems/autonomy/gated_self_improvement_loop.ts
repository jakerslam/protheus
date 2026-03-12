#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/autonomy/gated_self_improvement_loop.js'
}, process.argv.slice(2));
