#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const SRS_PATH = 'docs/workspace/SRS.md';
const TODO_PATH = 'docs/workspace/TODO.md';
const OUT_JSON = 'core/local/artifacts/srs_todo_sync_current.json';

type SrsStatus =
  | 'queued'
  | 'in_progress'
  | 'blocked'
  | 'blocked_external_prepared'
  | 'done'
  | 'existing-coverage-validated';

type SrsRow = {
  id: string;
  status: SrsStatus;
  section: string;
  impact: number;
};

type SectionSummary = {
  section: string;
  total: number;
  queued: number;
  in_progress: number;
  blocked: number;
  blocked_external_prepared: number;
  done: number;
  existing_coverage_validated: number;
};

const STATUS_ORDER: SrsStatus[] = [
  'queued',
  'in_progress',
  'blocked',
  'blocked_external_prepared',
  'done',
  'existing-coverage-validated',
];

function read(path: string): string {
  return readFileSync(resolve(path), 'utf8');
}

function parseSrsRows(markdown: string): SrsRow[] {
  const out: SrsRow[] = [];
  const lines = markdown.split('\n');
  let section = 'Uncategorized';
  for (const line of lines) {
    const sectionMatch = line.match(/^##\s+(.+)$/);
    if (sectionMatch) {
      section = sectionMatch[1].trim();
      continue;
    }
    if (!line.startsWith('|')) continue;
    const statusMatch = line.match(
      /^\|\s*(V[^|\n]+?)\s*\|\s*(queued|in_progress|blocked|blocked_external_prepared|done|existing-coverage-validated)\s*\|/i,
    );
    if (!statusMatch) continue;
    const id = statusMatch[1].trim().toUpperCase();
    const status = statusMatch[2].toLowerCase() as SrsStatus;
    if (!/^V[0-9A-Z._-]+$/.test(id)) continue;
    const cells = line
      .split('|')
      .slice(1, -1)
      .map((cell) => cell.trim());
    const impact = Number(cells[5] ?? 0);
    out.push({
      id,
      status,
      section,
      impact: Number.isFinite(impact) ? impact : 0,
    });
  }
  return out;
}

function bySection(rows: SrsRow[]) {
  const order: string[] = [];
  const map = new Map<string, SectionSummary>();
  for (const row of rows) {
    if (!map.has(row.section)) {
      map.set(row.section, {
        section: row.section,
        total: 0,
        queued: 0,
        in_progress: 0,
        blocked: 0,
        blocked_external_prepared: 0,
        done: 0,
        existing_coverage_validated: 0,
      });
      order.push(row.section);
    }
    const summary = map.get(row.section)!;
    summary.total += 1;
    switch (row.status) {
      case 'queued':
        summary.queued += 1;
        break;
      case 'in_progress':
        summary.in_progress += 1;
        break;
      case 'blocked':
        summary.blocked += 1;
        break;
      case 'blocked_external_prepared':
        summary.blocked_external_prepared += 1;
        break;
      case 'done':
        summary.done += 1;
        break;
      case 'existing-coverage-validated':
        summary.existing_coverage_validated += 1;
        break;
      default:
        break;
    }
  }
  return order.map((section) => map.get(section)!);
}

function statusCounts(rows: SrsRow[]) {
  const counts = Object.fromEntries(STATUS_ORDER.map((status) => [status, 0])) as Record<SrsStatus, number>;
  for (const row of rows) counts[row.status] += 1;
  return counts;
}

function checkboxForSection(section: SectionSummary): 'x' | ' ' {
  const openCount =
    section.queued + section.in_progress + section.blocked + section.blocked_external_prepared;
  return openCount === 0 ? 'x' : ' ';
}

function checkboxForStatus(status: SrsStatus): 'x' | ' ' {
  return status === 'queued' || status === 'in_progress' || status === 'blocked' || status === 'blocked_external_prepared'
    ? ' '
    : 'x';
}

function toTodo(rows: SrsRow[]) {
  const counts = statusCounts(rows);
  const sections = bySection(rows);
  const actionable = rows
    .filter((row) => row.status === 'queued' || row.status === 'in_progress')
    .sort((a, b) => b.impact - a.impact || a.id.localeCompare(b.id));
  const externalBlocked = rows
    .filter((row) => row.status === 'blocked_external_prepared')
    .sort((a, b) => b.impact - a.impact || a.id.localeCompare(b.id));

  const lines: string[] = [];
  lines.push('# TODO (SRS Execution Checklist)');
  lines.push('');
  lines.push(`Updated: ${new Date().toISOString()}`);
  lines.push('');
  lines.push('## Global Rollup');
  lines.push(`- total_rows: ${rows.length}`);
  lines.push(`- queued: ${counts.queued}`);
  lines.push(`- in_progress: ${counts.in_progress}`);
  lines.push(`- blocked: ${counts.blocked}`);
  lines.push(`- blocked_external_prepared: ${counts.blocked_external_prepared}`);
  lines.push(`- done: ${counts.done}`);
  lines.push(`- existing_coverage_validated: ${counts['existing-coverage-validated']}`);
  lines.push('');
  lines.push('## SRS Section Checklist');
  for (const section of sections) {
    lines.push(
      `- [${checkboxForSection(section)}] ${section.section} — queued=${section.queued}, in_progress=${section.in_progress}, blocked=${section.blocked}, blocked_external_prepared=${section.blocked_external_prepared}, done=${section.done}, existing_coverage_validated=${section.existing_coverage_validated}`,
    );
  }
  lines.push('');
  lines.push('## Actionable SRS Items (Queued/In Progress)');
  if (actionable.length === 0) {
    lines.push('- [x] none');
  } else {
    for (const row of actionable) {
      lines.push(`- [${checkboxForStatus(row.status)}] \`${row.id}\` — ${row.status} — ${row.section}`);
    }
  }
  lines.push('');
  lines.push('## External Blockers');
  if (externalBlocked.length === 0) {
    lines.push('- [x] none');
  } else {
    for (const row of externalBlocked) {
      lines.push(
        `- [ ] \`${row.id}\` — blocked_external_prepared — ${row.section} (requires external evidence packet / human approval)`,
      );
    }
  }
  lines.push('');
  lines.push('## Regression Runbook');
  lines.push('- npm run -s ops:backlog:actionable-report');
  lines.push('- cargo run -q -p infring-ops-core --bin infring-ops -- backlog-queue-executor run --all=1 --with-tests=1');
  lines.push('- npm run -s ops:srs:full:regression');
  lines.push('- npm run -s ops:srs:top200:regression');
  lines.push('- npm run -s test:ops:srs-contract-runtime-evidence');
  lines.push('- ./verify.sh');
  lines.push('');
  return {
    markdown: `${lines.join('\n')}\n`,
    summary: {
      total_rows: rows.length,
      queued: counts.queued,
      in_progress: counts.in_progress,
      blocked: counts.blocked,
      blocked_external_prepared: counts.blocked_external_prepared,
      done: counts.done,
      existing_coverage_validated: counts['existing-coverage-validated'],
      section_count: sections.length,
      actionable_count: actionable.length,
      external_blocked_count: externalBlocked.length,
    },
  };
}

function main() {
  const srs = parseSrsRows(read(SRS_PATH));
  const payload = toTodo(srs);
  writeFileSync(resolve(TODO_PATH), payload.markdown, 'utf8');
  writeFileSync(
    resolve(OUT_JSON),
    `${JSON.stringify(
      {
        ok: true,
        type: 'srs_todo_sync',
        generatedAt: new Date().toISOString(),
        summary: payload.summary,
        out_todo: TODO_PATH,
      },
      null,
      2,
    )}\n`,
    'utf8',
  );
  console.log(
    JSON.stringify(
      {
        ok: true,
        type: 'srs_todo_sync',
        summary: payload.summary,
        out_todo: TODO_PATH,
        out_json: OUT_JSON,
      },
      null,
      2,
    ),
  );
}

main();
