'use strict';

export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');
const { normalizeOpsBridgeEnvAliases } = require('./queued_backlog_runtime.ts');

normalizeOpsBridgeEnvAliases();
process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_USE_PREBUILT =
  process.env.INFRING_OPS_USE_PREBUILT || process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS =
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'state_artifact_contract', 'state-artifact-contract-kernel');

type AnyObj = Record<string, any>;
type ArtifactOptions = {
  schemaId?: string,
  schemaVersion?: string,
  artifactType?: string,
  writeLatest?: boolean,
  appendReceipt?: boolean,
  maxReceiptRows?: number
};

function encodeBase64(value: unknown) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command: string, payload: Record<string, unknown> = {}, opts: Record<string, unknown> = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `state_artifact_contract_kernel_${command}_failed`);
    if (opts && opts.throwOnError === false) return { ok: false, error: message || `state_artifact_contract_kernel_${command}_failed` };
    throw new Error(message || `state_artifact_contract_kernel_${command}_failed`);
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `state_artifact_contract_kernel_${command}_bridge_failed`
      : `state_artifact_contract_kernel_${command}_bridge_failed`;
    if (opts && opts.throwOnError === false) return { ok: false, error: message };
    throw new Error(message);
  }
  return payloadOut;
}

function nowIso() {
  return String(invoke('now-iso', {}).ts || new Date().toISOString());
}

function decorateArtifactRow(payload: AnyObj, opts: ArtifactOptions = {}) {
  return invoke('decorate-artifact-row', {
    payload: payload && typeof payload === 'object' ? payload : {},
    options: opts && typeof opts === 'object' ? opts : {}
  }).row;
}

function trimJsonlRows(filePath: string, maxRows: number) {
  invoke('trim-jsonl-rows', {
    file_path: String(filePath || ''),
    max_rows: Number.isFinite(Number(maxRows)) ? Math.floor(Number(maxRows)) : 0
  });
}

function writeArtifactSet(paths: {
  latestPath?: string,
  receiptsPath?: string,
  historyPath?: string
}, payload: AnyObj, opts: ArtifactOptions = {}) {
  return invoke('write-artifact-set', {
    paths: {
      latestPath: paths && paths.latestPath ? String(paths.latestPath) : undefined,
      receiptsPath: paths && paths.receiptsPath ? String(paths.receiptsPath) : undefined,
      historyPath: paths && paths.historyPath ? String(paths.historyPath) : undefined
    },
    payload: payload && typeof payload === 'object' ? payload : {},
    options: opts && typeof opts === 'object' ? opts : {}
  }).row;
}

function appendArtifactHistory(historyPath: string, payload: AnyObj, opts: ArtifactOptions = {}) {
  return invoke('append-artifact-history', {
    history_path: String(historyPath || ''),
    payload: payload && typeof payload === 'object' ? payload : {},
    options: opts && typeof opts === 'object' ? opts : {}
  }).row;
}

module.exports = {
  decorateArtifactRow,
  writeArtifactSet,
  appendArtifactHistory,
  trimJsonlRows,
  nowIso
};
