#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const ts = require('typescript');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '..', '..');
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
const {
  DIRECTIVES_DIR,
  loadActiveDirectives,
  mergeConstraints,
  parseYaml,
  validateAction,
  validateTier1DirectiveQuality
} = require(path.join(ROOT, 'client', 'runtime', 'lib', 'directive_resolver.ts'));

assert(DIRECTIVES_DIR.endsWith(path.join('client', 'runtime', 'config', 'directives')));

const directives = loadActiveDirectives();
assert(Array.isArray(directives), 'expected directive array');
assert(directives.length >= 1, 'expected active directives');

const merged = mergeConstraints(directives);
assert(merged.high_stakes_domains instanceof Set, 'expected high_stakes_domains set');
assert(merged.high_stakes_domains.has('finance'), 'expected finance domain');

const parsed = parseYaml(`
metadata:
  id: demo
intent:
  primary: "test"
`);
assert.strictEqual(parsed.metadata.id, 'demo');
assert.strictEqual(parsed.intent.primary, 'test');

const weakTier1 = validateTier1DirectiveQuality(
  `
metadata:
  id: weak
intent:
  primary: "test"
`,
  'weak'
);
assert.strictEqual(weakTier1.ok, false);
assert(weakTier1.missing.includes('intent.definitions_timebound'));

const blocked = validateAction({
  action_id: 'act_secret',
  tier: 2,
  type: 'other',
  summary: 'inspect payload',
  risk: 'low',
  payload: {
    token: 'moltbook_sk_1234567890123456789012345'
  }
});
assert.strictEqual(blocked.allowed, false);
assert(/redacted/i.test(String(blocked.blocked_reason || '')));

const approval = validateAction({
  action_id: 'act_rm',
  tier: 2,
  type: 'other',
  summary: 'cleanup deployment',
  risk: 'low',
  payload: {},
  metadata: {
    command_text: 'rm -rf /tmp/demo'
  }
});
assert.strictEqual(approval.allowed, true);
assert.strictEqual(approval.requires_approval, true);

assertNoPlaceholderOrPromptLeak({ directives, merged, parsed, weakTier1, blocked, approval }, 'directive_resolver_rust_bridge_test');\nassertStableToolingEnvelope({ blocked, approval }, 'directive_resolver_rust_bridge_test');\nconsole.log('directive_resolver_rust_bridge.test.ts: OK');
