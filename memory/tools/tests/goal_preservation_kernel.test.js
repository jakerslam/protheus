#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');

function ensureDir(p) {
  if (!fs.existsSync(p)) fs.mkdirSync(p, { recursive: true });
}

function writeJson(filePath, value) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2), 'utf8');
}

function runNode(cwd, args) {
  return spawnSync('node', args, { cwd, encoding: 'utf8', env: process.env });
}

function parseJson(out) {
  const lines = String(out || '').trim().split('\n').map((row) => row.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {
      // continue
    }
  }
  return null;
}

function run() {
  const repoRoot = path.resolve(__dirname, '..', '..', '..');
  const scriptPath = path.join(repoRoot, 'systems', 'security', 'goal_preservation_kernel.js');
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'goal-preservation-'));
  const constitution = path.join(tmp, 'constitution.md');
  fs.writeFileSync(constitution, '# Constitution\nPreserve user sovereignty\n', 'utf8');

  const policyPath = path.join(tmp, 'goal_policy.json');
  writeJson(policyPath, {
    version: '1.0',
    strict_mode: true,
    constitution_path: constitution,
    protected_axiom_markers: ['user sovereignty'],
    blocked_mutation_paths: ['^systems/security/guard\\.(ts|js)$'],
    output: {
      state_path: path.join(tmp, 'state', 'latest.json'),
      receipts_path: path.join(tmp, 'state', 'receipts.jsonl')
    }
  });

  const blockedProposalPath = path.join(tmp, 'blocked_proposal.json');
  writeJson(blockedProposalPath, {
    proposal_id: 'p_blocked',
    mutation_paths: ['systems/security/guard.ts'],
    summary: 'disable user sovereignty checks'
  });

  const blocked = runNode(repoRoot, [scriptPath, 'evaluate', `--proposal-file=${blockedProposalPath}`, `--policy=${policyPath}`]);
  assert.strictEqual(blocked.status, 0, blocked.stderr || 'blocked proposal should return payload');
  const blockedPayload = parseJson(blocked.stdout);
  assert.strictEqual(blockedPayload.allowed, false, 'blocked proposal must be denied');
  assert.ok(Array.isArray(blockedPayload.reasons) && blockedPayload.reasons.includes('blocked_mutation_path'));

  const safeProposalPath = path.join(tmp, 'safe_proposal.json');
  writeJson(safeProposalPath, {
    proposal_id: 'p_safe',
    mutation_paths: ['systems/weaver/weaver_core.ts'],
    summary: 'improve value arbitration logging only'
  });

  const allowed = runNode(repoRoot, [scriptPath, 'evaluate', `--proposal-file=${safeProposalPath}`, `--policy=${policyPath}`]);
  assert.strictEqual(allowed.status, 0, allowed.stderr || 'safe proposal should return payload');
  const allowedPayload = parseJson(allowed.stdout);
  assert.strictEqual(allowedPayload.allowed, true, `safe proposal should be allowed: ${JSON.stringify(allowedPayload.reasons || [])}`);

  console.log('goal_preservation_kernel.test.js: OK');
}

try {
  run();
} catch (err) {
  console.error(`goal_preservation_kernel.test.js: FAIL: ${err && err.stack ? err.stack : err.message}`);
  process.exit(1);
}
