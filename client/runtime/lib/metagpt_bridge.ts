#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: client/runtime/lib (thin bridge over core/layer0/ops metagpt-bridge)

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'metagpt_bridge', 'metagpt-bridge', {
  preferLocalCore: true
});

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command, payload = {}, opts = {}) {
  const args = [command, `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`];
  if (payload && payload.state_path) args.push(`--state-path=${String(payload.state_path)}`);
  if (payload && payload.history_path) args.push(`--history-path=${String(payload.history_path)}`);
  if (payload && payload.approval_queue_path) args.push(`--approval-queue-path=${String(payload.approval_queue_path)}`);
  if (payload && payload.trace_path) args.push(`--trace-path=${String(payload.trace_path)}`);
  const out = bridge.run(args);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object' ? receipt.payload : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `metagpt_bridge_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `metagpt_bridge_${command}_failed`);
    return { ok: false, error: message || `metagpt_bridge_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `metagpt_bridge_${command}_bridge_failed`
      : `metagpt_bridge_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

const status = (opts = {}) => invoke('status', opts);
const registerCompany = (payload) => invoke('register-company', payload);
const runSop = (payload) => invoke('run-sop', payload);
const simulatePr = (payload) => invoke('simulate-pr', payload);
const runDebate = (payload) => invoke('run-debate', payload);
const planRequirements = (payload) => invoke('plan-requirements', payload);
const recordOversight = (payload) => invoke('record-oversight', payload);
const recordPipelineTrace = (payload) => invoke('record-pipeline-trace', payload);
const ingestConfig = (payload) => invoke('ingest-config', payload);

module.exports = {
  status,
  registerCompany,
  runSop,
  simulatePr,
  runDebate,
  planRequirements,
  recordOversight,
  recordPipelineTrace,
  ingestConfig,
};
