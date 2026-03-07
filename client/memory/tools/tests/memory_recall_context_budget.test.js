#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const assert = require('assert');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'memory', 'memory_recall.js');

function mkDir(p) {
  if (!fs.existsSync(p)) fs.mkdirSync(p, { recursive: true });
}

function writeFile(p, text) {
  mkDir(path.dirname(p));
  fs.writeFileSync(p, text, 'utf8');
}

function parseJson(stdout) {
  try { return JSON.parse(String(stdout || '').trim()); } catch { return null; }
}

function makeWorkspace() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'memory-context-budget-'));
  mkDir(path.join(root, 'memory'));
  mkDir(path.join(root, 'state', 'memory', 'working_set'));

  writeFile(
    path.join(root, 'memory', 'MEMORY_INDEX.md'),
    [
      '# MEMORY_INDEX.md',
      '| node_id | uid | tags | file | summary |',
      '|---------|-----|------|------|---------|',
      '| dense-context-node | memdense001 | #memory #context | 2026-01-01.md | Dense context node for budget guard tests |',
      ''
    ].join('\n')
  );

  writeFile(
    path.join(root, 'memory', 'TAGS_INDEX.md'),
    [
      '# TAGS_INDEX.md',
      '#memory -> dense-context-node',
      '#context -> dense-context-node',
      ''
    ].join('\n')
  );

  const largeSection = Array.from({ length: 360 })
    .map((_, i) => `- dense-line-${i} token-heavy context payload for budget trimming regression coverage.`)
    .join('\n');

  writeFile(
    path.join(root, 'memory', '2026-01-01.md'),
    [
      '---',
      'date: 2026-01-01',
      'node_id: dense-context-node',
      'uid: memdense001',
      'tags: [memory, context]',
      'edges_to: []',
      '---',
      '# dense-context-node',
      largeSection,
      ''
    ].join('\n')
  );

  return root;
}

function runRecall(root, args) {
  return spawnSync('node', [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      MEMORY_RECALL_ROOT: root,
      MEMORY_RECALL_BACKEND: 'js'
    }
  });
}

function runTest(name, fn) {
  try {
    fn();
    console.log(`   OK ${name}`);
  } catch (err) {
    console.error(`   FAIL ${name}: ${err && err.message ? err.message : err}`);
    process.exitCode = 1;
  }
}

console.log('memory_recall_context_budget.test.js');

runTest('trim mode caps expanded recall payload to requested token budget', () => {
  const root = makeWorkspace();
  const out = runRecall(root, [
    'query',
    '--q=context',
    '--expand=always',
    '--excerpt-lines=120',
    '--top=1',
    '--context-budget-mode=trim',
    '--context-budget-tokens=256',
    '--session=ctxtrim'
  ]);
  assert.strictEqual(out.status, 0, out.stderr || 'trim query should pass');
  const payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'payload should be ok');
  assert.ok(payload.context_budget, 'context budget metadata should be present');
  assert.strictEqual(String(payload.context_budget.mode), 'trim');
  assert.strictEqual(Boolean(payload.context_budget.capped), true, 'trim mode should report capped payload');
  assert.ok(Number(payload.context_budget.tokens_est_after || 0) <= 256, 'trimmed payload should fit requested budget');
  assert.ok(Array.isArray(payload.hits) && payload.hits.length >= 1, 'trim mode should return at least one hit');
});

runTest('reject mode fails closed on context budget overflow', () => {
  const root = makeWorkspace();
  const out = runRecall(root, [
    'query',
    '--q=context',
    '--expand=always',
    '--excerpt-lines=120',
    '--top=1',
    '--context-budget-mode=reject',
    '--context-budget-tokens=256',
    '--session=ctxreject'
  ]);
  assert.notStrictEqual(out.status, 0, 'reject mode should fail when context exceeds budget');
  const payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === false, 'reject payload should be error');
  assert.strictEqual(String(payload.error || ''), 'context_budget_exceeded');
  assert.ok(payload.context_budget && payload.context_budget.mode === 'reject', 'reject mode should be reflected in payload');
});

if (process.exitCode && process.exitCode !== 0) {
  console.error('memory_recall_context_budget.test.js: FAIL');
  process.exit(process.exitCode);
}

console.log('memory_recall_context_budget.test.js: OK');
