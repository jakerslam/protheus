#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/autonomy/zero_permission_conversational_layer.js'
}, process.argv.slice(2));
