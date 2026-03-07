#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'memory', 'memory_auto_recall.js');

function run(args, env) {
  const proc = spawnSync('node', [SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: { ...process.env, ...env }
  });
  return {
    status: Number.isFinite(Number(proc.status)) ? Number(proc.status) : 1,
    stdout: String(proc.stdout || '').trim(),
    stderr: String(proc.stderr || '').trim()
  };
}

function parseJson(raw) {
  const text = String(raw || '').trim();
  return text ? JSON.parse(text) : null;
}

try {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'memory-auto-recall-test-'));
  const matrixPath = path.join(tmp, 'tag_memory_matrix.json');
  const policyPath = path.join(tmp, 'memory_auto_recall_policy.json');
  const eventsPath = path.join(tmp, 'events.jsonl');
  const latestPath = path.join(tmp, 'latest.json');

  const matrix = {
    ok: true,
    type: 'tag_memory_matrix',
    generated_at: new Date().toISOString(),
    tags: [
      {
        tag: 'alpha',
        tag_priority: 90,
        node_count: 2,
        node_ids: ['node-best', 'node-alt'],
        nodes: [
          {
            node_id: 'node-best',
            tags: ['alpha', 'beta'],
            priority_score: 88,
            recency_score: 0.9,
            dream_score: 0.4,
            level_token: 'node1',
            date: '2026-03-07'
          },
          {
            node_id: 'node-alt',
            tags: ['alpha'],
            priority_score: 42,
            recency_score: 0.5,
            dream_score: 0.1,
            level_token: 'tag2',
            date: '2026-03-06'
          }
        ]
      },
      {
        tag: 'beta',
        tag_priority: 80,
        node_count: 1,
        node_ids: ['node-best'],
        nodes: [
          {
            node_id: 'node-best',
            tags: ['alpha', 'beta'],
            priority_score: 88,
            recency_score: 0.9,
            dream_score: 0.4,
            level_token: 'node1',
            date: '2026-03-07'
          }
        ]
      }
    ]
  };
  fs.writeFileSync(matrixPath, `${JSON.stringify(matrix, null, 2)}\n`);

  const policy = {
    enabled: true,
    dry_run: true,
    min_shared_tags: 1,
    max_matches: 3,
    max_matrix_age_ms: 3600000,
    enqueue_to_attention: true,
    summary_max_chars: 180,
    min_priority_score: 0
  };
  fs.writeFileSync(policyPath, `${JSON.stringify(policy, null, 2)}\n`);

  const env = {
    MEMORY_AUTO_RECALL_POLICY_PATH: policyPath,
    MEMORY_AUTO_RECALL_EVENTS_PATH: eventsPath,
    MEMORY_AUTO_RECALL_LATEST_PATH: latestPath,
    MEMORY_MATRIX_JSON_PATH: matrixPath
  };

  const out = run(['filed', '--node-id=incoming-node', '--tags=alpha,beta', '--dry-run=1'], env);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  const payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'auto recall should return ok');
  assert.ok(Array.isArray(payload.matches) && payload.matches.length >= 1, 'matches should exist');
  assert.strictEqual(payload.matches[0].node_id, 'node-best', 'best overlap match should rank first');
  assert.ok(payload.attention && payload.attention.skipped === true, 'dry-run should skip queue enqueue');
  assert.ok(fs.existsSync(eventsPath), 'events log should be written');
  assert.ok(fs.existsSync(latestPath), 'latest state should be written');

  console.log('memory_auto_recall.test.js: OK');
} catch (err) {
  console.error(`memory_auto_recall.test.js: FAIL: ${err && err.message ? err.message : err}`);
  process.exit(1);
}
