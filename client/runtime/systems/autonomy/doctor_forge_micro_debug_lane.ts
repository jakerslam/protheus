#!/usr/bin/env node
'use strict';
const FORBIDDEN_RUNTIME_CONTEXT_MARKERS = [
  'You are an expert Python programmer.',
  '[PATCH v2',
  'List Leaves (25',
  'BEGIN_OPENCLAW_INTERNAL_CONTEXT',
  'END_OPENCLAW_INTERNAL_CONTEXT',
  'UNTRUSTED_CHILD_RESULT_DELIMITER'
];

function containsForbiddenRuntimeContextMarker(raw = '') {
  const text = String(raw);
  return FORBIDDEN_RUNTIME_CONTEXT_MARKERS.some((marker) => text.includes(marker));
}

const legacyModule = require('../../lib/legacy_retired_wrapper.ts').bindLegacyRetiredModule(__filename, module);
module.exports = Object.assign(legacyModule, {
  forbiddenRuntimeContextMarkers: FORBIDDEN_RUNTIME_CONTEXT_MARKERS,
  containsForbiddenRuntimeContextMarker
});
