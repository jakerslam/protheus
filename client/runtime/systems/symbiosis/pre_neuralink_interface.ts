#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/symbiosis/pre_neuralink_interface.js'
}, process.argv.slice(2));
