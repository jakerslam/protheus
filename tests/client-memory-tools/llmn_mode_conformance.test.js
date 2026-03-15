#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');
const legacyAliasAdapter = require(path.join(ROOT, 'client/runtime/systems/compat/legacy_alias_adapter.ts'));
const policyValidator = require(path.join(ROOT, 'client/runtime/systems/memory/policy_validator.ts'));

function main() {
  // REQ-35 path alias normalization should resolve legacy wrappers to canonical runtime lanes.
  const aliasLane = legacyAliasAdapter.resolveLane(
    '',
    path.join(ROOT, 'client/runtime/systems/memory/memory_index_freshness_gate.ts')
  );
  assert.strictEqual(
    aliasLane,
    'RUNTIME-SYSTEMS-MEMORY-MEMORY_INDEX_FRESHNESS_GATE',
    'legacy alias adapter must map to canonical runtime lane IDs'
  );

  const compatLane = legacyAliasAdapter.laneFromAliasRel('systems/memory/policy_validator.ts');
  assert.strictEqual(
    compatLane,
    'RUNTIME-SYSTEMS-MEMORY-POLICY_VALIDATOR',
    'alias-relative paths must normalize to deterministic runtime lane IDs'
  );

  // REQ-35 mode conformance now enforces client-side policy gates before any Rust-core execution path.
  const bypassAttempt = policyValidator.validateMemoryPolicy([
    'query-index',
    '--session-id=llmn-conformance-session',
    '--allow-full-scan=1'
  ]);
  assert.strictEqual(bypassAttempt.ok, false);
  assert.strictEqual(bypassAttempt.reason_code, 'index_first_bypass_forbidden');

  // Guard against silent configuration drift: REQ-35 backlog rows must stay present with test evidence.
  const registryPath = path.join(ROOT, 'client/runtime/config/backlog_registry.json');
  const registry = JSON.parse(fs.readFileSync(registryPath, 'utf8'));
  const rows = Array.isArray(registry)
    ? registry
    : Array.isArray(registry.rows)
      ? registry.rows
      : [];
  assert(rows.length > 0, 'backlog registry rows must be present');
  const req35Rows = rows.filter((row) => /^V6-LLMN-00[34]$/.test(String(row && row.id ? row.id : '')));
  assert.strictEqual(req35Rows.length, 2, 'expected V6-LLMN-003 and V6-LLMN-004 backlog rows');
  const acceptanceBlob = req35Rows
    .map((row) => String(row && row.acceptance ? row.acceptance : ''))
    .join('\n')
    .toLowerCase();
  assert(
    acceptanceBlob.includes('llmn_mode_conformance'),
    'REQ-35 acceptance rows must include llmn conformance evidence linkage'
  );
  assert(
    acceptanceBlob.includes('legacy_path_alias_adapters'),
    'REQ-35 acceptance rows must include legacy path alias adapter evidence linkage'
  );

  console.log(JSON.stringify({ ok: true, type: 'llmn_mode_conformance_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
