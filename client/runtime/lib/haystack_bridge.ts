#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: client/runtime/lib (thin bridge over core/layer0/ops haystack-bridge)

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'haystack_bridge', 'haystack-bridge', {
  preferLocalCore: true
});

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command, payload = {}, opts = {}) {
  const args = [command, `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`];
  if (payload && payload.state_path) args.push(`--state-path=${String(payload.state_path)}`);
  if (payload && payload.history_path) args.push(`--history-path=${String(payload.history_path)}`);
  if (payload && payload.swarm_state_path) args.push(`--swarm-state-path=${String(payload.swarm_state_path)}`);
  const out = bridge.run(args);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object' ? receipt.payload : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `haystack_bridge_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `haystack_bridge_${command}_failed`);
    return { ok: false, error: message || `haystack_bridge_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `haystack_bridge_${command}_bridge_failed`
      : `haystack_bridge_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

const status = (opts = {}) => invoke('status', opts);
const registerPipeline = (payload) => invoke('register-pipeline', payload);
const runPipeline = (payload) => invoke('run-pipeline', payload);
const runAgentToolset = (payload) => invoke('run-agent-toolset', payload);
const registerTemplate = (payload) => invoke('register-template', payload);
const renderTemplate = (payload) => invoke('render-template', payload);
const registerDocumentStore = (payload) => invoke('register-document-store', payload);
const retrieveDocuments = (payload) => invoke('retrieve-documents', payload);
const routeAndRank = (payload) => invoke('route-and-rank', payload);
const recordMultimodalEval = (payload) => invoke('record-multimodal-eval', payload);
const traceRun = (payload) => invoke('trace-run', payload);
const importConnector = (payload) => invoke('import-connector', payload);
const assimilateIntake = (payload) => invoke('assimilate-intake', payload);

module.exports = {
  status,
  registerPipeline,
  runPipeline,
  runAgentToolset,
  registerTemplate,
  renderTemplate,
  registerDocumentStore,
  retrieveDocuments,
  routeAndRank,
  recordMultimodalEval,
  traceRun,
  importConnector,
  assimilateIntake,
};
