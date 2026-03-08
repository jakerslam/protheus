#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');

(function main() {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'protheus-verify-empty-fort-'));
  const manifestPath = path.join(tmp, 'manifest.json');
  const readmePath = path.join(tmp, 'README.md');
  const rcPath = path.join(tmp, '.all-contributorsrc');

  const contributors = [
    { login: 'alice-dev', consent_token: 'token-a', email: 'alice-dev@users.noreply.github.com' },
    { login: 'bob-ops', consent_token: 'token-b', email: 'bob-ops@users.noreply.github.com' }
  ];

  fs.writeFileSync(manifestPath, JSON.stringify({ contributors }, null, 2));
  fs.writeFileSync(rcPath, JSON.stringify({ contributors: contributors.map((c) => ({ login: c.login, contributions: ['code'] })) }, null, 2));
  fs.writeFileSync(
    readmePath,
    '# x\\n\\n<!-- EMPTY_FORT:START -->\\n' +
      `Claims in this section are generated from \`${manifestPath}\`.\\n` +
      '<!-- EMPTY_FORT:END -->\\n'
  );

  const repoRoot = path.resolve(__dirname, '../../../..');
  const ok = spawnSync('bash', [
    path.join(repoRoot, 'scripts/verify-empty-fort.sh'),
    `--manifest=${manifestPath}`,
    `--contributorsrc=${rcPath}`,
    `--readme=${readmePath}`,
    '--min-count=2'
  ], { encoding: 'utf8' });

  assert.strictEqual(ok.status, 0, ok.stderr || ok.stdout);

  const bad = spawnSync('bash', [
    path.join(repoRoot, 'scripts/verify-empty-fort.sh'),
    `--manifest=${manifestPath}`,
    `--contributorsrc=${rcPath}`,
    `--readme=${readmePath}`,
    '--min-count=3'
  ], { encoding: 'utf8' });
  assert.notStrictEqual(bad.status, 0, 'min-count gate should fail when threshold unmet');

  console.log('ok verify_empty_fort');
})();
