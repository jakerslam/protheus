'use strict';
const path = require('path');
const { createDomainProxy } = require(path.resolve(process.cwd(), 'client/lib/legacy_conduit_proxy.ts'));

type AnyObj = Record<string, any>;

const runDomain = createDomainProxy(__dirname, 'IMPORTER_GENERIC_YAML', 'execution-yield-recovery');
const MAX_PAYLOAD_BYTES = 1024 * 1024;

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '').replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function encodePayloadBase64(payload: unknown) {
  try {
    const serialized = JSON.stringify(payload == null ? {} : payload);
    const bytes = Buffer.byteLength(serialized, 'utf8');
    if (bytes > MAX_PAYLOAD_BYTES) return { ok: false, error: 'payload_too_large:' + bytes };
    return { ok: true, encoded: Buffer.from(serialized, 'utf8').toString('base64') };
  } catch (error) {
    return { ok: false, error: cleanText(error && error.message ? error.message : error, 220) || 'payload_encode_failed' };
  }
}

function runViaConduit(payloadBase64: string) {
  const out = runDomain(['importer-generic-yaml', '--payload-base64=' + String(payloadBase64 || '')]);
  if (out && out.ok === true && out.payload && typeof out.payload === 'object' && out.payload.ok === true && out.payload.payload && typeof out.payload.payload === 'object') {
    return { ok: true, payload: out.payload.payload };
  }
  return { ok: false, error: cleanText((out && out.error) || 'conduit_importer_unavailable', 260) };
}

function normalizeImportedPayload(payload: AnyObj) {
  const entities = payload && typeof payload.entities === 'object' ? payload.entities : {};
  return {
    entities: {
      agents: Array.isArray(entities.agents) ? entities.agents : [],
      tasks: Array.isArray(entities.tasks) ? entities.tasks : [],
      workflows: Array.isArray(entities.workflows) ? entities.workflows : [],
      tools: Array.isArray(entities.tools) ? entities.tools : [],
      records: Array.isArray(entities.records) ? entities.records : []
    },
    source_item_count: Number((payload && payload.source_item_count) || 0),
    mapped_item_count: Number((payload && payload.mapped_item_count) || 0),
    warnings: Array.isArray(payload && payload.warnings)
      ? payload.warnings.map((v: unknown) => cleanText(v, 220)).filter(Boolean)
      : []
  };
}

function parseSimpleYaml(text: unknown) {
  const out: AnyObj = {};
  String(text || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith('#'))
    .forEach((line) => {
      const idx = line.indexOf(':');
      if (idx <= 0) return;
      const key = line.slice(0, idx).trim();
      const raw = line.slice(idx + 1).trim();
      if (!key) return;
      if (raw === 'true' || raw === 'false') { out[key] = raw === 'true'; return; }
      if (/^-?\d+(\.\d+)?$/.test(raw)) { out[key] = Number(raw); return; }
      out[key] = raw.replace(/^['"]|['"]$/g, '');
    });
  return out;
}

function importPayload(payload: unknown, context: AnyObj = {}) {
  void context;
  const encoded = encodePayloadBase64(payload);
  if (!encoded.ok) {
    return {
      entities: { agents: [], tasks: [], workflows: [], tools: [], records: [] },
      source_item_count: 0,
      mapped_item_count: 0,
      warnings: ['payload_encode_failed:' + cleanText(encoded.error || 'unknown', 220)]
    };
  }
  const result = runViaConduit(encoded.encoded);
  if (result.ok && result.payload) return normalizeImportedPayload(result.payload);
  const err = cleanText(result.error || 'conduit_importer_unavailable', 220);
  return {
    entities: { agents: [], tasks: [], workflows: [], tools: [], records: [] },
    source_item_count: 0,
    mapped_item_count: 0,
    warnings: ['conduit_importer_unavailable:' + err]
  };
}

module.exports = {
  engine: 'generic_yaml',
  parseSimpleYaml,
  importPayload
};
