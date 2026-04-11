#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/release_blocker_rubric.json');
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/release_blocker_rubric_current.json');

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = clean(raw, 24).toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function parseArgs(argv: string[]) {
  const parsed = {
    strict: false,
    out: DEFAULT_OUT,
  };
  for (const tokenRaw of argv) {
    const token = clean(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--strict=')) parsed.strict = parseBool(token.slice(9), false);
    else if (token.startsWith('--out=')) parsed.out = path.resolve(ROOT, clean(token.slice(6), 400));
  }
  return parsed;
}

function buildReport() {
  const rubric = JSON.parse(fs.readFileSync(POLICY_PATH, 'utf8'));
  const allowedClassifications = new Set(rubric.allowed_classifications || []);
  const allowedStatuses = new Set(rubric.allowed_statuses || []);
  const blockingStatuses = new Set(rubric.blocking_statuses || []);
  const invalid: string[] = [];
  const openBlockers: Array<{
    id: string;
    title: string;
    owner: string;
    status: string;
    age_days: number;
    exit_criteria: string;
  }> = [];
  const ownerCounts: Record<string, number> = {};
  const statusCounts: Record<string, number> = {};
  const budgetMaxOpen = Number.isFinite(Number(rubric.release_blocker_budget_max_open))
    ? Number(rubric.release_blocker_budget_max_open)
    : 0;
  for (const row of rubric.entries || []) {
    const id = clean(row.id, 120) || 'unknown';
    const owner = clean(row.owner, 120);
    const title = clean(row.title, 200);
    const status = clean(row.status, 80);
    const exitCriteria = clean(row.exit_criteria, 300);
    const openedAt = clean(row.opened_at, 40);
    if (!allowedClassifications.has(row.classification)) invalid.push(`${id}:invalid_classification`);
    if (!allowedStatuses.has(row.status)) invalid.push(`${id}:invalid_status`);
    if (!owner || !title || !clean(row.rationale, 300)) {
      invalid.push(`${id}:missing_required_fields`);
    }
    if (!openedAt) invalid.push(`${id}:opened_at_missing`);
    if (!exitCriteria) invalid.push(`${id}:exit_criteria_missing`);
    ownerCounts[owner || 'unassigned'] = (ownerCounts[owner || 'unassigned'] || 0) + 1;
    statusCounts[status || 'unknown'] = (statusCounts[status || 'unknown'] || 0) + 1;
    if (row.classification === 'release_blocker' && blockingStatuses.has(row.status)) {
      const openedAtMs = Date.parse(openedAt);
      const ageDays = Number.isFinite(openedAtMs)
        ? Math.max(0, Math.floor((Date.now() - openedAtMs) / 86400000))
        : -1;
      if (!Number.isFinite(openedAtMs)) invalid.push(`${id}:opened_at_invalid`);
      openBlockers.push({
        id,
        title,
        owner,
        status,
        age_days: ageDays,
        exit_criteria: exitCriteria,
      });
    }
  }
  const openBlockerIds = openBlockers.map((row) => row.id);
  const oldestOpenBlockerDays = openBlockers.reduce(
    (max, row) => Math.max(max, Number(row.age_days || 0)),
    0,
  );
  const budgetRemaining = budgetMaxOpen - openBlockers.length;
  return {
    ok: invalid.length === 0 && openBlockers.length <= budgetMaxOpen,
    type: 'release_blocker_rubric_gate',
    generated_at: new Date().toISOString(),
    invalid,
    release_blocker_budget_max_open: budgetMaxOpen,
    release_blocker_budget_remaining: budgetRemaining,
    open_release_blockers: openBlockerIds,
    oldest_open_blocker_days: oldestOpenBlockerDays,
    by_owner: ownerCounts,
    by_status: statusCounts,
    burn_down_dashboard: openBlockers,
    total_entries: Array.isArray(rubric.entries) ? rubric.entries.length : 0,
  };
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const report = buildReport();
  fs.mkdirSync(path.dirname(args.out), { recursive: true });
  fs.writeFileSync(args.out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report)}\n`);
  if (args.strict && report.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, buildReport };
