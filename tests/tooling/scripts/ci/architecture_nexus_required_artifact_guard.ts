import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const SRS_ID = 'V12-ARCH-NEXUS-REQUIRED-001';
const LEGACY_SRS_ID = 'V11-ARCH-005';
const DEFAULT_MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
const DEFAULT_REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const DEFAULT_OUT = 'core/local/artifacts/architecture_nexus_required_artifact_guard_current.json';
const DEFAULT_REPORT = 'local/workspace/reports/ARCHITECTURE_NEXUS_REQUIRED_ARTIFACT_GUARD_CURRENT.md';
const NEXUS_JSON = 'core/local/artifacts/kernel_nexus_coupling_guard_current.json';
const NEXUS_REPORT = 'local/workspace/reports/KERNEL_NEXUS_COUPLING_GUARD_CURRENT.md';
const SELF_GATE_ID = 'ops:architecture:nexus-required-artifact:guard';

type Check = {
  id: string;
  ok: boolean;
  detail?: string;
};

function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  return process.argv.find((value) => value.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function flag(name: string, fallback = false): boolean {
  const raw = arg(name, fallback ? '1' : '0').toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes';
}

function readJson(path: string): any {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function list(value: any): string[] {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

function artifactGroups(manifest: any): Map<string, string[]> {
  const groups = new Map<string, string[]>();
  const rawGroups = manifest?.artifact_groups ?? {};
  for (const [group, paths] of Object.entries(rawGroups)) {
    groups.set(group, list(paths));
  }
  return groups;
}

function registryArtifactPaths(registry: any, gateId: string): string[] {
  return list(registry?.gates?.[gateId]?.artifact_paths);
}

function registryHasRunnableEntry(registry: any, gateId: string): boolean {
  const entry = registry?.gates?.[gateId];
  return Boolean(entry && (Array.isArray(entry.command) || typeof entry.script === 'string'));
}

function check(id: string, ok: boolean, detail?: string): Check {
  return detail ? { id, ok, detail } : { id, ok };
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function writeReport(path: string, checks: Check[], pass: boolean): void {
  ensureParent(path);
  const lines = [
    '# Architecture Nexus Required Artifact Guard',
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
  const manifestPath = arg('manifest', DEFAULT_MANIFEST);
  const registryPath = arg('registry', DEFAULT_REGISTRY);
  const outJson = arg('out-json', DEFAULT_OUT);
  const outMarkdown = arg('out-markdown', DEFAULT_REPORT);
  const strict = flag('strict', true);
  const manifest = readJson(manifestPath);
  const registry = readJson(registryPath);
  const groups = artifactGroups(manifest);
  const releaseGovernance = groups.get('release_governance') ?? [];
  const requiredArtifacts = list(manifest?.required_artifacts);
  const optionalArtifacts = list(manifest?.optional_artifacts);
  const nexusRegistryArtifacts = registryArtifactPaths(registry, 'ops:nexus:kernel-coupling:guard');
  const selfRegistryArtifacts = registryArtifactPaths(registry, SELF_GATE_ID);
  const checks = [
    check(
      'kernel_nexus_json_is_release_governance_artifact',
      releaseGovernance.includes(NEXUS_JSON),
      NEXUS_JSON,
    ),
    check(
      'kernel_nexus_json_is_required_proof_pack_artifact',
      requiredArtifacts.includes(NEXUS_JSON),
      NEXUS_JSON,
    ),
    check(
      'kernel_nexus_markdown_report_is_optional_proof_pack_artifact',
      optionalArtifacts.includes(NEXUS_REPORT),
      NEXUS_REPORT,
    ),
    check(
      'kernel_nexus_gate_registry_has_runnable_entry',
      registryHasRunnableEntry(registry, 'ops:nexus:kernel-coupling:guard'),
    ),
    check(
      'kernel_nexus_gate_registry_exports_json_and_markdown',
      nexusRegistryArtifacts.includes(NEXUS_JSON) && nexusRegistryArtifacts.includes(NEXUS_REPORT),
    ),
    check(
      'nexus_required_artifact_guard_is_release_governance_artifact',
      releaseGovernance.includes(DEFAULT_OUT),
      DEFAULT_OUT,
    ),
    check(
      'nexus_required_artifact_guard_is_required_proof_pack_artifact',
      requiredArtifacts.includes(DEFAULT_OUT),
      DEFAULT_OUT,
    ),
    check(
      'nexus_required_artifact_guard_markdown_is_optional_proof_pack_artifact',
      optionalArtifacts.includes(DEFAULT_REPORT),
      DEFAULT_REPORT,
    ),
    check('nexus_required_artifact_guard_has_registry_entry', registryHasRunnableEntry(registry, SELF_GATE_ID)),
    check(
      'nexus_required_artifact_guard_registry_exports_json_and_markdown',
      selfRegistryArtifacts.includes(DEFAULT_OUT) && selfRegistryArtifacts.includes(DEFAULT_REPORT),
    ),
  ];
  const pass = checks.every((row) => row.ok);
  const payload = {
    ok: pass,
    type: 'architecture_nexus_required_artifact_guard',
    srs_id: SRS_ID,
    legacy_srs_ids: [LEGACY_SRS_ID],
    generated_at: new Date().toISOString(),
    inputs: { manifest_path: manifestPath, registry_path: registryPath },
    summary: {
      pass,
      check_count: checks.length,
      release_governance_artifact_count: releaseGovernance.length,
      required_artifact_count: requiredArtifacts.length,
    },
    checks,
    artifact_paths: [outJson, outMarkdown],
  };
  ensureParent(outJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeReport(outMarkdown, checks, pass);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && !pass) process.exit(1);
}

main();
