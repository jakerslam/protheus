#!/usr/bin/env node
'use strict';

const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..');
const RUN_PROTHEUS_OPS_PATH = path.join(ROOT, 'adapters', 'runtime', 'run_protheus_ops.ts');
const OPS_LANE_BRIDGE_PATH = path.join(ROOT, 'adapters', 'runtime', 'ops_lane_bridge.ts');

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => String(line || '').trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}

function extractPayload(receipt) {
  if (receipt && typeof receipt === 'object' && receipt.payload && typeof receipt.payload === 'object') {
    return receipt.payload;
  }
  if (receipt && typeof receipt === 'object') return receipt;
  return null;
}

function loadBridgeInvoker() {
  delete require.cache[require.resolve(RUN_PROTHEUS_OPS_PATH)];
  delete require.cache[require.resolve(OPS_LANE_BRIDGE_PATH)];
  return require(RUN_PROTHEUS_OPS_PATH).invokeProtheusOpsViaBridge;
}

function invokeKernel(domain, command, payload = {}, options = {}) {
  const invokeProtheusOpsViaBridge = loadBridgeInvoker();
  const args = [
    domain,
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload && typeof payload === 'object' ? payload : {}))}`,
  ];
  const run =
    invokeProtheusOpsViaBridge(args, {
      allowProcessFallback: options.allowProcessFallback === true,
      unknownDomainFallback: options.unknownDomainFallback === true,
    }) || { status: 1, stdout: '', stderr: 'resident_ipc_bridge_unavailable', payload: null };

  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  const stdout = String(run.stdout || '');
  const stderr = String(run.stderr || '');
  const receipt = run && run.payload && typeof run.payload === 'object' ? run.payload : parseLastJson(stdout);
  const payloadOut = extractPayload(receipt);

  return {
    ok: status === 0 && !!payloadOut,
    status,
    stdout,
    stderr,
    receipt,
    payload: payloadOut,
  };
}

function stderrOrStdout(result) {
  return String(result.stderr || '').trim() || String(result.stdout || '').trim();
}

function invokeKernelPayload(domain, command, payload = {}, options = {}) {
  const result = invokeKernel(domain, command, payload, options);
  if (result.status !== 0 || !result.payload || typeof result.payload !== 'object') {
    const fallback = cleanText(options.fallbackError || `${domain}_${command}_bridge_failed`, 320);
    const message = cleanText(
      (result.payload && result.payload.error) || stderrOrStdout(result) || fallback,
      320,
    );
    if (options.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return result.payload;
}

module.exports = {
  cleanText,
  encodeBase64,
  extractPayload,
  invokeKernel,
  invokeKernelPayload,
  parseLastJson,
};
