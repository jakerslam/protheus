#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/redteam/self_improving_redteam_trainer.js'
}, process.argv.slice(2));
