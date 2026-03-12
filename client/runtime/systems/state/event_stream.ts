#!/usr/bin/env node
'use strict';
const { runLegacyAlias } = require('../../../lib/legacy_alias_adapter.ts');

runLegacyAlias({
  alias_rel: 'systems/state/event_stream.js',
  target_rel: 'systems/ops/event_sourced_control_plane.js'
});
