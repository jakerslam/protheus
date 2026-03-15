#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const SCHEMA_PATH = path.join(ROOT, 'client/cognition/orchestration/schemas/finding-v1.json');
const schemaRuntime = require(path.join(ROOT, 'client/cognition/orchestration/schema_runtime.ts'));

function main() {
  const schema = JSON.parse(fs.readFileSync(SCHEMA_PATH, 'utf8'));
  const required = new Set(schema.required || []);
  for (const field of ['audit_id', 'item_id', 'severity', 'status', 'location', 'evidence', 'timestamp']) {
    assert(required.has(field), `missing required field: ${field}`);
  }
  assert.deepStrictEqual(schema.properties.severity.enum, ['critical', 'high', 'medium', 'low', 'info']);

  const valid = schemaRuntime.validateFinding({
    audit_id: 'audit-1',
    item_id: 'item-1',
    severity: 'high',
    status: 'open',
    location: '/tmp/file.ts:10',
    evidence: [{ type: 'receipt', value: 'abc' }],
    timestamp: new Date().toISOString()
  });
  assert.strictEqual(valid.ok, true);

  const invalid = schemaRuntime.validateFinding({
    audit_id: 'audit-1',
    item_id: 'item-1',
    severity: 'fatal',
    status: 'open',
    location: '/tmp/file.ts:10',
    evidence: [{ type: 'receipt', value: 'abc' }],
    timestamp: new Date().toISOString()
  });
  assert.strictEqual(invalid.ok, false);
  assert.strictEqual(invalid.reason_code, 'finding_invalid_severity');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_schema_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
