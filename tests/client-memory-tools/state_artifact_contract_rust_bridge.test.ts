#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const ts = require('typescript');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

if (!require.extensions['.ts']) {
  require.extensions['.ts'] = function compileTs(module, filename) {
    const source = fs.readFileSync(filename, 'utf8');
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        esModuleInterop: true,
        allowSyntheticDefaultImports: true
      },
      fileName: filename,
      reportDiagnostics: false
    }).outputText;
    module._compile(transpiled, filename);
  };
}

const ROOT = path.resolve(__dirname, '../..');
process.env.INFRING_OPS_USE_PREBUILT = '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';

const mod = require(path.join(ROOT, 'client/runtime/lib/state_artifact_contract.ts'));
const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'artifact-contract-'));
const latestPath = path.join(workspace, 'latest.json');
const receiptsPath = path.join(workspace, 'receipts.jsonl');
const historyPath = path.join(workspace, 'history.jsonl');
const row = mod.decorateArtifactRow({ type: 'unit' }, { schemaId: 'artifact_row', artifactType: 'receipt' });
assert.equal(row.schema_id, 'artifact_row');
const written = mod.writeArtifactSet({ latestPath, receiptsPath, historyPath }, { kind: 'alpha' }, { maxReceiptRows: 2 });
assert.equal(written.kind, 'alpha');
mod.writeArtifactSet({ receiptsPath }, { kind: 'beta' }, { maxReceiptRows: 2, writeLatest: false, appendReceipt: true });
mod.writeArtifactSet({ receiptsPath }, { kind: 'gamma' }, { maxReceiptRows: 2, writeLatest: false, appendReceipt: true });
mod.appendArtifactHistory(historyPath, { kind: 'history-only' }, { artifactType: 'history' });
mod.trimJsonlRows(receiptsPath, 2);
assert.equal(fs.existsSync(latestPath), true);
assert.equal(fs.readFileSync(receiptsPath, 'utf8').trim().split('\n').length, 2);
assert.equal(fs.readFileSync(historyPath, 'utf8').trim().split('\n').length >= 2, true);
assert.match(mod.nowIso(), /^20\d\d-/);
assertNoPlaceholderOrPromptLeak({ row, written }, 'state_artifact_contract_rust_bridge_test');\nassertStableToolingEnvelope(written, 'state_artifact_contract_rust_bridge_test');\nconsole.log(JSON.stringify({ ok: true, type: 'state_artifact_contract_rust_bridge_test' }));
