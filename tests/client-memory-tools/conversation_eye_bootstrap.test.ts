#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const MANIFEST = path.join(ROOT, 'core', 'layer0', 'ops', 'Cargo.toml');

function parseJsonOutput(text) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  for (const line of trimmed.split('\n').reverse()) {
    const candidate = line.trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function runKernel(command, payload) {
  const payloadBase64 = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const proc = spawnSync(
    'cargo',
    [
      'run',
      '--quiet',
      '--manifest-path',
      MANIFEST,
      '--bin',
      'infring-ops',
      '--',
      'conversation-eye-collector-kernel',
      command,
      `--payload-base64=${payloadBase64}`,
    ],
    {
      cwd: ROOT,
      encoding: 'utf8',
      env: {
        ...process.env,
        CARGO_TERM_COLOR: 'never',
      },
    },
  );
  assert.equal(proc.status, 0, proc.stderr || proc.stdout);
  const receipt = parseJsonOutput(proc.stdout) || parseJsonOutput(proc.stderr);
  assert(receipt && receipt.ok === true, `expected ok receipt for ${command}`);
  return receipt;
}

function main() {
  const bootstrap = runKernel('normalize-topics', { topics: [] });
  assert.equal(bootstrap.type, 'conversation_eye_collector_kernel');
  assert.deepStrictEqual(bootstrap.payload.topics, [
    'conversation',
    'decision',
    'insight',
    'directive',
    't1',
  ]);

  const processed = runKernel('process-nodes', {
    index: { emitted_node_ids: {} },
    topics: bootstrap.payload.topics,
    max_items: 1,
    candidates: [
      {
        node: {
          node_id: 'bootstrap-node',
          ts: '2026-01-01T00:00:00Z',
          title: 'Bootstrap node',
          preview: 'Conversation eye bootstrap proof',
          level: 2,
          node_tags: ['collector', 'bootstrap'],
          edges_to: ['alpha', 'alpha'],
        },
      },
    ],
  });
  assert.equal(processed.payload.ok, true);
  assert.deepStrictEqual(processed.payload.items[0].topics, [
    'conversation',
    'decision',
    'insight',
    'directive',
    't1',
  ]);
  assert.equal(typeof processed.receipt_hash, 'string');

  console.log(JSON.stringify({ ok: true, type: 'conversation_eye_bootstrap_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
