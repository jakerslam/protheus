#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const ts = require('typescript');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

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

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'action-receipts-rust-'));
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/runtime/lib/action_receipts.ts'));
  assert.match(mod.nowIso(), /^20\d\d-/);

  const receiptPath = path.join(workspace, 'receipts.jsonl');
  const withContract = mod.withReceiptContract({ type: 'unit' }, { attempted: true, verified: false });
  assert.equal(withContract.receipt_contract.recorded, true);

  const first = mod.writeContractReceipt(receiptPath, { type: 'unit', run: 1 }, { attempted: true, verified: false });
  const second = mod.writeContractReceipt(receiptPath, { type: 'unit', run: 2 }, { attempted: true, verified: true });
  assert.equal(first.receipt_contract.integrity.seq, 1);
  assert.equal(second.receipt_contract.integrity.seq, 2);
  assert.equal(second.receipt_contract.verified, true);
  assert.equal(fs.existsSync(`${receiptPath}.chain.json`), true);

  mod.appendJsonl(receiptPath, { ok: true, type: 'manual_append' });
  const rows = fs.readFileSync(receiptPath, 'utf8').trim().split('\n');
  assert.equal(rows.length >= 3, true);

  assertNoPlaceholderOrPromptLeak({ withContract, first, second, rows }, 'action_receipts_rust_bridge_test');
  assertStableToolingEnvelope(second, 'action_receipts_rust_bridge_test');
  console.log(JSON.stringify({ ok: true, type: 'action_receipts_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
