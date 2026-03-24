#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');
const { validateSessionIsolation } = require(path.join(
  ROOT,
  'client/runtime/systems/memory/session_isolation.ts'
));

function main() {
  // V6-MEMORY-013: node/uid scoped commands must carry a valid session id.
  const missingSession = validateSessionIsolation([
    'query-index',
    '--uid=ABC123'
  ]);
  assert.strictEqual(missingSession.ok, false);
  assert.strictEqual(missingSession.reason_code, 'missing_session_id');

  const invalidSession = validateSessionIsolation([
    'query-index',
    '--uid=ABC123',
    '--session-id=*'
  ]);
  assert.strictEqual(invalidSession.ok, false);
  assert.strictEqual(invalidSession.reason_code, 'invalid_session_id');

  const statePath = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'memory-uid-isolation-')), 'state.json');
  const firstSession = validateSessionIsolation(
    ['query-index', '--uid=ABC123', '--session-id=session-alpha'],
    { statePath }
  );
  assert.strictEqual(firstSession.ok, true);

  // Cross-session access to same memory resource must fail closed.
  const secondSession = validateSessionIsolation(
    ['query-index', '--uid=ABC123', '--session-id=session-beta'],
    { statePath }
  );
  assert.strictEqual(secondSession.ok, false);
  assert.strictEqual(secondSession.reason_code, 'cross_session_leak_blocked');

  console.log(
    JSON.stringify({
      ok: true,
      type: 'memory_uid_enforcement_test'
    })
  );
}

if (require.main === module) {
  main();
}

module.exports = { main };
