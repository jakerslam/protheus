'use strict';
const { createConduitImporter } = require('./generic_json_importer.ts');

type AnyObj = Record<string, any>;
const importer = createConduitImporter(
  'generic_yaml',
  'importer-generic-yaml',
  'IMPORTER_GENERIC_YAML',
);

function cleanText(value: unknown, maxLen = 260) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, maxLen);
}

function parseScalar(raw: string) {
  const value = String(raw || '').trim();
  if (!value) return '';
  if (value === 'true') return true;
  if (value === 'false') return false;
  if (value === 'null') return null;
  if (/^[+-]?\d+$/.test(value)) {
    const parsedInt = Number(value);
    if (Number.isSafeInteger(parsedInt)) return parsedInt;
  }
  if (/^[+-]?\d+\.\d+$/.test(value)) {
    const parsedFloat = Number.parseFloat(value);
    if (Number.isFinite(parsedFloat)) return parsedFloat;
  }
  if ((value.startsWith('{') && value.endsWith('}')) || (value.startsWith('[') && value.endsWith(']'))) {
    try {
      return JSON.parse(value);
    } catch {
      // scalar fallback below
    }
  }
  return value.replace(/^['"]|['"]$/g, '');
}

function parseSimpleYaml(text: unknown) {
  const out: AnyObj = {};
  const lines = String(text || '').split(/\r?\n/);
  let currentArrayKey = '';

  for (const rawLine of lines) {
    const line = String(rawLine || '').replace(/\t/g, '  ').trim();
    if (!line || line.startsWith('#')) continue;

    if (currentArrayKey && /^-\s+/.test(line)) {
      const itemValue = parseScalar(line.replace(/^-\s+/, ''));
      if (!Array.isArray(out[currentArrayKey])) {
        out[currentArrayKey] = [];
      }
      out[currentArrayKey].push(itemValue);
      continue;
    }

    const keyOnly = line.match(/^([A-Za-z0-9_.-]+):\s*$/);
    if (keyOnly) {
      const arrayKey = cleanText(keyOnly[1], 120);
      if (arrayKey) {
        currentArrayKey = arrayKey;
        if (!Array.isArray(out[currentArrayKey])) out[currentArrayKey] = [];
      }
      continue;
    }

    const idx = line.indexOf(':');
    if (idx <= 0) continue;
    const key = cleanText(line.slice(0, idx), 120);
    const raw = line.slice(idx + 1).trim();
    if (!key) continue;
    out[key] = parseScalar(raw);
    currentArrayKey = '';
  }

  return out;
}

function normalizePayload(payload: unknown) {
  if (payload && typeof payload === 'object' && !Array.isArray(payload)) {
    return payload;
  }
  if (typeof payload === 'string') {
    return parseSimpleYaml(payload);
  }
  return {};
}

function importPayload(payload: unknown, context: AnyObj = {}) {
  const normalized = normalizePayload(payload);
  const normalizedContext = {
    ...(context && typeof context === 'object' && !Array.isArray(context) ? context : {}),
    source_engine: 'generic_yaml',
  };
  return importer.importPayload(normalized, normalizedContext);
}

module.exports = {
  engine: importer.engine,
  parseSimpleYaml,
  importPayload,
};
