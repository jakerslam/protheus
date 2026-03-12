#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/fractal/warden/complexity_warden_meta_organ.js'
}, process.argv.slice(2));
