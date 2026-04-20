#!/usr/bin/env node
'use strict';

import assert from 'node:assert';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = process.cwd();
const FIXTURE_ROOT = path.join(ROOT, 'tests/fixtures/incident_governance');

type GatePayload = {
  ok?: boolean;
  failures?: string[];
  waivers_applied?: Array<{ waiver_id?: string; check_id?: string }>;
};

function runFixture(name: string, expectOk: boolean): GatePayload {
  const fixtureDir = path.join(FIXTURE_ROOT, name);
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), `incident-governance-${name}-`));
  const outJson = path.join(tempDir, 'gate.json');
  const outMd = path.join(tempDir, 'gate.md');

  const cmd = [
    'client/runtime/lib/ts_entrypoint.ts',
    'tests/tooling/scripts/ci/incident_operations_governance_gate.ts',
    '--strict=1',
    `--policy=${path.join(fixtureDir, 'policy.json')}`,
    `--process=${path.join(fixtureDir, 'process.md')}`,
    `--owner-roster=${path.join(fixtureDir, 'owner_roster.json')}`,
    `--artifact-schema=${path.join(fixtureDir, 'artifact_schema.json')}`,
    `--waivers=${path.join(fixtureDir, 'waivers.json')}`,
    `--out-json=${outJson}`,
    `--out-markdown=${outMd}`,
  ];

  const result = spawnSync('node', cmd, { cwd: ROOT, encoding: 'utf8' });

  if (expectOk) {
    assert.equal(result.status, 0, `fixture ${name} should pass: ${result.stderr}`);
  } else {
    assert.notEqual(result.status, 0, `fixture ${name} should fail`);
  }

  assert.ok(fs.existsSync(outJson), `fixture ${name} should emit out json`);
  const payload = JSON.parse(fs.readFileSync(outJson, 'utf8')) as GatePayload;
  assert.equal(Boolean(payload.ok), expectOk, `fixture ${name} payload.ok mismatch`);
  return payload;
}

async function run(): Promise<void> {
  const pass = runFixture('pass', true);
  assert.equal((pass.failures || []).length, 0, 'pass fixture must have zero failures');

  const placeholder = runFixture('fail_placeholder_owner', false);
  assert.ok(
    (placeholder.failures || []).some((row) => row.includes('owner_roster_contract')),
    'placeholder fixture should fail owner roster contract',
  );

  const expiredWaiver = runFixture('fail_expired_waiver', false);
  assert.ok(
    (expiredWaiver.failures || []).some((row) => row.includes('waiver_contract') || row.includes('process_doc_contract')),
    'expired waiver fixture should fail waiver or process contract',
  );

  const waived = runFixture('pass_with_waiver', true);
  assert.ok(
    (waived.waivers_applied || []).some((row) => row.check_id === 'process_doc_contract'),
    'waived fixture should apply waiver to process_doc_contract',
  );

  process.stdout.write(
    `${JSON.stringify({ ok: true, type: 'incident_operations_governance_gate_test' }, null, 2)}\n`,
  );
}

run().catch((error) => {
  process.stderr.write(`${String(error && (error as Error).stack ? (error as Error).stack : error)}\n`);
  process.exit(1);
});
