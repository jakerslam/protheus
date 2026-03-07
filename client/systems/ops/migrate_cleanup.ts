#!/usr/bin/env node
'use strict';
export {};

const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  emit
} = require('../../lib/queued_backlog_runtime');

const OPS_DIR = path.join(ROOT, 'client', 'systems', 'ops');

function usage() {
  console.log('Usage:');
  console.log('  node client/systems/ops/migrate_cleanup.js plan');
  console.log('  node client/systems/ops/migrate_cleanup.js run [--apply=1|0] [--strict=1|0]');
  console.log('  node client/systems/ops/migrate_cleanup.js status');
}

function runNode(scriptPath: string, args: string[]) {
  const out = spawnSync('node', [scriptPath, ...args], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  const stdout = String(out.stdout || '');
  const stderr = String(out.stderr || '');
  let payload: any = null;
  try { payload = stdout.trim() ? JSON.parse(stdout) : null; } catch {}
  return {
    status,
    ok: status === 0,
    stdout,
    stderr,
    payload
  };
}

function plan() {
  const migration = runNode(path.join(OPS_DIR, 'migrate_to_planes.js'), ['plan']);
  return {
    ok: migration.ok,
    type: 'migrate_cleanup_plan',
    ts: nowIso(),
    migration
  };
}

function status() {
  const migration = runNode(path.join(OPS_DIR, 'migrate_to_planes.js'), ['status']);
  const runtimeSurface = runNode(path.join(OPS_DIR, 'runtime_state_surface_guard.js'), ['status']);
  return {
    ok: migration.ok && runtimeSurface.ok,
    type: 'migrate_cleanup_status',
    ts: nowIso(),
    migration,
    runtime_surface_guard: runtimeSurface
  };
}

function run(args: Record<string, any>) {
  const apply = String(args.apply == null ? '1' : args.apply).trim() !== '0';
  const strict = String(args.strict == null ? '1' : args.strict).trim() !== '0';

  const migrationArgs = apply
    ? ['run', '--apply=1', '--move-untracked=1', '--compat-symlinks=0']
    : ['run', '--apply=0', '--move-untracked=1', '--compat-symlinks=0'];
  const migration = runNode(path.join(OPS_DIR, 'migrate_to_planes.js'), migrationArgs);

  const checks = [
    {
      id: 'runtime_state_surface_guard',
      run: () => runNode(path.join(OPS_DIR, 'runtime_state_surface_guard.js'), ['check', '--strict=1'])
    },
    {
      id: 'root_surface_contract',
      run: () => runNode(path.join(OPS_DIR, 'root_surface_contract.js'), ['check', '--strict=1'])
    },
    {
      id: 'source_runtime_classifier_contract',
      run: () => runNode(path.join(OPS_DIR, 'source_runtime_classifier_contract.js'), ['check', '--strict=1'])
    },
    {
      id: 'dependency_boundary_guard',
      run: () => runNode(path.join(OPS_DIR, 'dependency_boundary_guard.js'), ['check', '--strict=1'])
    }
  ];

  const checkResults = checks.map((entry) => ({
    id: entry.id,
    result: entry.run()
  }));
  const failedChecks = checkResults.filter((row) => !row.result.ok).map((row) => row.id);
  const ok = migration.ok && (!strict || failedChecks.length === 0);

  return {
    ok,
    type: 'migrate_cleanup',
    ts: nowIso(),
    apply,
    strict,
    migration,
    checks: checkResults,
    failed_checks: failedChecks
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 40).toLowerCase();
  if (args.help || cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    return emit({ ok: true, help: true }, 0);
  }
  if (cmd === 'plan') {
    const out = plan();
    return emit(out, out.ok ? 0 : 1);
  }
  if (cmd === 'status') {
    const out = status();
    return emit(out, out.ok ? 0 : 1);
  }
  if (cmd === 'run') {
    const out = run(args);
    return emit(out, out.ok ? 0 : 1);
  }
  usage();
  emit({ ok: false, error: `unknown_command:${cmd}` }, 1);
}

main();

