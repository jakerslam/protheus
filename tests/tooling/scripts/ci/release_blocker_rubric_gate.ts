#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/release_blocker_rubric.json');
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/release_blocker_rubric_current.json');
const REQUIRED_CLASSIFICATIONS = ['release_blocker', 'post_release_improvement', 'experimental_only'];
const REQUIRED_STATUSES = ['open', 'mitigated', 'accepted_risk', 'done'];
const REQUIRED_BLOCKING_STATUSES = ['open', 'mitigated'];

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = clean(raw, 24).toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function duplicateValues(values: string[]): string[] {
  const counts = new Map<string, number>();
  for (const value of values) counts.set(value, (counts.get(value) || 0) + 1);
  return [...counts.entries()]
    .filter(([, count]) => count > 1)
    .map(([value]) => value)
    .sort();
}

function hasPlaceholder(value: string): boolean {
  const token = clean(value, 240).toLowerCase();
  return token.includes('${') || token === 'tbd' || token === 'todo' || token === 'pending' || token === 'unknown';
}

function isCanonicalOwner(value: string): boolean {
  return /^[a-z][a-z0-9_-]*$/.test(clean(value, 120));
}

function isCanonicalBlockerId(value: string): boolean {
  return /^RBR-\d{3,}$/.test(clean(value, 120));
}

