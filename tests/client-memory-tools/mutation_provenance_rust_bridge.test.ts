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
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'mutation-prov-rust-'));
  const policyPath = path.join(workspace, 'client', 'runtime', 'config', 'mutation_provenance_policy.json');
  fs.mkdirSync(path.dirname(policyPath), { recursive: true });
  fs.writeFileSync(policyPath, JSON.stringify({
    version: '2.0',
    channels: {
      adaptive: {
        allowed_source_prefixes: ['systems/adaptive/', 'lib/'],
        require_reason: true
      }
    }
  }, null, 2));

  process.env.PROTHEUS_WORKSPACE_ROOT = workspace;
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/lib/mutation_provenance.ts'));
  const policy = mod.loadPolicy();
  assert.equal(policy.version, '2.0');

  const sourcePath = path.join(workspace, 'client', 'runtime', 'systems', 'adaptive', 'planner.ts');
  const normalized = mod.normalizeMeta({ source: sourcePath }, '', 'sync');
  assert.equal(normalized.source, 'systems/adaptive/planner.ts');
  assert.equal(normalized.reason, 'sync');

  const pass = mod.enforceMutationProvenance('adaptive', {
    source: sourcePath,
    reason: 'sync',
    actor: 'tester'
  });
  assert.equal(pass.ok, true);
  assert.equal(pass.source_rel, 'systems/adaptive/planner.ts');

  const softFail = mod.enforceMutationProvenance('adaptive', {
    source: path.join(workspace, 'bad.ts'),
    reason: ''
  }, { strict: false, context: 'unit' });
  assert.equal(softFail.ok, false);
  assert.deepEqual(softFail.violations, ['source_not_allowlisted', 'missing_reason']);

  let strictBlocked = false;
  try {
    mod.enforceMutationProvenance('adaptive', {
      source: path.join(workspace, 'bad.ts'),
      reason: ''
    }, { strict: true });
  } catch (error) {
    strictBlocked = /mutation_provenance_blocked:adaptive:/.test(String(error.message || error));
  }
  assert.equal(strictBlocked, true);

  mod.recordMutationAudit('adaptive', { type: 'bridge_test' });
  const auditPath = path.join(workspace, 'client', 'local', 'state', 'security', 'adaptive_mutations.jsonl');
  const violationPath = path.join(workspace, 'client', 'local', 'state', 'security', 'adaptive_mutation_violations.jsonl');
  assert.equal(fs.existsSync(auditPath), true);
  assert.equal(fs.existsSync(violationPath), true);

  assertNoPlaceholderOrPromptLeak({ policy, normalized, pass, softFail }, 'mutation_provenance_rust_bridge_test');
  assertStableToolingEnvelope(pass, 'mutation_provenance_rust_bridge_test');
  console.log(JSON.stringify({ ok: true, type: 'mutation_provenance_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
