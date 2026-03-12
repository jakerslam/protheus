#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/hybrid/mobile/protheus_mobile_adapter.js'
}, process.argv.slice(2));
