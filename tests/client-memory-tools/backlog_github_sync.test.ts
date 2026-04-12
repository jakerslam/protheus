#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const WRAPPER = path.join(ROOT, 'client/runtime/systems/ops/backlog_github_sync.ts');
const OPS_CARGO = path.join(ROOT, 'core/layer0/ops/Cargo.toml');

function parseJsonOutput(text) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  try {
    return JSON.parse(trimmed);
  } catch {}
  for (const line of trimmed.split('\n').reverse()) {
    const candidate = line.trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function resolveOpsBinary() {
  for (const rel of ['target/debug/protheus-ops', 'target/debug/infring-ops']) {
    const full = path.join(ROOT, rel);
    if (fs.existsSync(full)) return full;
  }
  const build = spawnSync('cargo', ['build', '--manifest-path', OPS_CARGO, '--bin', 'protheus-ops'], {
    cwd: ROOT,
    encoding: 'utf8',
  });
  assert.equal(build.status, 0, build.stderr || build.stdout || 'cargo build failed');
  const built = path.join(ROOT, 'target/debug/protheus-ops');
  assert.equal(fs.existsSync(built), true, 'expected built protheus-ops binary');
  return built;
}

function runDomain(args) {
  const binary = resolveOpsBinary();
  const proc = spawnSync(binary, ['backlog-github-sync', ...args], { cwd: ROOT, encoding: 'utf8' });
  assert.equal(proc.status, 0, proc.stderr || proc.stdout || 'backlog_github_sync failed');
  const payload = parseJsonOutput(proc.stdout) || parseJsonOutput(proc.stderr);
  assert(payload && payload.ok === true, 'expected ok backlog github sync payload');
  return payload;
}

function main() {
  const wrapperSource = fs.readFileSync(WRAPPER, 'utf8');
  assert.equal(wrapperSource.includes('backlogGithubSync'), true);

  const status = runDomain(['status']);
  const check = runDomain(['check', '--strict=0']);

  assert.equal(status.type, 'backlog_github_sync');
  assert.equal(status.command, 'status');
  assert.equal(status.lane, 'backlog_github_sync');
  assert.equal(status.replacement, 'protheus-ops backlog-github-sync');
  assert.equal(typeof status.receipt_hash, 'string');

  assert.equal(check.type, 'backlog_github_sync');
  assert.equal(check.command, 'check');
  assert.equal(check.flags.strict, '0');
  assert.equal(check.lane, 'backlog_github_sync');
  assert.equal(check.replacement, 'protheus-ops backlog-github-sync');
  assert.equal(typeof check.receipt_hash, 'string');

  console.log(JSON.stringify({ ok: true, type: 'backlog_github_sync_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
