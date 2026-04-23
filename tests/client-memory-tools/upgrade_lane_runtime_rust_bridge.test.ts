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

function captureLaneRun(runFn, argv) {
  const originalArgv = process.argv.slice();
  const originalWrite = process.stdout.write.bind(process.stdout);
  const originalExit = process.exit;
  let output = '';
  process.argv = ['node', 'lane'].concat(argv);
  process.stdout.write = (chunk, encoding, cb) => {
    output += String(chunk == null ? '' : chunk);
    if (typeof encoding === 'function') encoding();
    if (typeof cb === 'function') cb();
    return true;
  };
  process.exit = (code) => {
    const error = new Error(`EXIT:${code}`);
    error.code = code;
    throw error;
  };
  try {
    runFn();
  } catch (error) {
    if (!String(error && error.message || '').startsWith('EXIT:')) throw error;
  } finally {
    process.argv = originalArgv;
    process.stdout.write = originalWrite;
    process.exit = originalExit;
  }
  return JSON.parse(output.trim());
}

function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'upgrade-lane-rust-'));
  const policyPath = path.join(workspace, 'client', 'runtime', 'config', 'core_profile_policy.json');
  fs.mkdirSync(path.dirname(policyPath), { recursive: true });
  fs.writeFileSync(policyPath, JSON.stringify({
    enabled: true,
    strict_default: true
  }, null, 2));

  process.env.INFRING_WORKSPACE = workspace;
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client', 'runtime', 'lib', 'upgrade_lane_runtime.ts'));

  const opts = {
    lane_id: 'V3-RACE-169',
    script_rel: 'packages/infring-core/core_profile_contract.js',
    policy_path: policyPath,
    stream: 'core.profiles',
    paths: {
      memory_dir: 'client/runtime/local/memory/core_profiles',
      adaptive_index_path: 'client/cognition/adaptive/core_profiles/index.json',
      events_path: 'client/runtime/local/state/core/profiles/events.jsonl',
      latest_path: 'client/runtime/local/state/core/profiles/latest.json',
      receipts_path: 'client/runtime/local/state/core/profiles/receipts.jsonl'
    },
    handlers: {
      bootstrap(_policy, args, ctx) {
        return ctx.cmdRecord({}, {
          ...args,
          action: 'bootstrap',
          event: 'core_profile_bootstrap',
          payload_json: JSON.stringify({
            mode: String(args.mode || 'lite')
          })
        });
      }
    }
  };

  const bootstrap = captureLaneRun(() => mod.runStandardLane(opts), ['bootstrap', '--owner=jay', '--mode=lite']);
  assert.equal(bootstrap.ok, true);
  assert.equal(bootstrap.event, 'core_profile_bootstrap');

  const status = captureLaneRun(() => mod.runStandardLane(opts), ['status']);
  assert.equal(status.ok, true);
  assert.equal(status.artifacts.latest_path, 'client/runtime/local/state/core/profiles/latest.json');

  assertNoPlaceholderOrPromptLeak({ bootstrap, status }, 'upgrade_lane_runtime_rust_bridge_test');
  assertStableToolingEnvelope(status, 'upgrade_lane_runtime_rust_bridge_test');
  const latestPath = path.join(workspace, 'client', 'runtime', 'local', 'state', 'core', 'profiles', 'latest.json');
  const adaptiveIndex = path.join(workspace, 'client', 'cognition', 'adaptive', 'core_profiles', 'index.json');
  assert.equal(fs.existsSync(latestPath), true);
  assert.equal(fs.existsSync(adaptiveIndex), true);

  console.log(JSON.stringify({ ok: true, type: 'upgrade_lane_runtime_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
