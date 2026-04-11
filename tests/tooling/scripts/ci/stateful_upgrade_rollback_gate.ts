#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const FIXTURE_DIR = path.join(ROOT, 'tests/fixtures/release_state_compat');
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/stateful_upgrade_rollback_gate_current.json');

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

function readJson(filePath: string): any {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function validateFixtureShape(id: string, payload: any): string[] {
  const errors: string[] = [];
  if (!Array.isArray(payload?.receipts) || payload.receipts.length === 0) errors.push(`${id}:receipts_missing`);
  if (!Array.isArray(payload?.task_fabric?.tasks) || payload.task_fabric.tasks.length === 0) errors.push(`${id}:task_fabric_missing`);
  if (!Array.isArray(payload?.memory_state?.objects) || payload.memory_state.objects.length === 0) errors.push(`${id}:memory_state_missing`);
  if (!payload?.assimilation_state?.protocol_state?.latest_receipt_id) errors.push(`${id}:assimilation_state_missing`);
  for (const task of payload?.task_fabric?.tasks || []) {
    if (!task.task_id || !task.lifecycle_status || !task.readiness) errors.push(`${id}:task_missing_required_fields`);
    if (!Array.isArray(task.related_links)) errors.push(`${id}:task_related_links_missing`);
  }
  for (const object of payload?.memory_state?.objects || []) {
    if (!object.object_id || !object.scope || typeof object.canonical !== 'boolean') {
      errors.push(`${id}:memory_object_missing_required_fields`);
    }
  }
  return errors;
}

function validateUpgradePath(previous: any, next: any): string[] {
  const errors: string[] = [];
  const previousTaskFields = Object.keys(previous.task_fabric.tasks[0] || {});
  const nextTaskFields = new Set(Object.keys(next.task_fabric.tasks[0] || {}));
  for (const field of previousTaskFields) {
    if (!nextTaskFields.has(field)) errors.push(`upgrade_missing_task_field:${field}`);
  }
  const previousReceipt = previous.receipts[0] || {};
  const nextReceipt = next.receipts[0] || {};
  for (const field of Object.keys(previousReceipt)) {
    if (!(field in nextReceipt)) errors.push(`upgrade_missing_receipt_field:${field}`);
  }
  return errors;
}

function validateRollbackPath(previous: any, next: any): string[] {
  const errors: string[] = [];
  const previousLatest = clean(previous.assimilation_state?.protocol_state?.latest_receipt_id || '', 120);
  const nextParents = Array.isArray(next.receipts?.[0]?.parent_receipt_ids) ? next.receipts[0].parent_receipt_ids : [];
  if (previousLatest && !nextParents.includes(previousLatest)) {
    errors.push('rollback_parent_receipt_link_missing');
  }
  const previousScopes = new Set((previous.memory_state?.objects || []).map((row: any) => clean(row.scope, 80)));
  const nextScopes = new Set((next.memory_state?.objects || []).map((row: any) => clean(row.scope, 80)));
  for (const scope of previousScopes) {
    if (!nextScopes.has(scope)) errors.push(`rollback_scope_missing:${scope}`);
  }
  return errors;
}

function buildReport() {
  const previous = readJson(path.join(FIXTURE_DIR, 'v0_3_8_alpha.json'));
  const next = readJson(path.join(FIXTURE_DIR, 'v0_3_9_alpha.json'));
  const errors = []
    .concat(validateFixtureShape('v0_3_8_alpha', previous))
    .concat(validateFixtureShape('v0_3_9_alpha', next))
    .concat(validateUpgradePath(previous, next))
    .concat(validateRollbackPath(previous, next));
  return {
    ok: errors.length === 0,
    type: 'stateful_upgrade_rollback_gate',
    generated_at: new Date().toISOString(),
    fixtures: [
      'tests/fixtures/release_state_compat/v0_3_8_alpha.json',
      'tests/fixtures/release_state_compat/v0_3_9_alpha.json',
    ],
    checks: {
      upgrade_path_verified: !errors.some((row) => row.startsWith('upgrade_')),
      rollback_path_verified: !errors.some((row) => row.startsWith('rollback_')),
      fixture_shape_verified: !errors.some((row) => row.includes('_missing')),
    },
    errors,
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
