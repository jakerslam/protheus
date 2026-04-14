#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: client/runtime/lib (thin bridge over core/layer0/ops pydantic-ai-bridge)

const path = require('path');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK = process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK || '0';
process.env.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK = process.env.PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK || '0';
const bridge = createOpsLaneBridge(__dirname, 'pydantic_ai_bridge', 'pydantic-ai-bridge', {
  preferLocalCore: true
});
const PATH_ARG_KEYS = ['state_path', 'history_path', 'swarm_state_path', 'approval_queue_path'];

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function normalizePayload(payload = {}) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    return {};
  }
  const normalized = { ...payload };
  for (const key of PATH_ARG_KEYS) {
    const value = normalized[key];
    if (typeof value === 'string' && value.trim().length > 0) {
      normalized[key] = path.resolve(value);
    }
  }
  return normalized;
}

function appendPathArgs(args, payload, keys) {
  for (const key of keys) {
    const value = payload[key];
    if (typeof value === 'string' && value.length > 0) {
      args.push(`--${key.replace(/_/g, '-')}=${String(value)}`);
    }
  }
}

function invoke(command, payload = {}, opts = {}) {
  const normalizedPayload = normalizePayload(payload);
  const args = [command, `--payload-base64=${encodeBase64(JSON.stringify(normalizedPayload))}`];
  appendPathArgs(args, normalizedPayload, PATH_ARG_KEYS);
  const out = bridge.run(args);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object' ? receipt.payload : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `pydantic_ai_bridge_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `pydantic_ai_bridge_${command}_failed`);
    return { ok: false, error: message || `pydantic_ai_bridge_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `pydantic_ai_bridge_${command}_bridge_failed`
      : `pydantic_ai_bridge_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

const status = (opts = {}) => invoke('status', opts);
const registerAgent = (payload) => invoke('register-agent', payload);
const validateOutput = (payload) => invoke('validate-output', payload);
const registerToolContext = (payload) => invoke('register-tool-context', payload);
const invokeToolContext = (payload) => invoke('invoke-tool-context', payload);
const bridgeProtocol = (payload) => invoke('bridge-protocol', payload);
const durableRun = (payload) => invoke('durable-run', payload);
const approvalCheckpoint = (payload) => invoke('approval-checkpoint', payload);
const recordLogfire = (payload) => invoke('record-logfire', payload);
const executeGraph = (payload) => invoke('execute-graph', payload);
const streamModel = (payload) => invoke('stream-model', payload);
const recordEval = (payload) => invoke('record-eval', payload);
const assimilateIntake = (payload) => invoke('assimilate-intake', payload);
const registerRuntimeBridge = (payload) => invoke('register-runtime-bridge', payload);
const routeModel = (payload) => invoke('route-model', payload);
const runGovernedWorkflow = (payload) => invoke('run-governed-workflow', payload);

module.exports = {
  status,
  registerAgent,
  validateOutput,
  registerToolContext,
  invokeToolContext,
  bridgeProtocol,
  durableRun,
  approvalCheckpoint,
  recordLogfire,
  executeGraph,
  streamModel,
  recordEval,
  assimilateIntake,
  registerRuntimeBridge,
  routeModel,
  runGovernedWorkflow,
  normalizePayload,
  appendPathArgs,
  PATH_ARG_KEYS,
};
