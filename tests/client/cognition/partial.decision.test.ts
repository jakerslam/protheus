#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../../..');
const partial = require(path.join(ROOT, 'client/cognition/orchestration/partial.ts'));

function main() {
  const out = partial.retrievePartialResults({
    task_id: 'partial-decision-task',
    session_history: [
      {
        session_id: 'session-decision-1',
        processed_count: 1,
        output: {
          partial_results: [{ item_id: 'REQ-38-008' }]
        }
      }
    ],
    decision: 'abort'
  });

  assert.strictEqual(out.ok, true);
  assert.strictEqual(out.decision, 'abort');

  const fallback = partial.normalizeDecision('', false);
  assert.strictEqual(fallback, 'retry');

  console.log(JSON.stringify({ ok: true, type: 'orchestration_partial_decision_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
