#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = process.cwd();
const FIXTURE_DIR = path.join(ROOT, 'tests/fixtures/release_state_compat');
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/stateful_upgrade_rollback_gate_current.json');
const RUN_OPS_SCRIPT = path.join(ROOT, 'adapters/runtime/run_infring_ops.ts');
const PERSONA_ORCHESTRATION_SCRIPT = path.join(ROOT, 'client/runtime/systems/personas/orchestration.ts');
const ASSIMILATION_GUARD_SCRIPT = path.join(ROOT, 'tests/tooling/scripts/ci/assimilation_v1_support_guard.ts');
const TS_ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');

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
  if (!Array.isArray(payload?.package_runtime_surfaces?.packages) || payload.package_runtime_surfaces.packages.length === 0) {
    errors.push(`${id}:package_runtime_surfaces_missing`);
  }
  for (const task of payload?.task_fabric?.tasks || []) {
    if (!task.task_id || !task.lifecycle_status || !task.readiness) errors.push(`${id}:task_missing_required_fields`);
    if (!Array.isArray(task.related_links)) errors.push(`${id}:task_related_links_missing`);
  }
  for (const object of payload?.memory_state?.objects || []) {
    if (!object.object_id || !object.scope || typeof object.canonical !== 'boolean') {
      errors.push(`${id}:memory_object_missing_required_fields`);
    }
  }
  for (const pkg of payload?.package_runtime_surfaces?.packages || []) {
    if (!pkg.name || !pkg.support_level || !pkg.transport_mode) {
      errors.push(`${id}:package_surface_missing_required_fields`);
    }
    if (!Array.isArray(pkg.supported_commands) || pkg.supported_commands.length === 0) {
      errors.push(`${id}:package_surface_commands_missing`);
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
  const previousPackages = new Map(
    (previous.package_runtime_surfaces?.packages || []).map((row: any) => [clean(row?.name), row]),
  );
  const nextPackages = new Map(
    (next.package_runtime_surfaces?.packages || []).map((row: any) => [clean(row?.name), row]),
  );
  for (const [name, pkg] of previousPackages.entries()) {
    if (!nextPackages.has(name)) {
      errors.push(`upgrade_missing_package_surface:${name}`);
      continue;
    }
    const nextPkg = nextPackages.get(name);
    const previousCommands = new Set((pkg?.supported_commands || []).map((row: any) => clean(row)));
    const nextCommands = new Set((nextPkg?.supported_commands || []).map((row: any) => clean(row)));
    for (const command of previousCommands) {
      if (!nextCommands.has(command)) errors.push(`upgrade_missing_package_command:${name}:${command}`);
    }
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
  const nextPackages = next.package_runtime_surfaces?.packages || [];
  for (const pkg of nextPackages) {
    const supportLevel = clean(pkg?.support_level, 80);
    if (!supportLevel) errors.push('rollback_package_support_level_missing');
    if (!clean(pkg?.transport_mode, 80)) errors.push('rollback_package_transport_mode_missing');
  }
  return errors;
}

function parseJsonLine(stdout: string): any {
  const whole = String(stdout || '').trim();
  if (whole) {
    try {
      return JSON.parse(whole);
    } catch {}
  }
  const lines = String(stdout || '')
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean);
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    try {
      return JSON.parse(lines[index]);
    } catch {}
  }
  return null;
}

