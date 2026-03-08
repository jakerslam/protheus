#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');

function run(args, cwd) {
  return spawnSync('node', ['scripts/add_contributors_from_csv.js', ...args], {
    cwd,
    encoding: 'utf8'
  });
}

(function main() {
  const repoRoot = path.resolve(__dirname, '../../../..');
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'protheus-contrib-test-'));
  fs.cpSync(repoRoot, tmp, { recursive: true, filter: (src) => !src.includes(`${path.sep}.git${path.sep}`) });

  const csvPath = path.join(tmp, 'contributors.csv');
  fs.writeFileSync(
    csvPath,
    [
      'github_username,role,consent_token,name',
      'alice-dev,code;doc,token-a,Alice',
      'bob-ops,infra,token-b,Bob'
    ].join('\n') + '\n'
  );

  const ok = run([`--csv=${csvPath}`], tmp);
  assert.strictEqual(ok.status, 0, ok.stderr || ok.stdout);

  const rcPath = path.join(tmp, '.all-contributorsrc');
  const manifestPath = path.join(tmp, 'docs/client/community/contributors_manifest.json');

  assert.ok(fs.existsSync(rcPath), 'missing .all-contributorsrc');
  assert.ok(fs.existsSync(manifestPath), 'missing contributors manifest');

  const rc = JSON.parse(fs.readFileSync(rcPath, 'utf8'));
  const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));

  assert.strictEqual(rc.contributors.length, 2, 'unexpected contributor count in rc');
  assert.strictEqual(manifest.contributor_count, 2, 'unexpected contributor count in manifest');
  assert.strictEqual(manifest.contributors[0].login, 'alice-dev', 'contributors should be sorted deterministically');

  const dupCsv = path.join(tmp, 'dup.csv');
  fs.writeFileSync(
    dupCsv,
    [
      'github_username,role,consent_token',
      'alice-dev,code,token-a',
      'alice-dev,doc,token-b'
    ].join('\n') + '\n'
  );

  const dup = run([`--csv=${dupCsv}`], tmp);
  assert.notStrictEqual(dup.status, 0, 'duplicate usernames should fail');

  console.log('ok add_contributors_from_csv');
})();
