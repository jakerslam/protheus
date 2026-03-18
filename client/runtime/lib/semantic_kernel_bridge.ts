#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: client/runtime/lib (thin bridge over core/layer0/ops semantic-kernel-bridge)

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'semantic_kernel_bridge', 'semantic-kernel-bridge', {
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
      : (out && out.stderr ? String(out.stderr).trim() : `semantic_kernel_bridge_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `semantic_kernel_bridge_${command}_failed`);
    return { ok: false, error: message || `semantic_kernel_bridge_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `semantic_kernel_bridge_${command}_bridge_failed`
      : `semantic_kernel_bridge_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

const status = (opts = {}) => invoke('status', opts);
const registerService = (payload) => invoke('register-service', payload);
const registerPlugin = (payload) => invoke('register-plugin', payload);
const invokePlugin = (payload) => invoke('invoke-plugin', payload);
const collaborate = (payload) => invoke('collaborate', payload);
const plan = (payload) => invoke('plan', payload);
const registerVectorConnector = (payload) => invoke('register-vector-connector', payload);
const retrieve = (payload) => invoke('retrieve', payload);
const registerLlmConnector = (payload) => invoke('register-llm-connector', payload);
const routeLlm = (payload) => invoke('route-llm', payload);
const validateStructuredOutput = (payload) => invoke('validate-structured-output', payload);
const emitEnterpriseEvent = (payload) => invoke('emit-enterprise-event', payload);
const registerDotnetBridge = (payload) => invoke('register-dotnet-bridge', payload);
const invokeDotnetBridge = (payload) => invoke('invoke-dotnet-bridge', payload);

module.exports = {
  status,
  registerService,
  registerPlugin,
  invokePlugin,
  collaborate,
  plan,
  registerVectorConnector,
  retrieve,
  registerLlmConnector,
  routeLlm,
  validateStructuredOutput,
  emitEnterpriseEvent,
  registerDotnetBridge,
  invokeDotnetBridge,
};