function runTs(scriptAbs: string, argv: string[]) {
  const started = Date.now();
  const out = spawnSync('node', [TS_ENTRYPOINT, scriptAbs, ...argv], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const stdout = String(out.stdout || '');
  const stderr = String(out.stderr || '');
  return {
    status: Number.isFinite(Number(out.status)) ? Number(out.status) : 1,
    stdout,
    stderr: clean(stderr, 800),
    payload: parseJsonLine(stdout),
    duration_ms: Date.now() - started,
  };
}

function buildLiveRehearsal() {
  const stamp = `${Date.now()}-${process.pid}`;
  const taskGroupId = `release-rehearsal-${stamp}`;
  const rehearsalRoot = path.join(ROOT, 'core/local/artifacts/release_rehearsal', taskGroupId);
  fs.mkdirSync(rehearsalRoot, { recursive: true });

  const ensurePayload = JSON.stringify({
    task_group_id: taskGroupId,
    task_type: 'release-rehearsal',
    coordinator_session: 'release-rehearsal-session',
    agent_count: 2,
  });
  const ensureTaskGroup = runTs(RUN_OPS_SCRIPT, [
    'orchestration',
    'invoke',
    '--op=taskgroup.ensure',
    `--payload-json=${ensurePayload}`,
  ]);

  const coordinatorPayload = JSON.stringify({
    task_id: `release-rehearsal-task-${stamp}`,
    task_group_id: taskGroupId,
    root_dir: rehearsalRoot,
    prompt: 'Run a deterministic release rehearsal smoke checkpoint.',
    strict_mode: true,
  });
  const coordinatorRun = runTs(RUN_OPS_SCRIPT, [
    'orchestration',
    'invoke',
    '--op=coordinator.run',
    `--payload-json=${coordinatorPayload}`,
  ]);

  const memoryStatus = runTs(RUN_OPS_SCRIPT, ['memory-plane', 'unified-heap', 'status']);
  const runtimeReceipt = runTs(PERSONA_ORCHESTRATION_SCRIPT, [
    'meeting',
    'release rehearsal',
    '--dry-run=1',
    '--strict=1',
  ]);
  const assimilationGuard = runTs(ASSIMILATION_GUARD_SCRIPT, [
    '--strict=0',
    '--out=core/local/artifacts/assimilation_v1_support_guard_current.json',
  ]);

  const ensuredTaskGroupId = clean(
    ensureTaskGroup.payload?.task_group_id || ensureTaskGroup.payload?.task_group?.task_group_id || '',
    160,
  );
  const checkpointPath = clean(
    coordinatorRun.payload?.checkpoint_path || coordinatorRun.payload?.checkpoint?.checkpoint_path || '',
    400,
  );
  const completionSummaryComplete =
    coordinatorRun.payload?.completion_summary?.complete === true || coordinatorRun.payload?.complete === true;
  const runtimeReceiptType = clean(
    runtimeReceipt.payload?.payload?.payload?.type || runtimeReceipt.payload?.payload?.type || runtimeReceipt.payload?.type || '',
    120,
  );
  const runtimeSystemId = clean(
    runtimeReceipt.payload?.payload?.payload?.system_id || runtimeReceipt.payload?.payload?.system_id || runtimeReceipt.payload?.system_id || '',
    120,
  );

  const checks = {
    live_taskgroup_rehearsal_verified:
      ensureTaskGroup.status === 0 &&
      ensuredTaskGroupId === taskGroupId &&
      fs.existsSync(path.join(ROOT, 'local/workspace/scratchpad/taskgroups', `${taskGroupId}.json`)),
    live_receipt_rehearsal_verified:
      coordinatorRun.status === 0 &&
      completionSummaryComplete &&
      checkpointPath.length > 0 &&
      fs.existsSync(checkpointPath),
    live_memory_surface_verified:
      memoryStatus.status === 0 && clean(memoryStatus.payload?.type || '', 120) === 'unified_memory_heap_status',
    live_runtime_receipt_verified:
      runtimeReceipt.status === 0 &&
      runtimeReceiptType === 'runtime_systems_run' &&
      runtimeSystemId === 'SYSTEMS-PERSONAS-ORCHESTRATION',
    live_assimilation_contract_verified: assimilationGuard.status === 0 && assimilationGuard.payload?.ok === true,
  };

  const errors = Object.entries(checks)
    .filter(([, ok]) => !ok)
    .map(([id]) => id);

  return {
    ok: errors.length === 0,
    task_group_id: taskGroupId,
    rehearsal_root: rehearsalRoot,
    checks,
    errors,
    commands: [
      {
        id: 'taskgroup.ensure',
        status: ensureTaskGroup.status,
        duration_ms: ensureTaskGroup.duration_ms,
        stderr: ensureTaskGroup.stderr,
        payload: ensureTaskGroup.payload,
      },
      {
        id: 'coordinator.run',
        status: coordinatorRun.status,
        duration_ms: coordinatorRun.duration_ms,
        stderr: coordinatorRun.stderr,
        payload: coordinatorRun.payload,
      },
      {
        id: 'memory.unified_heap.status',
        status: memoryStatus.status,
        duration_ms: memoryStatus.duration_ms,
        stderr: memoryStatus.stderr,
        payload: memoryStatus.payload,
      },
      {
        id: 'personas.orchestration.meeting',
        status: runtimeReceipt.status,
        duration_ms: runtimeReceipt.duration_ms,
        stderr: runtimeReceipt.stderr,
        payload: runtimeReceipt.payload,
      },
      {
        id: 'assimilation_v1_support_guard',
        status: assimilationGuard.status,
        duration_ms: assimilationGuard.duration_ms,
        stderr: assimilationGuard.stderr,
        payload: assimilationGuard.payload,
      },
    ],
  };
}

function buildReport() {
  const previous = readJson(path.join(FIXTURE_DIR, 'v0_3_8_alpha.json'));
  const next = readJson(path.join(FIXTURE_DIR, 'v0_3_9_alpha.json'));
  const liveRehearsal = buildLiveRehearsal();
  const errors = []
    .concat(validateFixtureShape('v0_3_8_alpha', previous))
    .concat(validateFixtureShape('v0_3_9_alpha', next))
    .concat(validateUpgradePath(previous, next))
    .concat(validateRollbackPath(previous, next))
    .concat(liveRehearsal.errors);
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
      ...liveRehearsal.checks,
    },
    live_rehearsal: liveRehearsal,
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
