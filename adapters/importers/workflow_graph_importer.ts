'use strict';
const { createConduitImporter } = require('./generic_json_importer.ts');
const importer = createConduitImporter(
  'workflow_graph',
  'importer-workflow-graph',
  'IMPORTER_WORKFLOW_GRAPH',
);

function normalizePayload(payload: unknown) {
  if (payload && typeof payload === 'object' && !Array.isArray(payload)) {
    return payload;
  }
  if (typeof payload === 'string') {
    try {
      const parsed = JSON.parse(payload);
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        return parsed;
      }
    } catch {
      // keep fail-closed fallback
    }
  }
  return {};
}

function importPayload(payload: unknown, context: Record<string, unknown> = {}) {
  return importer.importPayload(normalizePayload(payload), {
    ...(context && typeof context === 'object' && !Array.isArray(context) ? context : {}),
    source_engine: 'workflow_graph',
  });
}

module.exports = {
  engine: importer.engine,
  importPayload,
};
