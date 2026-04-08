#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: client/runtime/lib (thin bridge over core/layer0/ops mastra-bridge)

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'mastra_bridge', 'mastra-bridge', {
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
  if (payload && payload.approval_queue_path) args.push(`--approval-queue-path=${String(payload.approval_queue_path)}`);
  const out = bridge.run(args);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object' ? receipt.payload : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `mastra_bridge_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `mastra_bridge_${command}_failed`);
    return { ok: false, error: message || `mastra_bridge_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `mastra_bridge_${command}_bridge_failed`
      : `mastra_bridge_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

const status = (opts = {}) => invoke('status', opts);
const registerGraph = (payload) => invoke('register-graph', payload);
const executeGraph = (payload) => invoke('execute-graph', payload);
const runAgentLoop = (payload) => invoke('run-agent-loop', payload);
const memoryRecall = (payload) => invoke('memory-recall', payload);
const suspendRun = (payload) => invoke('suspend-run', payload);
const resumeRun = (payload) => invoke('resume-run', payload);
const registerMcpBridge = (payload) => invoke('register-mcp-bridge', payload);
const invokeMcpBridge = (payload) => invoke('invoke-mcp-bridge', payload);
const recordEvalTrace = (payload) => invoke('record-eval-trace', payload);
const deployShell = (payload) => invoke('deploy-shell', payload);
const registerRuntimeBridge = (payload) => invoke('register-runtime-bridge', payload);
const routeModel = (payload) => invoke('route-model', payload);
const scaffoldIntake = (payload) => invoke('scaffold-intake', payload);
const runGovernedWorkflow = (payload) => invoke('run-governed-workflow', payload);

module.exports = {
  status,
  registerGraph,
  executeGraph,
  runAgentLoop,
  memoryRecall,
  suspendRun,
  resumeRun,
  registerMcpBridge,
  invokeMcpBridge,
  recordEvalTrace,
  deployShell,
  registerRuntimeBridge,
  routeModel,
  scaffoldIntake,
  runGovernedWorkflow,
};
