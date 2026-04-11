#!/usr/bin/env node
'use strict';
// Legacy runtime lane retained as a governed compatibility surface.
// Layer ownership: core/layer1/memory_runtime + core/layer0/ops::legacy-retired-lane (authoritative)
const { bindLegacyRetiredModule } = require('../../lib/legacy_retired_wrapper.ts');

module.exports = bindLegacyRetiredModule(__filename, module);
