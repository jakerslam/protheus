#!/usr/bin/env node
'use strict';

const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');
const { sanitizeBridgeArg } = require('../../../client/runtime/lib/runtime_system_entrypoint.ts');

const pkgRoot = path.resolve(__dirname, '..');
const cliPath = path.join(pkgRoot, 'bin', 'infring.ts');
const SMOKE_TIMEOUT_MS = 20000;

function sanitizeText(value, maxLen = 4000) {
  return sanitizeBridgeArg(value, maxLen).replace(/\s+/g, ' ').trim();
}

function extractJsonObjectCandidates(raw) {
  const candidates = [];
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;
  for (let index = 0; index < raw.length; index += 1) {
    const ch = raw[index] || '';
    if (escaped) {
      escaped = false;
      continue;
    }
    if (ch === '\\') {
      if (inString) escaped = true;
      continue;
    }
    if (ch === '"') {
      inString = !inString;
      continue;
    }
    if (inString) continue;
    if (ch === '{') {
      if (depth === 0) start = index;
      depth += 1;
      continue;
    }
    if (ch === '}' && depth > 0) {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        candidates.push(raw.slice(start, index + 1));
        start = -1;
      }
    }
  }
  return candidates;
}

function parseJsonRecordCandidates(raw) {
  const out = [];
  const text = String(raw || '').trim();
  if (!text) return out;
  try {
    const parsed = JSON.parse(text);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      out.push(parsed);
      return out;
    }
  } catch {
    // mixed output fallback below
  }
  for (const fragment of extractJsonObjectCandidates(text)) {
    try {
      const parsed = JSON.parse(fragment);
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) out.push(parsed);
    } catch {
      // ignore malformed fragment
    }
  }
  return out;
}

function readNestedErrorMessage(record) {
  if (!record || typeof record !== 'object' || Array.isArray(record)) return '';
  if (record.error && typeof record.error === 'object') {
    const nested = readNestedErrorMessage(record.error);
    if (nested) return nested;
  }
  if (typeof record.message === 'string' && record.message.trim()) return record.message.trim();
  if (typeof record.error === 'string' && record.error.trim()) return record.error.trim();
  if (typeof record.detail === 'string' && record.detail.trim()) return record.detail.trim();
  return '';
}

function run(args) {
  const out = spawnSync(process.execPath, [cliPath, ...(Array.isArray(args) ? args : [])], {
    cwd: path.resolve(pkgRoot, '..', '..'),
    encoding: 'utf8',
    timeout: SMOKE_TIMEOUT_MS,
  });
  const stdout = String(out.stdout || '');
  const stderr = String(out.stderr || '');
  const parsed = parseJsonRecordCandidates(`${stdout}\n${stderr}`);
  const extractedError = parsed.map((record) => readNestedErrorMessage(record)).find(Boolean) || '';
  return {
    code: Number.isFinite(out.status) ? out.status : 1,
    stdout,
    stderr,
    parsed,
    extractedError,
    error: out.error ? sanitizeText(out.error.message || out.error, 240) : null,
  };
}

function main() {
  const help = run(['--help']);
  assert.strictEqual(help.code, 0, help.error || help.extractedError || help.stderr || help.stdout);

  const combined = sanitizeText(help.stdout + ' ' + help.stderr, 6000);
  const parsedSignals = help.parsed.some((record) =>
    record && typeof record === 'object' && (
      record.ok === true ||
      typeof record.usage === 'string' ||
      typeof record.type === 'string'
    )
  );

  assert.ok(
    combined.includes('Usage') ||
      combined.includes('infring') ||
      combined.includes('ok') ||
      combined.includes('lane_id') ||
      parsedSignals,
    'expected help text or structured receipt from infring wrapper'
  );

  process.stdout.write('packages/infring-npm/scripts/smoke.ts: OK\n');
}

try {
  main();
} catch (err) {
  process.stderr.write('packages/infring-npm/scripts/smoke.ts: FAIL: ' + err.message + '\n');
  process.exit(1);
}
