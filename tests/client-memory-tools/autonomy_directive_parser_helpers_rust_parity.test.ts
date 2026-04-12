#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const MANIFEST = path.join(ROOT, 'core', 'layer2', 'execution', 'Cargo.toml');

function runExecution(command, payload) {
  const out = spawnSync(
    'cargo',
    [
      'run',
      '-q',
      '--manifest-path',
      MANIFEST,
      '--bin',
      'execution_core',
      '--',
      command,
      `--payload=${JSON.stringify(payload)}`,
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
  let parsed = null;
  try {
    parsed = JSON.parse(String(out.stdout || '').trim() || '{}');
  } catch {}
  return {
    status: Number.isFinite(Number(out.status)) ? Number(out.status) : 1,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || ''),
    payload: parsed,
  };
}

function main() {
  const parseLowerList = runExecution('autoscale', {
    mode: 'parse_lower_list',
    parse_lower_list_input: {
      list: [],
      csv: 'Alpha\nBeta; GAMMA, delta',
    },
  });
  assert.equal(parseLowerList.status, 0, `parse_lower_list failed\n${parseLowerList.stderr}`);
  assert.equal(parseLowerList.payload.ok, true);
  assert.equal(parseLowerList.payload.mode, 'parse_lower_list');
  assert.deepStrictEqual(parseLowerList.payload.payload.items, ['alpha', 'beta', 'gamma', 'delta']);

  const parseDirectiveFile = runExecution('autoscale', {
    mode: 'parse_directive_file_arg',
    parse_directive_file_arg_input: {
      command: 'validate --file client/runtime/config/directives/web_fetch.yaml --strict=1',
    },
  });
  assert.equal(parseDirectiveFile.status, 0, `parse_directive_file_arg failed\n${parseDirectiveFile.stderr}`);
  assert.equal(parseDirectiveFile.payload.ok, true);
  assert.equal(
    parseDirectiveFile.payload.payload.file,
    'client/runtime/config/directives/web_fetch.yaml',
  );

  const parseCandidates = runExecution('inversion', {
    mode: 'parse_candidate_list_from_llm_payload',
    parse_candidate_list_from_llm_payload_input: {
      payload: {
        payload: JSON.stringify({
          candidates: [
            {
              id: 'Web Fetch',
              filterStack: ['bounded_parallel_probe', 'risk_guard_compaction'],
              probability: '0.81',
              reason: 'nested payload works',
            },
            {
              id: 'skip-me',
              filters: [],
              probability: 0.2,
            },
          ],
        }),
      },
    },
  });
  assert.equal(parseCandidates.status, 0, `parse_candidate_list failed\n${parseCandidates.stderr}`);
  assert.equal(parseCandidates.payload.ok, true);
  assert.equal(parseCandidates.payload.mode, 'parse_candidate_list_from_llm_payload');
  assert.equal(parseCandidates.payload.payload.candidates.length, 1);
  assert.equal(parseCandidates.payload.payload.candidates[0].id, 'web_fetch');
  assert.deepStrictEqual(
    parseCandidates.payload.payload.candidates[0].filters,
    ['bounded_parallel_probe', 'risk_guard_compaction'],
  );

  const parseLane = runExecution('inversion', {
    mode: 'parse_lane_decision',
    parse_lane_decision_input: {
      args: {
        brainLane: 'Right-Brain',
      },
    },
  });
  assert.equal(parseLane.status, 0, `parse_lane_decision failed\n${parseLane.stderr}`);
  assert.equal(parseLane.payload.ok, true);
  assert.equal(parseLane.payload.payload.selected_lane, 'right-brain');
  assert.equal(parseLane.payload.payload.source, 'arg');

  console.log(JSON.stringify({ ok: true, type: 'autonomy_directive_parser_helpers_rust_parity_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
