#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { invokeTsModuleSync } from '../../../../client/runtime/lib/in_process_ts_delegate.ts';

type Args = {
  strict: boolean;
  out: string;
  runSmoke: boolean;
};

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

type Policy = {
  required_files?: string[];
  required_package_scripts?: string[];
  required_ci_invocations?: string[];
  required_verify_invocations?: string[];
  required_readme_markers?: string[];
  smoke_scripts?: string[];
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/production_readiness_closure_policy.json');

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = String(raw || '').trim().toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function parseArgs(argv: string[]): Args {
  const args: Args = {
    strict: false,
    out: 'core/local/artifacts/production_readiness_closure_gate_current.json',
    runSmoke: true,
  };
  for (const token of argv) {
    if (token === '--strict') args.strict = true;
    else if (token.startsWith('--strict=')) args.strict = parseBool(token.slice('--strict='.length), false);
    else if (token.startsWith('--out=')) args.out = token.slice('--out='.length);
    else if (token.startsWith('--run-smoke=')) args.runSmoke = parseBool(token.slice('--run-smoke='.length), true);
  }
  return args;
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function checkRequiredFiles(files: string[]): Check[] {
  return files.map((relPath) => {
    const ok = fs.existsSync(path.resolve(ROOT, relPath));
    return {
      id: `required_file:${relPath}`,
      ok,
      detail: ok ? 'present' : 'missing',
    };
  });
}

function checkPackageScripts(requiredScripts: string[]): Check[] {
  const pkg = readJson<{ scripts?: Record<string, string> }>(path.join(ROOT, 'package.json'), {});
  const scripts = pkg.scripts || {};
  return requiredScripts.map((scriptName) => {
    const ok = typeof scripts[scriptName] === 'string' && scripts[scriptName].trim().length > 0;
    return {
      id: `package_script:${scriptName}`,
      ok,
      detail: ok ? 'registered' : 'missing',
    };
  });
}

function checkTextMarkers(filePath: string, markers: string[], prefix: string): Check[] {
  let source = '';
  try {
    source = fs.readFileSync(filePath, 'utf8');
  } catch {
    return markers.map((marker) => ({
      id: `${prefix}:${marker}`,
      ok: false,
      detail: `${path.relative(ROOT, filePath)} missing`,
    }));
  }
  return markers.map((marker) => {
    const ok = source.includes(marker);
    return {
      id: `${prefix}:${marker}`,
      ok,
      detail: ok ? 'present' : 'missing',
    };
  });
}

function parseJsonLine(stdout: string): unknown {
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
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function runTsScript(scriptRelPath: string, args: string[] = []) {
  const out = invokeTsModuleSync(path.join(ROOT, scriptRelPath), {
    argv: args,
    cwd: ROOT,
    exportName: 'run',
    teeStdout: false,
    teeStderr: false,
  });
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  return {
    status,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || ''),
    payload: parseJsonLine(String(out.stdout || '')),
  };
}

function runSmokeScripts(scriptNames: string[]): Check[] {
  const registry: Record<string, { script: string; args: string[] }> = {
    'ops:legacy-runner:release-guard': {
      script: 'tests/tooling/scripts/ci/legacy_process_runner_release_guard.ts',
      args: ['--strict=1', '--out=core/local/artifacts/legacy_process_runner_release_guard_current.json'],
    },
    'ops:transport:topology:gate': {
      script: 'client/runtime/systems/ops/transport_topology_status.ts',
      args: ['--strict=1', '--json=1'],
    },
    'ops:production-topology:status': {
      script: 'tests/tooling/scripts/ops/production_topology_diagnostic.ts',
      args: ['--out=core/local/artifacts/production_topology_diagnostic_current.json'],
    },
    'ops:stateful-upgrade-rollback:gate': {
      script: 'tests/tooling/scripts/ci/stateful_upgrade_rollback_gate.ts',
      args: ['--strict=1', '--out=core/local/artifacts/stateful_upgrade_rollback_gate_current.json'],
    },
    'ops:release-blockers:gate': {
      script: 'tests/tooling/scripts/ci/release_blocker_rubric_gate.ts',
      args: ['--strict=1', '--out=core/local/artifacts/release_blocker_rubric_current.json'],
    },
    'dr:gameday': {
      script: 'client/runtime/systems/ops/dr_gameday.ts',
      args: ['run', '--strict=1'],
    },
    'dr:gameday:gate': {
      script: 'client/runtime/systems/ops/dr_gameday_gate.ts',
      args: ['run', '--strict=1'],
    },
    'ops:release-contract:gate': {
      script: 'tests/tooling/scripts/ci/release_contract_gate.ts',
      args: ['--strict=1', '--out=core/local/artifacts/release_contract_gate_current.json'],
    },
    'ops:support-bundle:export': {
      script: 'client/runtime/systems/ops/support_bundle_export.ts',
      args: ['run', '--out=core/local/artifacts/support_bundle_latest.json'],
    },
  };

  return scriptNames.map((scriptName) => {
    const spec = registry[scriptName];
    if (!spec) {
      return {
        id: `smoke_script:${scriptName}`,
        ok: false,
        detail: 'unregistered_direct_smoke_script',
      };
    }
    const out = runTsScript(spec.script, spec.args);
    return {
      id: `smoke_script:${scriptName}`,
      ok: out.status === 0,
      detail: out.status === 0
        ? 'ok'
        : `status=${out.status}; stderr=${String(out.stderr || '').trim().slice(0, 400)}`,
    };
  });
}

function buildReport(args: Args) {
  const checks: Check[] = [];
  if (!fs.existsSync(POLICY_PATH)) {
    checks.push({
      id: 'policy_file',
      ok: false,
      detail: 'client/runtime/config/production_readiness_closure_policy.json missing',
    });
  }
  const policy = readJson<Policy>(POLICY_PATH, {});
  checks.push(...checkRequiredFiles(policy.required_files || []));
  checks.push(...checkPackageScripts(policy.required_package_scripts || []));
  checks.push(
    ...checkTextMarkers(
      path.join(ROOT, '.github/workflows/ci.yml'),
      policy.required_ci_invocations || [],
      'ci_invocation',
    ),
  );
  checks.push(
    ...checkTextMarkers(
      path.join(ROOT, 'verify.sh'),
      policy.required_verify_invocations || [],
      'verify_invocation',
    ),
  );
  checks.push(
    ...checkTextMarkers(
      path.join(ROOT, 'README.md'),
      policy.required_readme_markers || [],
      'readme_marker',
    ),
  );
  if (args.runSmoke) checks.push(...runSmokeScripts(policy.smoke_scripts || []));

  const failed = checks.filter((row) => !row.ok);
  return {
    type: 'production_readiness_closure_gate',
    generated_at: new Date().toISOString(),
    strict: args.strict,
    run_smoke: args.runSmoke,
    summary: {
      check_count: checks.length,
      failed_count: failed.length,
      pass: failed.length === 0,
    },
    failed_ids: failed.map((row) => row.id),
    checks,
  };
}

export function run(rawArgs: Args | string[]): number {
  const args = Array.isArray(rawArgs) ? parseArgs(rawArgs) : rawArgs;
  const report = buildReport(args);
  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (args.strict && report.summary.failed_count > 0) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(parseArgs(process.argv.slice(2))));
}

module.exports = {
  buildReport,
  parseArgs,
  run,
};
