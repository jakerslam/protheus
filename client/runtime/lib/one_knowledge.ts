#!/usr/bin/env node
'use strict';
export {};

// Layer ownership: client/runtime/lib (thin bridge over core/layer2/ops public-api-catalog)

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_USE_PREBUILT =
  process.env.INFRING_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS =
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'one_knowledge', 'public-api-catalog', {
  preferLocalCore: true
});

function pushFlag(args, key, value) {
  if (value === undefined || value === null) return;
  const text = String(value).trim();
  if (!text) return;
  args.push(`--${key}=${text}`);
}

function pushJsonFlag(args, key, value) {
  if (value === undefined || value === null) return;
  args.push(`--${key}=${JSON.stringify(value)}`);
}

function payloadFromOut(out) {
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  return receipt && receipt.payload && typeof receipt.payload === 'object' ? receipt.payload : receipt;
}

function invoke(command, args = [], opts = {}) {
  const out = bridge.run([command].concat(Array.isArray(args) ? args : []));
  const payload = payloadFromOut(out);
  if (out.status !== 0) {
    const message = payload && typeof payload.error === 'string'
      ? payload.error
      : (out && out.stderr ? String(out.stderr).trim() : `one_knowledge_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `one_knowledge_${command}_failed`);
    return { ok: false, error: message || `one_knowledge_${command}_failed` };
  }
  if (!payload || typeof payload !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `one_knowledge_${command}_bridge_failed`
      : `one_knowledge_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payload;
}

function status(opts = {}) {
  const args = [];
  pushFlag(args, 'state-path', opts.state_path);
  pushFlag(args, 'policy', opts.policy);
  return invoke('status', args, opts);
}

function sync(payload = {}, opts = {}) {
  const args = [];
  pushFlag(args, 'state-path', payload.state_path);
  pushFlag(args, 'policy', payload.policy);
  pushFlag(args, 'catalog-path', payload.catalog_path);
  pushJsonFlag(args, 'catalog-json', payload.catalog_json);
  pushFlag(args, 'source', payload.source);
  pushFlag(args, 'strict', payload.strict);
  return invoke('sync', args, opts);
}

function search(payload = {}, opts = {}) {
  const args = [];
  pushFlag(args, 'state-path', payload.state_path);
  pushFlag(args, 'query', payload.query);
  pushFlag(args, 'limit', payload.limit);
  return invoke('search', args, opts);
}

function integrate(payload = {}, opts = {}) {
  const args = [];
  pushFlag(args, 'state-path', payload.state_path);
  pushFlag(args, 'action-id', payload.action_id || payload.id);
  pushFlag(args, 'strict', payload.strict);
  return invoke('integrate', args, opts);
}

function connect(payload = {}, opts = {}) {
  const args = [];
  pushFlag(args, 'state-path', payload.state_path);
  pushFlag(args, 'platform', payload.platform);
  pushFlag(args, 'connection-key', payload.connection_key);
  pushFlag(args, 'access-token', payload.access_token);
  pushFlag(args, 'refresh-token', payload.refresh_token);
  pushFlag(args, 'expires-epoch-ms', payload.expires_epoch_ms);
  pushFlag(args, 'oauth-passthrough', payload.oauth_passthrough);
  pushJsonFlag(args, 'metadata-json', payload.metadata_json || payload.metadata);
  return invoke('connect', args, opts);
}

function importFlow(payload = {}, opts = {}) {
  const args = [];
  pushFlag(args, 'state-path', payload.state_path);
  pushFlag(args, 'flow-path', payload.flow_path);
  pushJsonFlag(args, 'flow-json', payload.flow_json);
  pushFlag(args, 'workflow-id', payload.workflow_id);
  pushFlag(args, 'strict', payload.strict);
  return invoke('import-flow', args, opts);
}

function runFlow(payload = {}, opts = {}) {
  const args = [];
  pushFlag(args, 'state-path', payload.state_path);
  pushFlag(args, 'flow-path', payload.flow_path);
  pushFlag(args, 'workflow-id', payload.workflow_id);
  pushJsonFlag(args, 'input-json', payload.input_json);
  pushFlag(args, 'strict', payload.strict);
  return invoke('run-flow', args, opts);
}

function verify(payload = {}, opts = {}) {
  const args = [];
  pushFlag(args, 'state-path', payload.state_path);
  pushFlag(args, 'max-age-days', payload.max_age_days);
  pushFlag(args, 'strict', payload.strict);
  return invoke('verify', args, opts);
}

function runCli(args = process.argv.slice(2)) {
  const out = bridge.run(Array.isArray(args) ? args : []);
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
  return Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
}

if (require.main === module) {
  process.exit(runCli(process.argv.slice(2)));
}

module.exports = {
  status,
  sync,
  search,
  integrate,
  connect,
  importFlow,
  runFlow,
  verify,
  runCli
};
