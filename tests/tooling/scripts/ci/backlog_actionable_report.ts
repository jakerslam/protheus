#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync } from 'node:fs';

function parseSrsRows(markdown) {
  const rowsById = new Map();
  for (const line of markdown.split('\n')) {
    if (!line.startsWith('|')) continue;
    const cells = line
      .split('|')
      .slice(1, -1)
      .map((c) => c.trim());
    if (cells.length < 2) continue;
    if (cells[0] === 'ID' || cells[0] === '---') continue;
    const id = cells[0];
    if (!/^V[0-9A-Z]+-/.test(id)) continue;
    rowsById.set(id, {
      id,
      status: (cells[1] ?? '').toLowerCase(),
    });
  }
  return [...rowsById.values()];
}

const todoPath = 'docs/workspace/TODO.md';
const srsPath = 'docs/workspace/SRS.md';
const failOnActionable = process.argv.includes('--fail-on-actionable');

const todo = readFileSync(todoPath, 'utf8');
const srs = readFileSync(srsPath, 'utf8');
const srsRows = parseSrsRows(srs);

const todoUnchecked = (todo.match(/^- \[ \]/gm) ?? []).length;
const todoChecked = (todo.match(/^- \[x\]/gim) ?? []).length;
const srsQueued = srsRows.filter((row) => row.status === 'queued').length;
const srsInProgress = srsRows.filter((row) => row.status === 'in_progress').length;
const srsBlocked = srsRows.filter((row) => row.status === 'blocked').length;
const srsBlockedExternalPrepared = srsRows.filter((row) => row.status === 'blocked_external_prepared').length;
const srsDone = srsRows.filter((row) => row.status === 'done').length;
const srsExistingCoverage = srsRows.filter((row) => row.status === 'existing-coverage-validated').length;

const actionable = todoUnchecked + srsQueued + srsInProgress;
const report = {
  ok: failOnActionable ? actionable === 0 : true,
  type: 'backlog_actionable_report',
  actionable_count: actionable,
  todo: {
    unchecked: todoUnchecked,
    checked: todoChecked,
  },
  srs: {
    queued: srsQueued,
    in_progress: srsInProgress,
    blocked: srsBlocked,
    blocked_external_prepared: srsBlockedExternalPrepared,
    done: srsDone,
    existing_coverage_validated: srsExistingCoverage,
  },
};

console.log(JSON.stringify(report, null, 2));
if (failOnActionable && actionable > 0) {
  process.exit(1);
}
