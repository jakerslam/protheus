'use strict';
const { createConduitImporter } = require('./generic_json_importer.ts');
const importer = createConduitImporter('infring', 'importer-infring', 'IMPORTER_INFRING');

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
      // fall through
    }
  }
  return {};
}

function importPayload(payload: unknown, context: Record<string, unknown> = {}) {
  return importer.importPayload(normalizePayload(payload), {
    ...(context && typeof context === 'object' && !Array.isArray(context) ? context : {}),
    source_engine: 'infring',
  });
}

module.exports = {
  engine: importer.engine,
  importPayload,
};
