#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const partial = require(path.join(ROOT, 'client/cognition/orchestration/partial.ts'));

function main() {
  const out = partial.retrievePartialResults({
    task_id: 'partial-session-task',
    session_history: [
      {
        session_id: 'session-1',
        items_completed: 3,
        partial_results: [
          { item_id: 'V6-SEC-010', severity: 'high' }
        ]
      }
    ]
  });

  assert.strictEqual(out.ok, true);
  assert.strictEqual(out.source, 'session_history');
  assert.strictEqual(out.items_completed, 3);
  assert.strictEqual(out.findings_sofar.length, 1);
  assert.strictEqual(out.decision, 'continue');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_partial_session_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
