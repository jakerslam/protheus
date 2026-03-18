#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: client/runtime/lib (thin bridge over core/layer0/ops langchain-bridge)

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'langchain_bridge', 'langchain-bridge', {
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
      : (out && out.stderr ? String(out.stderr).trim() : `langchain_bridge_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `langchain_bridge_${command}_failed`);
    return { ok: false, error: message || `langchain_bridge_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `langchain_bridge_${command}_bridge_failed`
      : `langchain_bridge_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

const status = (opts = {}) => invoke('status', opts);
const registerChain = (payload) => invoke('register-chain', payload);
const executeChain = (payload) => invoke('execute-chain', payload);
const runDeepAgent = (payload) => invoke('run-deep-agent', payload);
const registerMemoryBridge = (payload) => invoke('register-memory-bridge', payload);
const recallMemory = (payload) => invoke('recall-memory', payload);
const importIntegration = (payload) => invoke('import-integration', payload);
const routePrompt = (payload) => invoke('route-prompt', payload);
const recordTrace = (payload) => invoke('record-trace', payload);
const checkpointRun = (payload) => invoke('checkpoint-run', payload);
const assimilateIntake = (payload) => invoke('assimilate-intake', payload);

module.exports = {
  status,
  registerChain,
  executeChain,
  runDeepAgent,
  registerMemoryBridge,
  recallMemory,
  importIntegration,
  routePrompt,
  recordTrace,
  checkpointRun,
  assimilateIntake,
};
