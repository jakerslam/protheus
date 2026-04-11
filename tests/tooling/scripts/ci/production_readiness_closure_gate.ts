#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { executeGate } from '../../lib/runner.ts';

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
  required_verify_profile_gate_ids?: Record<string, string[]>;
  required_readme_markers?: string[];
  smoke_scripts?: string[];
};

const ROOT = process.cwd();
const POLICY_PATH = path.join(ROOT, 'client/runtime/config/production_readiness_closure_policy.json');
const VERIFY_PROFILES_PATH = path.join(ROOT, 'tests/tooling/config/verify_profiles.json');
const GATE_REGISTRY_PATH = 'tests/tooling/config/tooling_gate_registry.json';
const TOPOLOGY_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/production_topology_diagnostic_current.json');
const STATE_COMPAT_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/stateful_upgrade_rollback_gate_current.json');
const SUPPORT_BUNDLE_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/support_bundle_latest.json');
const RELEASE_SCORECARD_PATH = path.join(
  ROOT,
  'client/runtime/local/state/release/scorecard/release_scorecard.json',
);

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

function checkWorkflowMarkers(markers: string[]): Check[] {
  const workflowFiles = [
    path.join(ROOT, '.github/workflows/ci.yml'),
    path.join(ROOT, '.github/workflows/release.yml'),
  ];
  const sources = workflowFiles
    .filter((filePath) => fs.existsSync(filePath))
    .map((filePath) => fs.readFileSync(filePath, 'utf8'));
  return markers.map((marker) => {
    const ok = sources.some((source) => source.includes(marker));
    return {
      id: `ci_invocation:${marker}`,
      ok,
      detail: ok ? 'present' : 'missing_from_ci_or_release_workflow',
    };
  });
}

function checkVerifyProfileGateIds(requiredProfiles: Record<string, string[]>): Check[] {
  const manifest = readJson<{ profiles?: Record<string, { gate_ids?: string[] }> }>(
    VERIFY_PROFILES_PATH,
    {},
  );
  const profiles = manifest.profiles || {};
  const checks: Check[] = [];
  for (const [profileId, requiredGateIds] of Object.entries(requiredProfiles || {})) {
    const gateIds = Array.isArray(profiles[profileId]?.gate_ids) ? profiles[profileId]?.gate_ids || [] : [];
    if (!profiles[profileId]) {
      checks.push({
        id: `verify_profile:${profileId}`,
        ok: false,
        detail: 'missing',
      });
      continue;
    }
    for (const gateId of requiredGateIds) {
      const ok = gateIds.includes(gateId);
      checks.push({
        id: `verify_profile_gate:${profileId}:${gateId}`,
        ok,
        detail: ok ? 'present' : 'missing',
      });
    }
  }
  return checks;
}

function runSmokeScripts(scriptNames: string[]): Check[] {
  return scriptNames.map((scriptName) => {
    try {
      const report = executeGate(scriptName, {
        registryPath: GATE_REGISTRY_PATH,
        strict: true,
      });
      return {
        id: `smoke_script:${scriptName}`,
        ok: report.ok,
        detail: report.ok
          ? `ok:${report.summary.exit_code}`
          : String(report.failures[0]?.detail || 'gate_failed').slice(0, 400),
      };
    } catch (error) {
      return {
        id: `smoke_script:${scriptName}`,
        ok: false,
        detail: `unregistered_or_failed:${String(error)}`.slice(0, 400),
      };
    }
  });
}

function checkReleaseEvidence(): Check[] {
  const topology = readJson<any>(TOPOLOGY_ARTIFACT_PATH, {});
  const stateCompat = readJson<any>(STATE_COMPAT_ARTIFACT_PATH, {});
  const supportBundle = readJson<any>(SUPPORT_BUNDLE_ARTIFACT_PATH, {});
  const scorecard = readJson<any>(RELEASE_SCORECARD_PATH, {});
  const liveChecks = stateCompat?.checks || {};
  const liveRehearsalOk =
    liveChecks.live_taskgroup_rehearsal_verified === true &&
    liveChecks.live_receipt_rehearsal_verified === true &&
    liveChecks.live_memory_surface_verified === true &&
    liveChecks.live_runtime_receipt_verified === true &&
    liveChecks.live_assimilation_contract_verified === true;
  return [
    {
      id: 'release_scorecard_numeric_thresholds',
      ok: scorecard?.ok === true,
      detail: scorecard?.ok === true ? 'enforced' : 'missing_or_failed',
    },
    {
      id: 'production_topology_supported',
      ok:
        topology?.ok === true &&
        topology?.supported_production_topology === true &&
        Array.isArray(topology?.degraded_flags) &&
        topology.degraded_flags.length === 0,
      detail: `support_level=${String(topology?.support_level || 'unknown')}`,
    },
    {
      id: 'stateful_upgrade_live_rehearsal',
      ok: stateCompat?.ok === true && liveRehearsalOk,
      detail: `live_rehearsal=${liveRehearsalOk}`,
    },
    {
      id: 'support_bundle_incident_truth_package',
      ok: supportBundle?.incident_truth_package?.ready === true,
      detail: `failed_checks=${Array.isArray(supportBundle?.incident_truth_package?.failed_checks) ? supportBundle.incident_truth_package.failed_checks.length : 'missing'}`,
    },
  ];
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
  checks.push(...checkWorkflowMarkers(policy.required_ci_invocations || []));
  checks.push(
    ...checkTextMarkers(
      path.join(ROOT, 'verify.sh'),
      policy.required_verify_invocations || [],
      'verify_invocation',
    ),
  );
  checks.push(...checkVerifyProfileGateIds(policy.required_verify_profile_gate_ids || {}));
  checks.push(
    ...checkTextMarkers(
      path.join(ROOT, 'README.md'),
      policy.required_readme_markers || [],
      'readme_marker',
    ),
  );
  if (args.runSmoke) checks.push(...runSmokeScripts(policy.smoke_scripts || []));
  if (args.runSmoke) checks.push(...checkReleaseEvidence());

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
