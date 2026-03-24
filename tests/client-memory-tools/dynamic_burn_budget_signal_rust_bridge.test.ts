#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const ts = require('typescript');

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
const mod = require(path.join(ROOT, 'client/lib/dynamic_burn_budget_signal.ts'));
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'burn-signal-'));
const latest = path.join(dir, 'latest.json');
fs.writeFileSync(latest, JSON.stringify({ ok: true, projection: { pressure: 'high', projected_runway_days: 14, providers_available: 2, reason_codes: ['budget spike'] } }, null, 2));
assert.equal(mod.normalizeBurnPressure('HIGH'), 'high');
assert.equal(mod.pressureRank('high'), 3);
assert.equal(mod.mapPressureToCostPressure('critical'), 1);
const signal = mod.loadDynamicBurnOracleSignal({ latest_path: latest });
assert.equal(signal.available, true);
assert.equal(signal.pressure, 'high');
assert.equal(signal.providers_available, 2);
console.log(JSON.stringify({ ok: true, type: 'dynamic_burn_budget_signal_rust_bridge_test' }));
