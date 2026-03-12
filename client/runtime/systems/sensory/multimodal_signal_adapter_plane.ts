#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/sensory/multimodal_signal_adapter_plane.js'
}, process.argv.slice(2));
