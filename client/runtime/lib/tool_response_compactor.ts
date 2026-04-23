#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

const CLIENT_ROOT = path.resolve(__dirname, '..');
const TOOL_RAW_DIR = path.join(CLIENT_ROOT, 'local', 'logs', 'tool_raw');
const COMPACTION_THRESHOLD_CHARS = 1200;
const COMPACTION_THRESHOLD_LINES = 40;

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'tool_response_compactor', 'tool-response-compactor-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function payloadFromBridge(out) {
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  return receipt && typeof receipt.payload === 'object' ? receipt.payload : receipt;
}

function invokeFailure(command, out, payloadOut, suffix) {
  const fallback = `tool_response_compactor_kernel_${command}_${suffix}`;
  if (payloadOut && typeof payloadOut.error === 'string' && payloadOut.error.trim()) {
    return payloadOut.error.trim();
  }
  const stderr = out && out.stderr ? String(out.stderr).trim() : '';
  return stderr || fallback;
}

function normalizeCompactorResult(result, fallbackContent) {
  const base = result && typeof result === 'object' ? result : {};
  const normalizedContent = String(
    base && typeof base.content === 'string' ? base.content : (fallbackContent == null ? '' : fallbackContent)
  );
  const metrics = base && typeof base.metrics === 'object' && base.metrics
    ? base.metrics
    : { chars: normalizedContent.length, lines: normalizedContent.split('\n').length };
  return Object.assign({}, base, {
    compacted: base && base.compacted === true,
    content: normalizedContent,
    metrics
  });
}

function invoke(command, payload = {}, opts = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const payloadOut = payloadFromBridge(out);
  if (out.status !== 0) {
    const message = invokeFailure(command, out, payloadOut, 'failed');
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = invokeFailure(command, out, payloadOut, 'bridge_failed');
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function redactSecrets(content) {
  const out = invoke('redact', {
    root_dir: CLIENT_ROOT,
    data: typeof content === 'string' ? content : JSON.stringify(content)
  });
  return String(out.content || '');
}

function extractSummary(data, toolName) {
  const out = invoke('extract-summary', {
    root_dir: CLIENT_ROOT,
    data,
    tool_name: toolName || 'unknown'
  });
  return Array.isArray(out.summary) ? out.summary : [];
}

function compactToolResponse(data, options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const out = invoke('compact', {
    root_dir: opts.rootDir || CLIENT_ROOT,
    data,
    tool_name: opts.toolName || 'unknown'
  });
  if (out && typeof out === 'object') {
    return normalizeCompactorResult(out, '');
  }
  const fallback = redactSecrets(typeof data === 'string' ? data : JSON.stringify(data));
  return normalizeCompactorResult({ compacted: false, content: fallback }, fallback);
}

function redactSecretsOnly(content) {
  return redactSecrets(content);
}

module.exports = {
  TOOL_RAW_DIR,
  COMPACTION_THRESHOLD_CHARS,
  COMPACTION_THRESHOLD_LINES,
  compactToolResponse,
  normalizeCompactorResult,
  redactSecrets,
  redactSecretsOnly,
  extractSummary
};

if (require.main === module) {
  let input = '';
  process.stdin.setEncoding('utf8');
  process.stdin.on('data', (chunk) => {
    input += chunk;
  });
  process.stdin.on('end', () => {
    const result = compactToolResponse(input, { toolName: process.argv[2] || 'test' });
    console.log(result.content);
    if (result.metrics) {
      console.error(`\n[COMPACTOR METRICS] ${JSON.stringify(result.metrics, null, 2)}`);
    }
  });
}
