import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-OPS-PRD-RELEASE-GATE-001';
const LEGACY_SRS_ID = 'V11-OPS-PRD-001';
const RELEASE_GATES = 'tests/tooling/config/release_gates.yaml';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const OUT_JSON = 'core/local/artifacts/production_release_gate_closure_audit_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/PRODUCTION_RELEASE_GATE_CLOSURE_AUDIT_CURRENT.md';
const GATE_ID = 'ops:production-release-gate:closure-audit';
const PROFILES = ['rich', 'pure', 'tiny-max'];
const ZERO_QUALITY_KEYS = [
  'workflow_unexpected_state_loop_max',
  'automatic_tool_trigger_events_max',
  'file_tool_route_misdirection_max',
];

type Check = {
  id: string;
  ok: boolean;
  detail?: string;
};

function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  return process.argv.find((item) => item.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function flag(name: string, fallback: boolean): boolean {
  const value = arg(name, fallback ? '1' : '0').toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

function readJson(path: string): any {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function list(value: any): string[] {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

function check(id: string, ok: boolean, detail?: string): Check {
  return detail ? { id, ok, detail } : { id, ok };
}

function sectionFor(source: string, profile: string): string {
  const lines = source.split(/\r?\n/);
  const header = `  ${profile}:`;
  const start = lines.findIndex((line) => line === header);
  if (start < 0) return '';
  const body: string[] = [];
  for (const line of lines.slice(start + 1)) {
    if (/^  [a-z0-9-]+:$/.test(line)) break;
    body.push(line);
  }
  return `${body.join('\n')}\n`;
}

function scalar(section: string, key: string): number | null {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = section.match(new RegExp(`^\\s+${escaped}:\\s*([0-9]+(?:\\.[0-9]+)?)\\s*$`, 'm'));
  return match ? Number(match[1]) : null;
}

function artifactPath(kind: 'gate' | 'metrics', profile: string): string {
  return `core/local/artifacts/runtime_proof_release_${kind}_${profile}_current.json`;
}

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function releaseGovernanceArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.release_governance);
}

function registryArtifacts(registry: any, gateId: string): string[] {
  return list(registry?.gates?.[gateId]?.artifact_paths);
}

function registryRunnable(registry: any, gateId: string): boolean {
  const entry = registry?.gates?.[gateId];
  return Boolean(entry && (Array.isArray(entry.command) || typeof entry.script === 'string'));
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function writeMarkdown(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Production Release Gate Closure Audit',
    '',
    `- pass: ${pass}`,
    `- srs_id: ${SRS_ID}`,
    `- legacy_srs_id: ${LEGACY_SRS_ID}`,
    '',
    '| Check | Status |',
    '| --- | --- |',
    ...checks.map((row) => `| ${row.id} | ${row.ok ? 'pass' : 'fail'} |`),
    '',
  ];
  writeFileSync(path, lines.join('\n'));
}

function main(): void {
  const releaseGatesPath = arg('release-gates', RELEASE_GATES);
  const manifestPath = arg('manifest', MANIFEST);
  const registryPath = arg('registry', REGISTRY);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const strict = flag('strict', true);
  const gates = readFileSync(releaseGatesPath, 'utf8');
  const manifest = readJson(manifestPath);
  const registry = readJson(registryPath);
  const required = requiredArtifacts(manifest);
  const releaseGovernance = releaseGovernanceArtifacts(manifest);
  const checks: Check[] = [
    check('release_gates_version_is_1', /^version:\s*1\s*$/m.test(gates)),
    check('closure_audit_registered_as_release_governance_artifact', releaseGovernance.includes(OUT_JSON), OUT_JSON),
    check('closure_audit_required_in_proof_pack', required.includes(OUT_JSON), OUT_JSON),
    check('closure_audit_markdown_registry_exported', registryArtifacts(registry, GATE_ID).includes(OUT_MARKDOWN), OUT_MARKDOWN),
    check('closure_audit_registry_entry_runnable', registryRunnable(registry, GATE_ID)),
  ];
  for (const profile of PROFILES) {
    const section = sectionFor(gates, profile);
    const gatePath = artifactPath('gate', profile);
    const metricsPath = artifactPath('metrics', profile);
    const gatePayload = existsSync(gatePath) ? readJson(gatePath) : null;
    const metricsPayload = existsSync(metricsPath) ? readJson(metricsPath) : null;
    checks.push(check(`release_gates_profile_present:${profile}`, section.length > 0));
    checks.push(
      check(`release_gate_artifacts_required:${profile}`, required.includes(gatePath) && required.includes(metricsPath)),
    );
    checks.push(check(`release_gate_artifact_ok:${profile}`, Boolean(gatePayload?.ok === true), gatePath));
    checks.push(check(`release_gate_profile_matches:${profile}`, gatePayload?.profile === profile, String(gatePayload?.profile ?? 'missing')));
    checks.push(check(`release_gate_selected_track_dual:${profile}`, gatePayload?.summary?.selected_track === 'dual'));
    checks.push(check(`release_gate_input_proof_track_dual:${profile}`, gatePayload?.inputs?.proof_track === 'dual'));
    checks.push(check(`release_metrics_artifact_ok:${profile}`, Boolean(metricsPayload?.ok === true), metricsPath));
    checks.push(
      check(
        `release_gate_synthetic_required_matches_policy:${profile}`,
        Number(gatePayload?.profile_requirements?.proof_tracks?.synthetic_required) === scalar(section, 'synthetic_required'),
      ),
    );
    checks.push(
      check(
        `release_gate_empirical_required_matches_policy:${profile}`,
        Number(gatePayload?.profile_requirements?.proof_tracks?.empirical_required) === scalar(section, 'empirical_required'),
      ),
    );
    checks.push(
      check(
        `release_gate_empirical_min_samples_matches_policy:${profile}`,
        Number(gatePayload?.profile_requirements?.proof_tracks?.empirical_min_sample_points) ===
          scalar(section, 'empirical_min_sample_points'),
      ),
    );
    for (const key of ZERO_QUALITY_KEYS) {
      checks.push(check(`release_gates_quality_zero_contract:${profile}:${key}`, scalar(section, key) === 0));
    }
  }
  const gateway = existsSync('core/local/artifacts/gateway_runtime_chaos_gate_current.json')
    ? readJson('core/local/artifacts/gateway_runtime_chaos_gate_current.json')
    : null;
  checks.push(
    check(
      'gateway_chaos_artifact_required_and_release_ready',
      required.includes('core/local/artifacts/gateway_runtime_chaos_gate_current.json') &&
        gateway?.ok === true &&
        Number(gateway?.metrics?.gateway_chaos_fail_closed_ratio) >= 1 &&
        Number(gateway?.metrics?.gateway_graduation_ratio) >= 1,
    ),
  );
  const windows = existsSync('core/local/artifacts/windows_installer_contract_guard_current.json')
    ? readJson('core/local/artifacts/windows_installer_contract_guard_current.json')
    : null;
  checks.push(
    check(
      'windows_installer_contract_required_and_passing',
      required.includes('core/local/artifacts/windows_installer_contract_guard_current.json') &&
        windows?.ok === true &&
        Number(windows?.summary?.failed_check_count ?? 0) === 0,
    ),
  );
  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'production_release_gate_closure_audit',
    srs_id: SRS_ID,
    legacy_srs_ids: [LEGACY_SRS_ID],
    generated_at: new Date().toISOString(),
    inputs: {
      release_gates_path: releaseGatesPath,
      manifest_path: manifestPath,
      registry_path: registryPath,
    },
    summary: {
      pass,
      check_count: checks.length,
      profile_count: PROFILES.length,
      zero_quality_keys: ZERO_QUALITY_KEYS,
    },
    checks,
    artifact_paths: [outJson, outMarkdown],
  };
  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeMarkdown(outMarkdown, checks, pass);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !pass) process.exit(1);
}

main();
