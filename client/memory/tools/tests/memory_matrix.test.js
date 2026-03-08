#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, 'systems', 'memory', 'memory_matrix.js');

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
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'memory-matrix-test-'));
  const memoryDir = path.join(tmp, 'memory');
  const stateDir = path.join(tmp, 'state', 'memory');
  fs.mkdirSync(memoryDir, { recursive: true });
  fs.mkdirSync(stateDir, { recursive: true });

  const memoryIndexPath = path.join(memoryDir, 'MEMORY_INDEX.md');
  const tagsIndexPath = path.join(memoryDir, 'TAGS_INDEX.md');
  const matrixJsonPath = path.join(stateDir, 'matrix', 'tag_memory_matrix.json');
  const matrixMdPath = path.join(memoryDir, 'TAG_MEMORY_MATRIX.md');
  const idleDir = path.join(stateDir, 'dreams', 'idle');
  const remDir = path.join(stateDir, 'dreams', 'rem');
  const conversationDir = path.join(stateDir, 'conversation_eye');
  const conversationNodesPath = path.join(conversationDir, 'nodes.jsonl');
  fs.mkdirSync(idleDir, { recursive: true });
  fs.mkdirSync(remDir, { recursive: true });
  fs.mkdirSync(conversationDir, { recursive: true });

  fs.writeFileSync(memoryIndexPath, [
    '# MEMORY_INDEX.md',
    '## System',
    '| node_id | uid | tags | file | summary |',
    '|---------|-----|------|------|---------|',
    '| node-alpha | uid1 | #focus #alpha | 2026-03-07.md | alpha node |',
    '| tag-beta | uid2 | #focus #beta | 2026-03-07.md | beta tag |',
    '| jot-gamma | uid3 | #focus #gamma | 2026-03-07.md | gamma jot |'
  ].join('\n'));
  fs.writeFileSync(tagsIndexPath, '#focus → node-alpha, tag-beta, jot-gamma\n');
  fs.writeFileSync(path.join(idleDir, '2026-03-07.jsonl'), `${JSON.stringify({ seeds: [] })}\n`);
  fs.writeFileSync(conversationNodesPath, '');

  const env = {
    MEMORY_MATRIX_MEMORY_DIR: memoryDir,
    MEMORY_MATRIX_INDEX_PATH: memoryIndexPath,
    MEMORY_MATRIX_TAGS_PATH: tagsIndexPath,
    MEMORY_MATRIX_JSON_PATH: matrixJsonPath,
    MEMORY_MATRIX_MD_PATH: matrixMdPath,
    MEMORY_MATRIX_IDLE_DIR: idleDir,
    MEMORY_MATRIX_REM_DIR: remDir,
    CONVERSATION_EYE_MEMORY_DIR: conversationDir,
    MEMORY_MATRIX_CONVERSATION_PATH: conversationNodesPath
  };

  const out = run(['run', '--apply=1', '--reason=test'], env);
  assert.strictEqual(out.status, 0, out.stderr || out.stdout);
  const payload = parseJson(out.stdout);
  assert.ok(payload && payload.ok === true, 'matrix build should succeed');
  assert.ok(fs.existsSync(matrixJsonPath), 'matrix json should exist');
  assert.ok(fs.existsSync(matrixMdPath), 'matrix markdown should exist');

  const matrix = JSON.parse(fs.readFileSync(matrixJsonPath, 'utf8'));
  const focus = Array.isArray(matrix.tags)
    ? matrix.tags.find((row) => row && row.tag === 'focus')
    : null;
  assert.ok(focus, 'focus tag should exist');
  const ordered = (focus.nodes || []).map((row) => row.node_id);
  assert.deepStrictEqual(ordered.slice(0, 3), ['node-alpha', 'tag-beta', 'jot-gamma'], 'level order must be node1 > tag2 > jot3');

  console.log('memory_matrix.test.js: OK');
} catch (err) {
  console.error(`memory_matrix.test.js: FAIL: ${err && err.message ? err.message : err}`);
  process.exit(1);
}
