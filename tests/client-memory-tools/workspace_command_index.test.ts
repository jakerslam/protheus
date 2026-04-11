#!/usr/bin/env node
'use strict';

import assert from 'node:assert';
import { readFileSync } from 'node:fs';
import {
  collectWorkspaceCommandIndex,
  filterIndex,
} from '../../client/runtime/systems/ops/workspace_command_index';

async function run(): Promise<void> {
  const payload = collectWorkspaceCommandIndex();
  const pkg = JSON.parse(readFileSync('package.json', 'utf8')) as {
    scripts?: Record<string, string>;
  };
  const expectedScriptCount = Object.keys(pkg.scripts || {}).length;
  assert.equal(payload.ok, true, 'workspace command index should return ok');
  assert.ok(
    payload.summary.total_scripts === expectedScriptCount,
    'workspace command index should report the current package script surface exactly',
  );
  assert.equal(
    payload.canonical_paths.local_dev?.script,
    'workspace:dev',
    'local dev canonical path should point to workspace:dev',
  );
  assert.equal(
    payload.canonical_paths.verify?.script,
    'workspace:verify',
    'verify canonical path should point to workspace:verify',
  );
  assert.equal(
    payload.canonical_paths.ci?.script,
    'workspace:ci',
    'ci canonical path should point to workspace:ci',
  );

  const opsOnly = filterIndex(payload, 'ops', '');
  assert.ok(
    opsOnly.namespaces.length === 1 && opsOnly.namespaces[0]?.name === 'ops',
    'namespace filter should isolate the requested namespace',
  );
  assert.ok(
    opsOnly.namespaces[0].scripts.some((entry) => entry.name === 'ops:arch:conformance'),
    'ops namespace should include architecture conformance guard',
  );

  const workspaceSearch = filterIndex(payload, '', 'workspace:');
  assert.ok(
    workspaceSearch.namespaces.some((entry) =>
      entry.scripts.some((script) => script.name === 'workspace:commands'),
    ),
    'search filter should retain workspace command index entrypoints',
  );

  process.stdout.write(
    `${JSON.stringify({ ok: true, type: 'workspace_command_index_test' }, null, 2)}\n`,
  );
}

run().catch((error) => {
  process.stderr.write(
    `${String(error && (error as Error).stack ? (error as Error).stack : error)}\n`,
  );
  process.exit(1);
});