function isCanonicalDateToken(value: string): boolean {
  return /^\d{4}-\d{2}-\d{2}$/.test(clean(value, 40));
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

function buildReport(args: { strict: boolean; out: string }) {
  const rubric = JSON.parse(fs.readFileSync(POLICY_PATH, 'utf8'));
  const policyFailures: string[] = [];
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

  const outRel = clean(path.relative(ROOT, args.out).replace(/\\/g, '/'), 400);
  if (!outRel || outRel.startsWith('../') || outRel.startsWith('..\\')) {
    policyFailures.push(`output_path_outside_repo:${outRel || args.out}`);
  }
  if (!outRel.endsWith('.json')) {
    policyFailures.push(`output_path_non_json:${outRel || args.out}`);
  }

  const schemaId = clean(rubric.schema_id, 120);
  if (schemaId !== 'release_blocker_rubric') {
    policyFailures.push(`schema_id_invalid:${schemaId || 'missing'}`);
  }
  const schemaVersion = clean(rubric.schema_version, 40);
  if (!/^\d+\.\d+$/.test(schemaVersion)) {
    policyFailures.push(`schema_version_invalid:${schemaVersion || 'missing'}`);
  }

  const allowedClassificationList = Array.isArray(rubric.allowed_classifications)
    ? rubric.allowed_classifications.map((row: unknown) => clean(row, 80)).filter(Boolean)
    : [];
  if (allowedClassificationList.length === 0) {
    policyFailures.push('allowed_classifications_empty');
  }
  const duplicateAllowedClassifications = duplicateValues(allowedClassificationList);
  if (duplicateAllowedClassifications.length > 0) {
    policyFailures.push(`allowed_classifications_duplicate:${duplicateAllowedClassifications.join(',')}`);
  }
  const missingRequiredClassifications = REQUIRED_CLASSIFICATIONS.filter((row) => !allowedClassificationList.includes(row));
  if (missingRequiredClassifications.length > 0) {
    policyFailures.push(`allowed_classifications_required_missing:${missingRequiredClassifications.join(',')}`);
  }
  if (allowedClassificationList.join('|') !== REQUIRED_CLASSIFICATIONS.join('|')) {
    policyFailures.push('allowed_classifications_order_or_membership_drift');
  }

  const allowedStatusList = Array.isArray(rubric.allowed_statuses)
    ? rubric.allowed_statuses.map((row: unknown) => clean(row, 80)).filter(Boolean)
    : [];
  if (allowedStatusList.length === 0) {
    policyFailures.push('allowed_statuses_empty');
  }
  const duplicateAllowedStatuses = duplicateValues(allowedStatusList);
  if (duplicateAllowedStatuses.length > 0) {
    policyFailures.push(`allowed_statuses_duplicate:${duplicateAllowedStatuses.join(',')}`);
  }
  const missingRequiredStatuses = REQUIRED_STATUSES.filter((row) => !allowedStatusList.includes(row));
  if (missingRequiredStatuses.length > 0) {
    policyFailures.push(`allowed_statuses_required_missing:${missingRequiredStatuses.join(',')}`);
  }
  if (allowedStatusList.join('|') !== REQUIRED_STATUSES.join('|')) {
    policyFailures.push('allowed_statuses_order_or_membership_drift');
  }

  const blockingStatusList = Array.isArray(rubric.blocking_statuses)
    ? rubric.blocking_statuses.map((row: unknown) => clean(row, 80)).filter(Boolean)
    : [];
  if (blockingStatusList.length === 0) {
    policyFailures.push('blocking_statuses_empty');
  }
  const duplicateBlockingStatuses = duplicateValues(blockingStatusList);
  if (duplicateBlockingStatuses.length > 0) {
    policyFailures.push(`blocking_statuses_duplicate:${duplicateBlockingStatuses.join(',')}`);
  }
  const missingRequiredBlocking = REQUIRED_BLOCKING_STATUSES.filter((row) => !blockingStatusList.includes(row));
  if (missingRequiredBlocking.length > 0) {
    policyFailures.push(`blocking_statuses_required_missing:${missingRequiredBlocking.join(',')}`);
  }
  if (blockingStatusList.some((row) => !allowedStatuses.has(row))) {
    const outsideAllowed = blockingStatusList.filter((row) => !allowedStatuses.has(row));
    policyFailures.push(`blocking_statuses_not_in_allowed:${outsideAllowed.join(',')}`);
  }
  if (blockingStatusList.join('|') !== REQUIRED_BLOCKING_STATUSES.join('|')) {
    policyFailures.push('blocking_statuses_order_or_membership_drift');
  }

  const budgetMaxOpen = Number.isFinite(Number(rubric.release_blocker_budget_max_open))
    ? Number(rubric.release_blocker_budget_max_open)
    : 0;
  if (!Number.isInteger(budgetMaxOpen) || budgetMaxOpen < 0) {
    policyFailures.push(`release_blocker_budget_invalid:${String(rubric.release_blocker_budget_max_open)}`);
  }

  const entries = Array.isArray(rubric.entries) ? rubric.entries : [];
  if (!Array.isArray(rubric.entries)) {
    policyFailures.push('entries_not_array');
  }
  if (entries.length === 0) {
    policyFailures.push('entries_empty');
  }
  const entryIds = entries.map((row: any) => clean(row?.id, 120)).filter(Boolean);
  const duplicateEntryIds = duplicateValues(entryIds);
  if (duplicateEntryIds.length > 0) {
    policyFailures.push(`entry_id_duplicate:${duplicateEntryIds.join(',')}`);
  }

  for (const row of entries) {
    const id = clean(row.id, 120) || 'unknown';
    const owner = clean(row.owner, 120);
    const title = clean(row.title, 200);
    const status = clean(row.status, 80);
    const classification = clean(row.classification, 80);
    const rationale = clean(row.rationale, 300);
    const exitCriteria = clean(row.exit_criteria, 300);
    const openedAt = clean(row.opened_at, 40);
    if (!isCanonicalBlockerId(id)) invalid.push(`${id}:id_noncanonical`);
    if (!allowedClassifications.has(row.classification)) invalid.push(`${id}:invalid_classification`);
    if (!allowedStatuses.has(row.status)) invalid.push(`${id}:invalid_status`);
    if (!owner || !title || !rationale) {
      invalid.push(`${id}:missing_required_fields`);
    }
    if (!isCanonicalOwner(owner) || hasPlaceholder(owner)) invalid.push(`${id}:owner_invalid`);
    if (hasPlaceholder(title)) invalid.push(`${id}:title_placeholder`);
    if (hasPlaceholder(rationale)) invalid.push(`${id}:rationale_placeholder`);
    if (!openedAt) invalid.push(`${id}:opened_at_missing`);
    if (openedAt && !isCanonicalDateToken(openedAt)) invalid.push(`${id}:opened_at_noncanonical`);
    if (!exitCriteria) invalid.push(`${id}:exit_criteria_missing`);
    if (hasPlaceholder(exitCriteria)) invalid.push(`${id}:exit_criteria_placeholder`);
    if (classification === 'release_blocker' && status === 'accepted_risk') {
      invalid.push(`${id}:release_blocker_accepted_risk_forbidden`);
    }
    ownerCounts[owner || 'unassigned'] = (ownerCounts[owner || 'unassigned'] || 0) + 1;
    statusCounts[status || 'unknown'] = (statusCounts[status || 'unknown'] || 0) + 1;
    if (row.classification === 'release_blocker' && blockingStatuses.has(row.status)) {
      const openedAtMs = Date.parse(openedAt);
      if (Number.isFinite(openedAtMs) && openedAtMs > Date.now()) {
        invalid.push(`${id}:opened_at_in_future`);
      }
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
  const duplicateOpenBlockerIds = duplicateValues(openBlockerIds);
  if (duplicateOpenBlockerIds.length > 0) {
    invalid.push(`open_release_blockers_duplicate:${duplicateOpenBlockerIds.join(',')}`);
  }
  const oldestOpenBlockerDays = openBlockers.reduce(
    (max, row) => Math.max(max, Number(row.age_days || 0)),
    0,
  );
  const dashboardIds = openBlockers.map((row) => row.id);
  if (dashboardIds.join('|') !== openBlockerIds.join('|')) {
    invalid.push('open_release_blockers_dashboard_id_drift');
  }
  const ownerTotal = Object.values(ownerCounts).reduce((sum, value) => sum + Number(value || 0), 0);
  if (ownerTotal !== entries.length) {
    invalid.push(`owner_count_total_drift:${ownerTotal}:${entries.length}`);
  }
  const statusTotal = Object.values(statusCounts).reduce((sum, value) => sum + Number(value || 0), 0);
  if (statusTotal !== entries.length) {
    invalid.push(`status_count_total_drift:${statusTotal}:${entries.length}`);
  }
  const recomputedOldest = openBlockers.reduce((max, row) => Math.max(max, Number(row.age_days || 0)), 0);
  if (recomputedOldest !== oldestOpenBlockerDays) {
    invalid.push(`oldest_open_blocker_days_drift:${recomputedOldest}:${oldestOpenBlockerDays}`);
  }
  const budgetRemaining = budgetMaxOpen - openBlockers.length;
  if (budgetRemaining !== budgetMaxOpen - openBlockerIds.length) {
    invalid.push(`release_blocker_budget_remaining_drift:${budgetRemaining}`);
  }
  const totalIssues = policyFailures.length + invalid.length;
  return {
    ok: totalIssues === 0 && openBlockers.length <= budgetMaxOpen,
    type: 'release_blocker_rubric_gate',
    generated_at: new Date().toISOString(),
    policy_failures: policyFailures,
    invalid,
    release_blocker_budget_max_open: budgetMaxOpen,
    release_blocker_budget_remaining: budgetRemaining,
    open_release_blockers: openBlockerIds,
    oldest_open_blocker_days: oldestOpenBlockerDays,
    by_owner: ownerCounts,
    by_status: statusCounts,
    burn_down_dashboard: openBlockers,
    total_entries: entries.length,
    summary: {
      policy_failure_count: policyFailures.length,
      invalid_entry_count: invalid.length,
      total_issue_count: totalIssues,
      open_release_blocker_count: openBlockers.length,
      pass: totalIssues === 0 && openBlockers.length <= budgetMaxOpen,
    },
  };
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const report = buildReport(args);
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
