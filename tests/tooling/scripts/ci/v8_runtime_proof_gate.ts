#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const SRS_PATH = resolve('docs/workspace/SRS.md');
const PROOF_TEST_PATH = resolve('core/layer0/ops/tests/v8_runtime_proof.rs');

function parseRows(markdown) {
  const rows = [];
  for (const line of markdown.split('\n')) {
    if (!line.startsWith('|')) continue;
    const cells = line
      .split('|')
      .slice(1, -1)
      .map((v) => v.trim());
    if (cells.length < 2) continue;
    if (cells[0] === 'ID' || cells[0] === '---') continue;
    if (!/^V[0-9A-Z]+-/.test(cells[0])) continue;
    rows.push({
      id: cells[0],
      status: (cells[1] ?? '').toLowerCase(),
    });
  }
  return rows;
}

function fail(payload) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}

function main() {
  const srs = readFileSync(SRS_PATH, 'utf8');
  const proofTest = readFileSync(PROOF_TEST_PATH, 'utf8');
  const rows = parseRows(srs);
  const doneV8 = rows.filter((row) => row.id.startsWith('V8-') && row.status === 'done');

  if (doneV8.length === 0) {
    console.log(
      JSON.stringify(
        {
          ok: true,
          type: 'v8_runtime_proof_gate',
          done_v8_rows: 0,
          skipped: true,
          reason: 'no_done_v8_rows',
        },
        null,
        2,
      ),
    );
    return;
  }

  const missingCoverage = doneV8
    .map((row) => row.id)
    .filter((id) => !proofTest.includes(`"${id}"`));
  if (missingCoverage.length > 0) {
    fail({
      ok: false,
      type: 'v8_runtime_proof_gate',
      reason: 'done_v8_row_missing_runtime_proof_registry_coverage',
      missing_ids: missingCoverage,
    });
  }

  const run = spawnSync(
    'cargo',
    ['test', '--manifest-path', 'core/layer0/ops/Cargo.toml', '--test', 'v8_runtime_proof'],
    {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );
  if (run.status !== 0) {
    fail({
      ok: false,
      type: 'v8_runtime_proof_gate',
      reason: 'runtime_proof_test_failed',
      exit_code: run.status,
      stdout_tail: String(run.stdout || '')
        .split('\n')
        .slice(-40)
        .join('\n'),
      stderr_tail: String(run.stderr || '')
        .split('\n')
        .slice(-40)
        .join('\n'),
    });
  }

  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'v8_runtime_proof_gate',
        done_v8_rows: doneV8.length,
        checked_ids: doneV8.map((row) => row.id),
      },
      null,
      2,
    ),
  );
}

main();

