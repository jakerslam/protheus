#!/usr/bin/env tsx

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const REGISTRY_PATH = 'validation/tests/contracts/validation_test_lifecycle_registry.json';
const RETIRED_TEST_MATURITY_MIRROR_PATH = ['tests', 'tooling', 'config', 'test_maturity_registry.json'].join('/');
const SUITE_MANIFEST_PATH = 'validation/tests/contracts/test_suite_lifecycle_manifests.json';
const ARTIFACT_ENVELOPE_CONTRACT_PATH = 'validation/tests/contracts/test_lifecycle_artifact_envelope_contract.json';
const TEMPORARY_TEST_CONTRACT_PATH = 'validation/tests/contracts/temporary_test_lifecycle_contract.json';
const PHYSICAL_PLACEMENT_POLICY_PATH = 'validation/tests/contracts/test_lifecycle_physical_placement_policy.json';
const UNREGISTERED_TEST_POLICY_PATH = 'validation/tests/contracts/unregistered_test_policy.json';
const TOOLING_REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PROFILES = 'tests/tooling/config/verify_profiles.json';
const MANIFEST = 'validation/release_gates/contracts/release_proof_pack_manifest.json';
const PACKAGE_JSON = 'package.json';
const OUT_JSON = 'core/local/artifacts/test_maturity_registry_current.json';
const OUT_MARKDOWN = 'local/workspace/reports/TEST_MATURITY_REGISTRY_CURRENT.md';
const RETIREMENT_JSON = 'core/local/artifacts/test_maturity_retirement_backlog_current.json';
const RETIREMENT_MARKDOWN = 'local/workspace/reports/TEST_MATURITY_RETIREMENT_BACKLOG.md';
const MONITOR_HISTORY = 'local/state/ops/test_maturity_monitor_history.json';
const GATE_ID = 'ops:test-maturity:registry:guard';
const ALLOWED = new Set([
  'self_enforcement_validator',
  'temporary_scaffold',
  'temporary_monitor',
  'runtime_crutch',
  'release_evidence_gate',
  'architecture_drift_guard',
  'one_time_closure_guard',
]);
const SCAFFOLD_CLASSES = new Set(['temporary_scaffold', 'temporary_monitor', 'one_time_closure_guard']);
const TEMPORARY_CLASSES = new Set([...SCAFFOLD_CLASSES, 'runtime_crutch']);
const MONITOR_STATES = new Set([
  'observing',
  'eligible_for_retirement',
  'retirement_backlog_created',
  'blocked_by_regression',
  'needs_runtime_strengthening',
  'insufficient_signal',
]);

type Entry = Record<string, any> & { gate_id: string; classification: string };
type Finding = { id: string; severity: 'fail' | 'warn' | 'info'; gate_id?: string; detail: string };
type MonitorRun = { at: string; ok: boolean | null; observation: 'pass' | 'fail' | 'unknown'; source_artifact?: string; detail?: string };
type MonitorHistory = {
  schema_version: number;
  updated_at: string;
  runs: Record<string, MonitorRun[]>;
  retirement_backlog: Record<string, any>;
};

type MonitorStatus = {
  gate_id: string;
  state: string;
  source_artifact?: string;
  runs_observed: number;
  window_runs: number;
  passes: number;
  failures: number;
  unknown: number;
  success_rate: number;
  consecutive_passes: number;
  criteria: Record<string, any>;
  migration_target?: string;
  retirement_action?: string;
  retirement_backlog_id?: string;
  detail: string;
};

type SuiteManifest = {
  id: string;
  classification: string;
  owner: string;
  runtime_owner: string;
  invariant: string;
  evidence_artifact: string;
  lifecycle_state: string;
  temporary: boolean;
  managed_gate_ids?: string[];
};

type LifecycleArtifactEnvelope = {
  ok: boolean;
  test_id: string;
  classification: string;
  invariant: string;
  runtime_owner: string;
  failure_signature: string | null;
  strengthen_signal: string;
};

function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  return process.argv.find((item) => item.startsWith(prefix))?.slice(prefix.length) ?? fallback;
}

function flag(name: string, fallback: boolean): boolean {
  const value = arg(name, fallback ? '1' : '0').toLowerCase();
  return value === '1' || value === 'true' || value === 'yes';
}

function readText(path: string): string {
  return readFileSync(path, 'utf8');
}

function readJson(path: string): any {
  return JSON.parse(readText(path));
}

function readJsonIfExists(path: string, fallback: any): any {
  if (!existsSync(path)) return fallback;
  try {
    return readJson(path);
  } catch {
    return fallback;
  }
}

function stableJson(value: any): string {
  return JSON.stringify(value);
}

function list(value: any): string[] {
  return Array.isArray(value) ? value.filter((item) => typeof item === 'string') : [];
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function hasText(value: unknown): boolean {
  return typeof value === 'string' && value.trim().length > 0;
}

function requiredArtifacts(manifest: any): string[] {
  return list(manifest?.required_artifacts);
}

function releaseGovernanceArtifacts(manifest: any): string[] {
  return list(manifest?.artifact_groups?.release_governance);
}

function optionalReports(manifest: any): string[] {
  return list(manifest?.optional_reports);
}

function profileGateIds(profiles: any, profile: string): string[] {
  return list(profiles?.profiles?.[profile]?.gate_ids);
}

function packageScript(pkg: any, name: string): string {
  const value = pkg?.scripts?.[name];
  return typeof value === 'string' ? value : '';
}

function gateIds(tooling: any): string[] {
  return Object.keys(tooling?.gates ?? {}).sort();
}

function registryArtifacts(tooling: any, gateId: string): string[] {
  return list(tooling?.gates?.[gateId]?.artifact_paths);
}

function addFinding(findings: Finding[], id: string, severity: Finding['severity'], detail: string, gate_id?: string): void {
  findings.push(gate_id ? { id, severity, gate_id, detail } : { id, severity, detail });
}

function daysUntil(raw: string, now: Date): number | null {
  if (!hasText(raw)) return null;
  const date = new Date(`${raw}T00:00:00Z`);
  if (Number.isNaN(date.getTime())) return null;
  return Math.ceil((date.getTime() - now.getTime()) / 86_400_000);
}

function requireFields(entry: Entry, fields: string[], findings: Finding[]): void {
  for (const field of fields) {
    if (!hasText(entry[field])) {
      addFinding(findings, 'missing_required_field', 'fail', `${entry.classification} requires ${field}`, entry.gate_id);
    }
  }
}

function validateSuiteManifests(path: string, payload: any, classifiedIds: Set<string>, findings: Finding[]): Record<string, any> {
  const states = new Set(list(payload?.lifecycle_states));
  const suites: SuiteManifest[] = Array.isArray(payload?.suites) ? payload.suites : [];
  const seen = new Set<string>();
  if (!path.startsWith('validation/tests/')) addFinding(findings, 'suite_manifest_not_validation_owned', 'fail', 'Suite lifecycle manifest must live under validation/tests.', GATE_ID);
  if (!payload?.schema_version) addFinding(findings, 'suite_manifest_missing_schema_version', 'fail', 'Suite lifecycle manifest must declare schema_version.', GATE_ID);
  if (suites.length === 0) addFinding(findings, 'suite_manifest_empty', 'fail', 'Suite lifecycle manifest must declare at least one suite.', GATE_ID);
  for (const suite of suites) {
    const id = hasText(suite?.id) ? suite.id : 'unknown_suite';
    if (seen.has(id)) addFinding(findings, 'duplicate_suite_manifest_id', 'fail', 'Suite lifecycle manifest id is duplicated.', id);
    seen.add(id);
    for (const field of ['id', 'classification', 'owner', 'runtime_owner', 'invariant', 'evidence_artifact', 'lifecycle_state']) {
      if (!hasText((suite as any)?.[field])) addFinding(findings, 'suite_manifest_missing_field', 'fail', `Suite lifecycle manifest requires ${field}.`, id);
    }
    if (typeof suite?.temporary !== 'boolean') addFinding(findings, 'suite_manifest_missing_temporary_flag', 'fail', 'Suite lifecycle manifest requires boolean temporary.', id);
    if (!ALLOWED.has(suite?.classification)) addFinding(findings, 'suite_manifest_invalid_classification', 'fail', `Invalid suite classification: ${suite?.classification}`, id);
    if (!states.has(suite?.lifecycle_state)) addFinding(findings, 'suite_manifest_invalid_lifecycle_state', 'fail', `Invalid lifecycle_state: ${suite?.lifecycle_state}`, id);
    const gateIds = list(suite?.managed_gate_ids);
    if (gateIds.length === 0) addFinding(findings, 'suite_manifest_missing_managed_gates', 'fail', 'Suite lifecycle manifest must map to at least one managed gate.', id);
    for (const gateId of gateIds) {
      if (!classifiedIds.has(gateId)) addFinding(findings, 'suite_manifest_unclassified_gate', 'fail', `Suite references unclassified gate ${gateId}.`, id);
    }
  }
  return {
    suite_manifest_path: path,
    suite_count: suites.length,
    temporary_suite_count: suites.filter((suite) => suite.temporary === true).length,
    managed_gate_count: new Set(suites.flatMap((suite) => list(suite.managed_gate_ids))).size,
  };
}

function validateArtifactEnvelopeContract(path: string, payload: any, findings: Finding[]): Record<string, any> {
  const required = list(payload?.required_fields);
  const expected = ['ok', 'test_id', 'classification', 'invariant', 'runtime_owner', 'failure_signature', 'strengthen_signal'];
  if (!path.startsWith('validation/tests/')) addFinding(findings, 'artifact_envelope_contract_not_validation_owned', 'fail', 'Artifact envelope contract must live under validation/tests.', GATE_ID);
  if (!payload?.schema_version) addFinding(findings, 'artifact_envelope_contract_missing_schema_version', 'fail', 'Artifact envelope contract must declare schema_version.', GATE_ID);
  for (const field of expected) {
    if (!required.includes(field)) addFinding(findings, 'artifact_envelope_contract_missing_required_field', 'fail', `Artifact envelope contract must require ${field}.`, GATE_ID);
  }
  return { artifact_envelope_contract_path: path, required_fields: required };
}

function validateTemporaryTestContract(path: string, payload: any, findings: Finding[]): Record<string, any> {
  const requiredFields = list(payload?.required_fields);
  const requiredCriteria = list(payload?.required_success_criteria_fields);
  const expectedFields = ['why_it_exists', 'runtime_weakness_signal', 'target_runtime_owner', 'expires_after', 'migration_target', 'delete_when', 'success_criteria'];
  const expectedCriteria = ['minimum_runs', 'success_rate_required', 'observation_window_days', 'consecutive_passes_required', 'max_regressions_allowed'];
  if (!path.startsWith('validation/tests/')) addFinding(findings, 'temporary_test_contract_not_validation_owned', 'fail', 'Temporary test lifecycle contract must live under validation/tests.', GATE_ID);
  if (!payload?.schema_version) addFinding(findings, 'temporary_test_contract_missing_schema_version', 'fail', 'Temporary test lifecycle contract must declare schema_version.', GATE_ID);
  for (const field of expectedFields) {
    if (!requiredFields.includes(field)) addFinding(findings, 'temporary_test_contract_missing_required_field', 'fail', `Temporary test lifecycle contract must require ${field}.`, GATE_ID);
  }
  for (const field of expectedCriteria) {
    if (!requiredCriteria.includes(field)) addFinding(findings, 'temporary_test_contract_missing_success_criteria_field', 'fail', `Temporary test lifecycle contract must require success_criteria.${field}.`, GATE_ID);
  }
  return { temporary_test_contract_path: path, required_fields: requiredFields, required_success_criteria_fields: requiredCriteria };
}

function validatePhysicalPlacementPolicy(
  path: string,
  payload: any,
  paths: Record<string, string>,
  tooling: any,
  suiteManifest: any,
  findings: Finding[],
): Record<string, any> {
  const canonicalPrefixes = list(payload?.canonical_definition_prefixes);
  const harnessPrefixes = list(payload?.harness_only_prefixes);
  const forbiddenPrefixes = list(payload?.forbidden_canonical_prefixes);
  const mirrors = Array.isArray(payload?.compatibility_mirrors) ? payload.compatibility_mirrors : [];
  if (!path.startsWith('validation/tests/')) addFinding(findings, 'physical_placement_policy_not_validation_owned', 'fail', 'Physical placement policy must live under validation/tests.', GATE_ID);
  if (!payload?.schema_version) addFinding(findings, 'physical_placement_policy_missing_schema_version', 'fail', 'Physical placement policy must declare schema_version.', GATE_ID);
  if (!canonicalPrefixes.includes('validation/tests/')) addFinding(findings, 'physical_placement_policy_missing_tests_prefix', 'fail', 'Physical placement policy must include validation/tests/.', GATE_ID);
  if (!harnessPrefixes.includes('tests/tooling/scripts/')) addFinding(findings, 'physical_placement_policy_missing_harness_prefix', 'fail', 'Physical placement policy must keep tests/tooling/scripts as harness-only.', GATE_ID);
  if (mirrors.length > 0) addFinding(findings, 'physical_placement_policy_has_retired_registry_mirror', 'fail', 'Physical placement policy must not retain retired test maturity compatibility mirrors.', GATE_ID);
  for (const [name, value] of Object.entries(paths)) {
    if (!canonicalPrefixes.some((prefix) => value.startsWith(prefix))) {
      addFinding(findings, 'validation_definition_not_in_canonical_domain', 'fail', `${name} must live under a Validation canonical definition prefix: ${value}`, GATE_ID);
    }
    if (forbiddenPrefixes.some((prefix) => value.startsWith(prefix))) {
      addFinding(findings, 'validation_definition_in_forbidden_tooling_prefix', 'fail', `${name} cannot use tests/tooling as canonical definition storage: ${value}`, GATE_ID);
    }
  }
  const command = list(tooling?.gates?.[GATE_ID]?.command).join(' ');
  if (!command.includes(`--registry=${REGISTRY_PATH}`)) addFinding(findings, 'lifecycle_guard_registry_not_validation_owned', 'fail', `${GATE_ID} must read the Validation-owned lifecycle registry.`, GATE_ID);
  if (command.includes('--compatibility-registry=') || command.includes(RETIRED_TEST_MATURITY_MIRROR_PATH)) addFinding(findings, 'lifecycle_guard_retired_compatibility_mirror_arg', 'fail', `${GATE_ID} must not read the retired test maturity compatibility mirror.`, GATE_ID);
  const suites = Array.isArray(suiteManifest?.suites) ? suiteManifest.suites : [];
  for (const suite of suites) {
    if (!canonicalPrefixes.some((prefix) => String(suite?.owner ?? '').startsWith(prefix.replace(/\/$/, '')))) {
      addFinding(findings, 'suite_owner_not_validation_domain', 'fail', `Suite ${suite?.id ?? 'unknown'} owner must be under Validation domains.`, String(suite?.id ?? GATE_ID));
    }
  }
  return {
    physical_placement_policy_path: path,
    canonical_definition_prefixes: canonicalPrefixes,
    harness_only_prefixes: harnessPrefixes,
    compatibility_mirror_count: mirrors.length,
    validation_owned_input_count: Object.keys(paths).length,
  };
}

function validateUnregisteredTestPolicy(path: string, payload: any, toolingIds: string[], classifiedIds: Set<string>, now: Date, findings: Finding[]): Record<string, any> {
  const harnessOnly = new Set(list(payload?.harness_only_gate_ids));
  const exemptions = Array.isArray(payload?.exemptions) ? payload.exemptions : [];
  const exemptIds = new Set<string>();
  if (!path.startsWith('validation/tests/')) addFinding(findings, 'unregistered_test_policy_not_validation_owned', 'fail', 'Unregistered test policy must live under validation/tests.', GATE_ID);
  if (!payload?.schema_version) addFinding(findings, 'unregistered_test_policy_missing_schema_version', 'fail', 'Unregistered test policy must declare schema_version.', GATE_ID);
  if (payload?.registered_registry_path !== REGISTRY_PATH) addFinding(findings, 'unregistered_test_policy_registry_mismatch', 'fail', 'Unregistered test policy must point at the canonical Validation lifecycle registry.', GATE_ID);
  if (payload?.suite_manifest_path !== SUITE_MANIFEST_PATH) addFinding(findings, 'unregistered_test_policy_suite_manifest_mismatch', 'fail', 'Unregistered test policy must point at the canonical suite manifest.', GATE_ID);
  for (const gateId of harnessOnly) {
    if (!toolingIds.includes(gateId)) addFinding(findings, 'harness_only_gate_not_registered', 'fail', 'Harness-only declaration references an unknown tooling gate.', gateId);
    if (classifiedIds.has(gateId)) addFinding(findings, 'harness_only_gate_also_lifecycle_registered', 'fail', 'Gate cannot be both harness_only and lifecycle registered.', gateId);
  }
  for (const row of exemptions) {
    const gateId = String(row?.gate_id ?? '');
    for (const field of ['gate_id', 'owner', 'reason', 'expires_after']) {
      if (!hasText(row?.[field])) addFinding(findings, 'unregistered_test_exemption_missing_field', 'fail', `Exemption requires ${field}.`, gateId || GATE_ID);
    }
    const days = daysUntil(String(row?.expires_after ?? ''), now);
    if (days === null) addFinding(findings, 'unregistered_test_exemption_invalid_expiry', 'fail', 'Exemption expiry must be YYYY-MM-DD.', gateId || GATE_ID);
    else if (days < 0) addFinding(findings, 'unregistered_test_exemption_expired', 'fail', `Exemption expired ${Math.abs(days)} day(s) ago.`, gateId || GATE_ID);
    else if (gateId) exemptIds.add(gateId);
  }
  const allowedUnregisteredIds = [...new Set([...harnessOnly, ...exemptIds])].sort();
  const unregisteredGateIds = toolingIds.filter((id) => !classifiedIds.has(id) && !harnessOnly.has(id) && !exemptIds.has(id));
  for (const id of unregisteredGateIds.slice(0, 100)) {
    addFinding(findings, 'unregistered_test_or_gate', 'fail', 'Every registered tooling gate must be lifecycle-registered, harness_only, or explicitly exempted.', id);
  }
  return {
    unregistered_test_policy_path: path,
    harness_only_gate_count: harnessOnly.size,
    exemption_count: exemptions.length,
    allowed_unregistered_ids: allowedUnregisteredIds,
    unregistered_gate_count: unregisteredGateIds.length,
    unregistered_gate_ids: unregisteredGateIds,
  };
}

function validateTemporaryLifecycleEntry(entry: Entry, findings: Finding[]): void {
  if (!TEMPORARY_CLASSES.has(entry.classification)) return;
  requireFields(entry, ['why_it_exists', 'runtime_weakness_signal', 'target_runtime_owner', 'expires_after', 'migration_target', 'delete_when'], findings);
  if (!entry.success_criteria || typeof entry.success_criteria !== 'object' || Array.isArray(entry.success_criteria)) {
    addFinding(findings, 'temporary_lifecycle_missing_success_criteria', 'fail', 'Temporary lifecycle entries require success_criteria object.', entry.gate_id);
    return;
  }
  const criteria = entry.success_criteria as Record<string, unknown>;
  for (const field of ['minimum_runs', 'success_rate_required', 'observation_window_days', 'consecutive_passes_required', 'max_regressions_allowed']) {
    if (!Number.isFinite(Number(criteria[field]))) addFinding(findings, 'temporary_lifecycle_missing_success_criteria_field', 'fail', `Temporary lifecycle entries require numeric success_criteria.${field}.`, entry.gate_id);
  }
}

function runtimeOwnerFor(entry: Entry): string {
  return String(entry.target_runtime_owner || entry.runtime_enforced_by || entry.drift_surface || entry.validation_scope || 'unspecified');
}

function buildLifecycleArtifactEnvelopes(entries: Entry[], findings: Finding[], statuses: MonitorStatus[]): LifecycleArtifactEnvelope[] {
  const statusByGate = new Map(statuses.map((status) => [status.gate_id, status]));
  return entries.map((entry) => {
    const related = findings.find((finding) => finding.gate_id === entry.gate_id && finding.severity !== 'info');
    const status = statusByGate.get(entry.gate_id);
    const failureSignature = related
      ? `${related.id}:${entry.gate_id}`
      : status && (status.state === 'blocked_by_regression' || status.state === 'needs_runtime_strengthening')
        ? `${status.state}:${entry.gate_id}`
        : null;
    return {
      ok: failureSignature === null,
      test_id: entry.gate_id,
      classification: entry.classification,
      invariant: String(entry.invariant || ''),
      runtime_owner: runtimeOwnerFor(entry),
      failure_signature: failureSignature,
      strengthen_signal: String(entry.strengthen_signal || ''),
    };
  });
}

function buildRuntimeStrengtheningFeedback(entries: Entry[], findings: Finding[], statuses: MonitorStatus[], envelopes: LifecycleArtifactEnvelope[]): Record<string, any> {
  const entryByGate = new Map(entries.map((entry) => [entry.gate_id, entry]));
  const statusByGate = new Map(statuses.map((status) => [status.gate_id, status]));
  const findingsByGate = new Map<string, Finding[]>();
  for (const finding of findings) {
    if (!finding.gate_id || finding.severity === 'info') continue;
    findingsByGate.set(finding.gate_id, [...(findingsByGate.get(finding.gate_id) ?? []), finding]);
  }
  const targets: Record<string, any> = {};
  function target(owner: string): any {
    if (!targets[owner]) {
      targets[owner] = {
        runtime_owner: owner,
        affected_gates: [],
        failure_signatures: [],
        blocked_monitors: [],
        needs_runtime_strengthening: [],
        strengthen_signals: [],
        recommended_action: `Strengthen ${owner} runtime self-enforcement so Validation can retire scaffold/monitor coverage instead of carrying it as a standing check.`,
      };
    }
    return targets[owner];
  }
  for (const envelope of envelopes) {
    const entry = entryByGate.get(envelope.test_id);
    if (!entry) continue;
    const status = statusByGate.get(envelope.test_id);
    const relatedFindings = findingsByGate.get(envelope.test_id) ?? [];
    const needsAttention = !envelope.ok || status?.state === 'blocked_by_regression' || status?.state === 'needs_runtime_strengthening' || relatedFindings.length > 0;
    if (!needsAttention) continue;
    const row = target(envelope.runtime_owner);
    row.affected_gates.push(envelope.test_id);
    if (envelope.failure_signature) row.failure_signatures.push(envelope.failure_signature);
    if (status?.state === 'blocked_by_regression') row.blocked_monitors.push(envelope.test_id);
    if (status?.state === 'needs_runtime_strengthening') row.needs_runtime_strengthening.push(envelope.test_id);
    if (envelope.strengthen_signal) row.strengthen_signals.push(envelope.strengthen_signal);
    for (const finding of relatedFindings) row.failure_signatures.push(`${finding.id}:${envelope.test_id}`);
  }
  const normalizedTargets = Object.values(targets).map((row: any) => ({
    ...row,
    affected_gates: [...new Set(row.affected_gates)].sort(),
    affected_gate_count: new Set(row.affected_gates).size,
    failure_signatures: [...new Set(row.failure_signatures)].sort(),
    blocked_monitors: [...new Set(row.blocked_monitors)].sort(),
    needs_runtime_strengthening: [...new Set(row.needs_runtime_strengthening)].sort(),
    strengthen_signals: [...new Set(row.strengthen_signals)].sort(),
  })).sort((a: any, b: any) => b.affected_gate_count - a.affected_gate_count || String(a.runtime_owner).localeCompare(String(b.runtime_owner)));
  return {
    total_targets: normalizedTargets.length,
    targets: normalizedTargets,
  };
}

function classifyStrengthenTargets(entries: Entry[]): Record<string, { scaffolds: number; runtime_crutches: number; temporary_monitors: number; gates: string[] }> {
  const rows: Record<string, { scaffolds: number; runtime_crutches: number; temporary_monitors: number; gates: string[] }> = {};
  for (const entry of entries) {
    const owner = String(entry.target_runtime_owner || entry.runtime_enforced_by || 'unspecified');
    if (!rows[owner]) rows[owner] = { scaffolds: 0, runtime_crutches: 0, temporary_monitors: 0, gates: [] };
    if (SCAFFOLD_CLASSES.has(entry.classification)) rows[owner].scaffolds += 1;
    if (entry.classification === 'runtime_crutch') rows[owner].runtime_crutches += 1;
    if (entry.classification === 'temporary_monitor') rows[owner].temporary_monitors += 1;
    if (SCAFFOLD_CLASSES.has(entry.classification) || entry.classification === 'runtime_crutch') rows[owner].gates.push(entry.gate_id);
  }
  return rows;
}

function defaultHistory(): MonitorHistory {
  return { schema_version: 1, updated_at: new Date().toISOString(), runs: {}, retirement_backlog: {} };
}

function normalizedHistory(raw: any): MonitorHistory {
  const history = raw && typeof raw === 'object' ? raw : defaultHistory();
  return {
    schema_version: Number(history.schema_version || 1),
    updated_at: typeof history.updated_at === 'string' ? history.updated_at : new Date().toISOString(),
    runs: history.runs && typeof history.runs === 'object' ? history.runs : {},
    retirement_backlog: history.retirement_backlog && typeof history.retirement_backlog === 'object' ? history.retirement_backlog : {},
  };
}

function monitorSourceArtifact(entry: Entry, tooling: any): string | undefined {
  if (hasText(entry.monitor_source_artifact)) return String(entry.monitor_source_artifact);
  return registryArtifacts(tooling, entry.gate_id).find((path) => path.endsWith('.json'));
}

function artifactObservation(path: string | undefined): MonitorRun {
  if (!path) return { at: new Date().toISOString(), ok: null, observation: 'unknown', detail: 'no source artifact declared' };
  if (!existsSync(path)) return { at: new Date().toISOString(), ok: null, observation: 'unknown', source_artifact: path, detail: 'source artifact missing' };
  try {
    const payload = readJson(path);
    if (payload?.ok === true) return { at: new Date().toISOString(), ok: true, observation: 'pass', source_artifact: path };
    if (payload?.ok === false) {
      const failedIds = Array.isArray(payload.failed_ids)
        ? payload.failed_ids.filter((row: unknown) => typeof row === 'string')
        : [];
      const failedChecks = Array.isArray(payload.checks)
        ? payload.checks
            .filter((row: any) => row && row.ok === false)
            .map((row: any) => `${String(row.id ?? 'unknown')}: ${String(row.detail ?? 'failed')}`)
        : [];
      const detail = failedChecks.length > 0
        ? failedChecks.join('; ')
        : failedIds.length > 0
          ? `failed_ids=${failedIds.join(',')}`
          : 'source artifact ok=false';
      return { at: new Date().toISOString(), ok: false, observation: 'fail', source_artifact: path, detail };
    }
    return { at: new Date().toISOString(), ok: null, observation: 'unknown', source_artifact: path, detail: 'source artifact has no boolean ok field' };
  } catch (error) {
    return { at: new Date().toISOString(), ok: null, observation: 'unknown', source_artifact: path, detail: `source artifact parse failed: ${error instanceof Error ? error.message : String(error)}` };
  }
}

function trimHistory(runs: MonitorRun[], cap = 120): MonitorRun[] {
  return runs.slice(-cap);
}

function consecutivePasses(runs: MonitorRun[]): number {
  let count = 0;
  for (let i = runs.length - 1; i >= 0; i -= 1) {
    if (runs[i].ok !== true) break;
    count += 1;
  }
  return count;
}

function windowRuns(runs: MonitorRun[], days: number, now: Date): MonitorRun[] {
  if (!Number.isFinite(days) || days <= 0) return runs;
  const threshold = now.getTime() - days * 86_400_000;
  return runs.filter((run) => {
    const parsed = Date.parse(run.at);
    return Number.isFinite(parsed) && parsed >= threshold;
  });
}

function criteriaFor(entry: Entry): Record<string, any> {
  const raw = entry.success_criteria && typeof entry.success_criteria === 'object' ? entry.success_criteria : {};
  return {
    minimum_runs: Math.max(1, Number(raw.minimum_runs ?? 30)),
    success_rate_required: Math.min(1, Math.max(0, Number(raw.success_rate_required ?? 0.98))),
    observation_window_days: Math.max(1, Number(raw.observation_window_days ?? 14)),
    consecutive_passes_required: Math.max(0, Number(raw.consecutive_passes_required ?? 10)),
    max_regressions_allowed: Math.max(0, Number(raw.max_regressions_allowed ?? 0)),
  };
}

function backlogIdFor(gateId: string): string {
  const slug = gateId.replace(/^ops:/, '').replace(/[^A-Za-z0-9]+/g, '-').replace(/^-|-$/g, '').toUpperCase();
  return `TEST-RETIRE-${slug}`;
}

function retirementActionItem(entry: Entry, status: MonitorStatus): Record<string, any> {
  return {
    id: status.retirement_backlog_id,
    action: 'operator_review_retire_or_merge_temporary_check',
    gate_id: entry.gate_id,
    classification: entry.classification,
    status: status.state,
    runtime_owner: entry.target_runtime_owner ?? entry.runtime_enforced_by ?? 'unspecified',
    migration_target: entry.migration_target,
    delete_when: entry.delete_when,
    success_criteria: status.criteria,
    observed: {
      window_runs: status.window_runs,
      passes: status.passes,
      failures: status.failures,
      unknown: status.unknown,
      success_rate: status.success_rate,
      consecutive_passes: status.consecutive_passes,
    },
    review_required: true,
    auto_delete_allowed: false,
    validation_commands: [
      'ops:test-maturity:registry:guard',
      entry.gate_id,
      'ops:srs:full:regression',
      'ops:churn:guard',
    ],
  };
}

function retirementText(entry: Entry, status: MonitorStatus): string {
  const action = retirementActionItem(entry, status);
  return [
    `- [ ] \`${action.id}\` Retire or merge temporary monitor \`${entry.gate_id}\`.`,
    `  Criteria met: ${status.passes}/${status.window_runs} passes in the observation window, success_rate=${status.success_rate}, consecutive_passes=${status.consecutive_passes}.`,
    `  Migration target: ${entry.migration_target}.`,
    `  Delete when: ${entry.delete_when}.`,
    `  Review required: ${action.review_required}; auto-delete allowed: ${action.auto_delete_allowed}.`,
    `  Validate after cleanup: \`ops:test-maturity:registry:guard\`, target gate(s), \`ops:srs:full:regression\`, and \`ops:churn:guard\`.`,
  ].join('\n');
}

function evaluateMonitor(entry: Entry, history: MonitorHistory, tooling: any, now: Date, updateHistory: boolean): MonitorStatus {
  const source = monitorSourceArtifact(entry, tooling);
  const observation = artifactObservation(source);
  const existing = Array.isArray(history.runs[entry.gate_id]) ? history.runs[entry.gate_id] : [];
  const runs = updateHistory ? trimHistory([...existing, observation]) : trimHistory(existing);
  if (updateHistory) history.runs[entry.gate_id] = runs;
  const criteria = criteriaFor(entry);
  const scoped = windowRuns(runs, criteria.observation_window_days, now);
  const passes = scoped.filter((run) => run.ok === true).length;
  const failures = scoped.filter((run) => run.ok === false).length;
  const unknown = scoped.filter((run) => run.ok === null).length;
  const successRate = scoped.length === 0 ? 0 : Number((passes / scoped.length).toFixed(4));
  const streak = consecutivePasses(runs);
  const enoughRuns = scoped.length >= criteria.minimum_runs;
  const regressionBlocked = failures > criteria.max_regressions_allowed;
  const meetsCriteria = enoughRuns && successRate >= criteria.success_rate_required && streak >= criteria.consecutive_passes_required && !regressionBlocked && unknown === 0;
  const days = daysUntil(entry.expires_after, now);
  const expired = days !== null && days < 0;
  const backlogId = backlogIdFor(entry.gate_id);
  let state = 'observing';
  let detail = `collecting evidence (${scoped.length}/${criteria.minimum_runs} required runs)`;
  if (regressionBlocked) {
    state = 'blocked_by_regression';
    detail = `${failures} regression(s) exceed allowed ${criteria.max_regressions_allowed}`;
  } else if (meetsCriteria) {
    if (!history.retirement_backlog[entry.gate_id] && updateHistory) {
      history.retirement_backlog[entry.gate_id] = {
        id: backlogId,
        created_at: new Date().toISOString(),
        gate_id: entry.gate_id,
        status: 'retirement_backlog_created',
        migration_target: entry.migration_target,
        delete_when: entry.delete_when,
      };
    }
    state = history.retirement_backlog[entry.gate_id] ? 'retirement_backlog_created' : 'eligible_for_retirement';
    detail = 'success criteria met; retirement backlog item available';
  } else if (expired) {
    state = 'needs_runtime_strengthening';
    detail = 'monitor review date passed before success criteria were met';
  } else if (scoped.length === 0 || unknown > 0) {
    state = 'insufficient_signal';
    detail = unknown > 0 ? `${unknown} unknown observation(s) in window` : 'no observations in window';
  }
  if (!MONITOR_STATES.has(state)) state = 'observing';
  return {
    gate_id: entry.gate_id,
    state,
    source_artifact: source,
    runs_observed: runs.length,
    window_runs: scoped.length,
    passes,
    failures,
    unknown,
    success_rate: successRate,
    consecutive_passes: streak,
    criteria,
    migration_target: entry.migration_target,
    retirement_action: entry.retirement_action,
    retirement_backlog_id: backlogId,
    detail,
  };
}

function writeRetirementBacklog(path: string, statuses: MonitorStatus[], entries: Entry[]): void {
  ensureParent(path);
  const byId = new Map(entries.map((entry) => [entry.gate_id, entry]));
  const candidates = statuses.filter((row) => row.state === 'eligible_for_retirement' || row.state === 'retirement_backlog_created');
  const blocked = statuses.filter((row) => row.state === 'blocked_by_regression' || row.state === 'needs_runtime_strengthening');
  const observing = statuses.filter((row) => row.state === 'observing' || row.state === 'insufficient_signal');
  const lines = [
    '# Test Maturity Retirement Backlog',
    '',
    `- generated_at: ${new Date().toISOString()}`,
    `- retirement_candidates: ${candidates.length}`,
    `- blocked_or_strengthen: ${blocked.length}`,
    `- observing: ${observing.length}`,
    '',
    '## Retirement Candidates',
    '',
    ...(candidates.length === 0 ? ['- None yet.'] : candidates.map((status) => retirementText(byId.get(status.gate_id)!, status))),
    '',
    '## Blocked / Strengthen Runtime',
    '',
    ...(blocked.length === 0 ? ['- None.'] : blocked.map((status) => `- \`${status.gate_id}\` — ${status.state}: ${status.detail}; strengthen target: ${byId.get(status.gate_id)?.target_runtime_owner ?? byId.get(status.gate_id)?.runtime_enforced_by ?? 'unspecified'}`)),
    '',
    '## Observing',
    '',
    ...(observing.length === 0 ? ['- None.'] : observing.map((status) => `- \`${status.gate_id}\` — ${status.detail}; ${status.passes}/${status.window_runs} passes, streak=${status.consecutive_passes}`)),
    '',
  ];
  writeFileSync(path, lines.join('\n'));
}

function buildScaffoldAudit(entries: Entry[], statuses: MonitorStatus[]): Record<string, any> {
  const statusByGate = new Map(statuses.map((status) => [status.gate_id, status]));
  const scaffoldEntries = entries.filter((entry) => SCAFFOLD_CLASSES.has(entry.classification));
  const alreadyRuntimeEnforced = scaffoldEntries
    .filter((entry) => hasText(entry.runtime_enforced_by))
    .map((entry) => ({
      gate_id: entry.gate_id,
      classification: entry.classification,
      runtime_enforced_by: entry.runtime_enforced_by,
      target_runtime_owner: entry.target_runtime_owner ?? null,
      monitor_state: statusByGate.get(entry.gate_id)?.state ?? null,
      migration_target: entry.migration_target ?? null,
      delete_when: entry.delete_when ?? null,
    }));
  const scaffoldOnly = scaffoldEntries
    .filter((entry) => !hasText(entry.runtime_enforced_by))
    .map((entry) => ({
      gate_id: entry.gate_id,
      classification: entry.classification,
      target_runtime_owner: entry.target_runtime_owner ?? null,
      monitor_state: statusByGate.get(entry.gate_id)?.state ?? null,
      migration_target: entry.migration_target ?? null,
      delete_when: entry.delete_when ?? null,
    }));
  const blocked = statuses
    .filter((status) => status.state === 'blocked_by_regression' || status.state === 'needs_runtime_strengthening')
    .map((status) => ({
      gate_id: status.gate_id,
      state: status.state,
      detail: status.detail,
      source_artifact: status.source_artifact ?? null,
      target_runtime_owner: entries.find((entry) => entry.gate_id === status.gate_id)?.target_runtime_owner ?? null,
      migration_target: status.migration_target ?? null,
    }));
  const retirementCandidates = statuses
    .filter((status) => status.state === 'eligible_for_retirement' || status.state === 'retirement_backlog_created')
    .map((status) => ({
      gate_id: status.gate_id,
      state: status.state,
      retirement_backlog_id: status.retirement_backlog_id ?? null,
      migration_target: status.migration_target ?? null,
    }));
  return {
    scaffold_count: scaffoldEntries.length,
    already_runtime_enforced_count: alreadyRuntimeEnforced.length,
    scaffold_only_count: scaffoldOnly.length,
    blocked_count: blocked.length,
    retirement_candidate_count: retirementCandidates.length,
    already_runtime_enforced: alreadyRuntimeEnforced,
    scaffold_only: scaffoldOnly,
    blocked,
    retirement_candidates: retirementCandidates,
  };
}

function writeMarkdown(path: string, payload: any): void {
  ensureParent(path);
  const lines = [
    '# Test Maturity Registry Report',
    '',
    `- pass: ${payload.ok}`,
    `- generated_at: ${payload.generated_at}`,
    `- registry: ${payload.inputs.registry_path}`,
    `- monitor_history: ${payload.inputs.monitor_history_path}`,
    '',
    '## Summary',
    '',
    `- classified_gates: ${payload.summary.classified_gates}`,
    `- scaffold_count: ${payload.summary.scaffold_count}`,
    `- temporary_monitor_count: ${payload.summary.temporary_monitor_count}`,
    `- runtime_crutch_count: ${payload.summary.runtime_crutch_count}`,
    `- self_enforcement_validator_count: ${payload.summary.self_enforcement_validator_count}`,
    `- release_evidence_gate_count: ${payload.summary.release_evidence_gate_count}`,
    `- architecture_drift_guard_count: ${payload.summary.architecture_drift_guard_count}`,
    `- expired_scaffold_count: ${payload.summary.expired_scaffold_count}`,
    `- runtime_enforcement_ratio: ${payload.summary.runtime_enforcement_ratio}`,
    `- retirement_candidates: ${payload.summary.retirement_candidates}`,
    `- monitors_needing_runtime_strengthening: ${payload.summary.monitors_needing_runtime_strengthening}`,
    `- scaffold_only_count: ${payload.scaffold_audit.scaffold_only_count}`,
    `- already_runtime_enforced_scaffolds: ${payload.scaffold_audit.already_runtime_enforced_count}`,
    '',
    '## Classification Counts',
    '',
    '| Classification | Count |',
    '| --- | ---: |',
    ...Object.entries(payload.classification_counts).map(([key, value]) => `| ${key} | ${value} |`),
    '',
    '## Temporary Monitor Status',
    '',
    '| Gate | State | Runs | Pass Rate | Streak | Detail |',
    '| --- | --- | ---: | ---: | ---: | --- |',
    ...payload.monitor_statuses.map((row: MonitorStatus) => `| ${row.gate_id} | ${row.state} | ${row.window_runs} | ${row.success_rate} | ${row.consecutive_passes} | ${row.detail.replace(/\|/g, '/') } |`),
    '',
    '## Scaffold Audit',
    '',
    `- scaffold_only_count: ${payload.scaffold_audit.scaffold_only_count}`,
    `- already_runtime_enforced_count: ${payload.scaffold_audit.already_runtime_enforced_count}`,
    `- blocked_count: ${payload.scaffold_audit.blocked_count}`,
    `- retirement_candidate_count: ${payload.scaffold_audit.retirement_candidate_count}`,
    '',
    '| Gate | Classification | Runtime Owner | Monitor State | Migration Target |',
    '| --- | --- | --- | --- | --- |',
    ...payload.scaffold_audit.already_runtime_enforced.map((row: any) => `| ${row.gate_id} | ${row.classification} | ${String(row.target_runtime_owner ?? row.runtime_enforced_by ?? '').replace(/\|/g, '/') } | ${row.monitor_state ?? ''} | ${String(row.migration_target ?? '').replace(/\|/g, '/') } |`),
    ...payload.scaffold_audit.scaffold_only.map((row: any) => `| ${row.gate_id} | ${row.classification} | ${String(row.target_runtime_owner ?? '').replace(/\|/g, '/') } | ${row.monitor_state ?? ''} | ${String(row.migration_target ?? '').replace(/\|/g, '/') } |`),
    '',
    '## Runtime Strengthening Targets',
    '',
    '| Target | Scaffolds | Temporary Monitors | Runtime Crutches | Gates |',
    '| --- | ---: | ---: | ---: | --- |',
    ...Object.entries(payload.strengthen_targets).map(([target, value]: [string, any]) => `| ${target} | ${value.scaffolds} | ${value.temporary_monitors} | ${value.runtime_crutches} | ${value.gates.join(', ')} |`),
    '',
    '## Runtime Strengthening Feedback',
    '',
    `- target_count: ${payload.runtime_strengthening_feedback.total_targets}`,
    '',
    '| Runtime Owner | Affected Gates | Failure Signatures | Recommended Action |',
    '| --- | ---: | --- | --- |',
    ...payload.runtime_strengthening_feedback.targets.map((row: any) => `| ${String(row.runtime_owner).replace(/\|/g, '/') } | ${row.affected_gate_count} | ${row.failure_signatures.join(', ').replace(/\|/g, '/') } | ${String(row.recommended_action).replace(/\|/g, '/') } |`),
    '',
    '## Findings',
    '',
    '| Severity | ID | Gate | Detail |',
    '| --- | --- | --- | --- |',
    ...payload.findings.map((row: Finding) => `| ${row.severity} | ${row.id} | ${row.gate_id ?? ''} | ${String(row.detail).replace(/\|/g, '/') } |`),
    '',
  ];
  writeFileSync(path, lines.join('\n'));
}

function main(): void {
  const registryPath = arg('registry', REGISTRY_PATH);
  const suiteManifestPath = arg('suite-manifest', SUITE_MANIFEST_PATH);
  const artifactEnvelopeContractPath = arg('artifact-envelope-contract', ARTIFACT_ENVELOPE_CONTRACT_PATH);
  const temporaryTestContractPath = arg('temporary-test-contract', TEMPORARY_TEST_CONTRACT_PATH);
  const physicalPlacementPolicyPath = arg('physical-placement-policy', PHYSICAL_PLACEMENT_POLICY_PATH);
  const unregisteredTestPolicyPath = arg('unregistered-test-policy', UNREGISTERED_TEST_POLICY_PATH);
  const toolingRegistryPath = arg('tooling-registry', TOOLING_REGISTRY);
  const profilesPath = arg('profiles', PROFILES);
  const manifestPath = arg('manifest', MANIFEST);
  const historyPath = arg('monitor-history', MONITOR_HISTORY);
  const outJson = arg('out-json', OUT_JSON);
  const outMarkdown = arg('out-markdown', OUT_MARKDOWN);
  const retirementJson = arg('retirement-json', RETIREMENT_JSON);
  const retirementMarkdown = arg('retirement-markdown', RETIREMENT_MARKDOWN);
  const strict = flag('strict', true);
  const updateMonitorHistory = flag('update-monitor-history', true);
  const now = new Date(arg('now', new Date().toISOString()).slice(0, 10) + 'T00:00:00Z');

  const registry = readJson(registryPath);
  const suiteManifest = readJson(suiteManifestPath);
  const artifactEnvelopeContract = readJson(artifactEnvelopeContractPath);
  const temporaryTestContract = readJson(temporaryTestContractPath);
  const physicalPlacementPolicy = readJson(physicalPlacementPolicyPath);
  const unregisteredTestPolicy = readJson(unregisteredTestPolicyPath);
  const tooling = readJson(toolingRegistryPath);
  const profiles = readJson(profilesPath);
  const manifest = readJson(manifestPath);
  const pkg = readJson(PACKAGE_JSON);
  const entries: Entry[] = Array.isArray(registry.entries) ? registry.entries : [];
  const findings: Finding[] = [];
  const toolingIds = new Set(gateIds(tooling));
  const classifiedIds = new Set<string>();
  const classificationCounts: Record<string, number> = {};
  const history = normalizedHistory(readJsonIfExists(historyPath, defaultHistory()));
  let expiredScaffoldCount = 0;

  if (!registry.schema_version) addFinding(findings, 'missing_schema_version', 'fail', 'Registry must declare schema_version.');
  if (entries.length === 0) addFinding(findings, 'empty_registry', 'fail', 'Registry must classify at least one gate.');
  if (!registryPath.startsWith('validation/tests/')) {
    addFinding(findings, 'registry_not_validation_owned', 'fail', 'Canonical test lifecycle registry must live under validation/tests.', GATE_ID);
  }

  for (const entry of entries) {
    if (!hasText(entry.gate_id)) {
      addFinding(findings, 'missing_gate_id', 'fail', 'Registry entry is missing gate_id.');
      continue;
    }
    if (classifiedIds.has(entry.gate_id)) addFinding(findings, 'duplicate_gate_id', 'fail', 'Gate is classified more than once.', entry.gate_id);
    classifiedIds.add(entry.gate_id);
    if (!ALLOWED.has(entry.classification)) addFinding(findings, 'invalid_classification', 'fail', `Invalid classification: ${entry.classification}`, entry.gate_id);
    classificationCounts[entry.classification] = (classificationCounts[entry.classification] ?? 0) + 1;
    if (!toolingIds.has(entry.gate_id)) addFinding(findings, 'classified_gate_not_registered', 'warn', 'Classified gate is not present in tooling_gate_registry.json.', entry.gate_id);
    if (!hasText(entry.invariant)) addFinding(findings, 'missing_invariant', 'fail', 'Every classified gate must declare the invariant it protects.', entry.gate_id);
    if (!hasText(entry.strengthen_signal)) addFinding(findings, 'missing_strengthen_signal', 'fail', 'Every classified gate must explain what subsystem weakness it indicates if it remains necessary.', entry.gate_id);

    if (entry.classification === 'self_enforcement_validator') requireFields(entry, ['runtime_enforced_by', 'validation_scope'], findings);
    if (entry.classification === 'release_evidence_gate') requireFields(entry, ['proof_artifact', 'validation_scope'], findings);
    if (entry.classification === 'architecture_drift_guard') requireFields(entry, ['drift_surface', 'validation_scope'], findings);
    if (entry.classification === 'temporary_scaffold') requireFields(entry, ['scaffold_reason', 'expires_after', 'migration_target', 'delete_when'], findings);
    if (entry.classification === 'temporary_monitor') {
      requireFields(entry, ['scaffold_reason', 'migration_target', 'retirement_action', 'delete_when', 'target_runtime_owner'], findings);
      if (!entry.success_criteria || typeof entry.success_criteria !== 'object' || Array.isArray(entry.success_criteria)) {
        addFinding(findings, 'missing_required_field', 'fail', 'temporary_monitor requires success_criteria object', entry.gate_id);
      }
    }
    if (entry.classification === 'runtime_crutch') requireFields(entry, ['weakness_indicator', 'target_runtime_owner', 'migration_target'], findings);
    if (entry.classification === 'one_time_closure_guard') requireFields(entry, ['scaffold_reason', 'expires_after', 'migration_target', 'delete_when', 'target_runtime_owner'], findings);
    validateTemporaryLifecycleEntry(entry, findings);

    if (SCAFFOLD_CLASSES.has(entry.classification) && entry.classification !== 'temporary_monitor') {
      const days = daysUntil(entry.expires_after, now);
      if (days === null) {
        addFinding(findings, 'invalid_expiry', 'fail', 'Scaffold expiry must be YYYY-MM-DD.', entry.gate_id);
      } else if (days < 0) {
        expiredScaffoldCount += 1;
        addFinding(findings, 'expired_scaffold', 'fail', `Scaffold expired ${Math.abs(days)} day(s) ago; delete, migrate, or renew intentionally.`, entry.gate_id);
      } else if (days <= 14) {
        addFinding(findings, 'scaffold_expiring_soon', 'warn', `Scaffold expires in ${days} day(s); plan deletion or migration.`, entry.gate_id);
      }
    }
  }

  const monitorEntries = entries.filter((entry) => entry.classification === 'temporary_monitor');
  const monitorStatuses = monitorEntries.map((entry) => evaluateMonitor(entry, history, tooling, now, updateMonitorHistory));
  for (const status of monitorStatuses) {
    if (status.state === 'needs_runtime_strengthening') {
      addFinding(findings, 'temporary_monitor_needs_runtime_strengthening', 'warn', status.detail, status.gate_id);
    } else if (status.state === 'blocked_by_regression') {
      addFinding(findings, 'temporary_monitor_blocked_by_regression', 'warn', status.detail, status.gate_id);
    } else if (status.state === 'retirement_backlog_created' || status.state === 'eligible_for_retirement') {
      addFinding(findings, 'temporary_monitor_retirement_candidate', 'info', status.detail, status.gate_id);
    }
  }

  const allToolingGateIds = gateIds(tooling);
  const unregisteredPolicySummary = validateUnregisteredTestPolicy(unregisteredTestPolicyPath, unregisteredTestPolicy, allToolingGateIds, classifiedIds, now, findings);
  const allowedUnregisteredIds = new Set<string>(unregisteredPolicySummary.allowed_unregistered_ids ?? []);
  const unclassifiedToolingGates = allToolingGateIds.filter((id) => !classifiedIds.has(id) && !allowedUnregisteredIds.has(id));

  const closureLike = allToolingGateIds.filter((id) => /closure|scaffold|assimilation/i.test(id));
  const unclassifiedClosureLike = closureLike.filter((id) => !classifiedIds.has(id) && !allowedUnregisteredIds.has(id));
  for (const id of unclassifiedClosureLike.slice(0, 50)) {
    addFinding(findings, 'unclassified_closure_like_gate', 'info', 'Closure/scaffold-like gate should be classified before it becomes permanent debt.', id);
  }

  const ownArtifacts = list(tooling?.gates?.[GATE_ID]?.artifact_paths);
  const required = requiredArtifacts(manifest);
  const releaseGovernance = releaseGovernanceArtifacts(manifest);
  const reports = optionalReports(manifest);
  const expectedArtifacts = [outJson, outMarkdown, retirementJson, retirementMarkdown];
  const suiteSummary = validateSuiteManifests(suiteManifestPath, suiteManifest, classifiedIds, findings);
  const artifactEnvelopeSummary = validateArtifactEnvelopeContract(artifactEnvelopeContractPath, artifactEnvelopeContract, findings);
  const temporaryTestContractSummary = validateTemporaryTestContract(temporaryTestContractPath, temporaryTestContract, findings);
  const physicalPlacementSummary = validatePhysicalPlacementPolicy(
    physicalPlacementPolicyPath,
    physicalPlacementPolicy,
    {
      registry: registryPath,
      suite_manifest: suiteManifestPath,
      artifact_envelope_contract: artifactEnvelopeContractPath,
      temporary_test_contract: temporaryTestContractPath,
      physical_placement_policy: physicalPlacementPolicyPath,
    },
    tooling,
    suiteManifest,
    findings,
  );
  const lifecycleArtifactEnvelopes = buildLifecycleArtifactEnvelopes(entries, findings, monitorStatuses);
  const runtimeStrengtheningFeedback = buildRuntimeStrengtheningFeedback(entries, findings, monitorStatuses, lifecycleArtifactEnvelopes);
  if (!packageScript(pkg, GATE_ID).includes('tooling:run')) addFinding(findings, 'package_script_missing', 'fail', `${GATE_ID} package script must invoke tooling:run.`, GATE_ID);
  if (!toolingIds.has(GATE_ID)) addFinding(findings, 'own_gate_missing_from_tooling_registry', 'fail', `${GATE_ID} must be registered in tooling_gate_registry.json.`, GATE_ID);
  if (!expectedArtifacts.every((path) => ownArtifacts.includes(path))) addFinding(findings, 'own_artifacts_missing_from_registry', 'fail', 'Own artifact paths must include maturity and retirement artifacts.', GATE_ID);
  for (const profile of ['fast', 'boundary', 'release']) {
    if (!profileGateIds(profiles, profile).includes(GATE_ID)) addFinding(findings, 'profile_missing_test_maturity_gate', 'fail', `${profile} profile must include ${GATE_ID}.`, GATE_ID);
  }
  if (!required.includes(outJson)) addFinding(findings, 'proof_pack_required_artifact_missing', 'fail', `${outJson} must be required proof-pack evidence.`, GATE_ID);
  if (!required.includes(retirementJson)) addFinding(findings, 'retirement_backlog_required_artifact_missing', 'fail', `${retirementJson} must be required proof-pack evidence.`, GATE_ID);
  if (!releaseGovernance.includes(outJson) || !releaseGovernance.includes(retirementJson)) addFinding(findings, 'release_governance_group_missing', 'fail', 'Maturity and retirement artifacts must be grouped under release_governance.', GATE_ID);
  if (!reports.includes(outMarkdown) || !reports.includes(retirementMarkdown)) addFinding(findings, 'operator_report_missing', 'fail', 'Maturity and retirement reports must be listed as optional operator reports.', GATE_ID);

  const scaffoldCount = entries.filter((entry) => SCAFFOLD_CLASSES.has(entry.classification)).length;
  const monitorCount = monitorEntries.length;
  const runtimeCrutchCount = entries.filter((entry) => entry.classification === 'runtime_crutch').length;
  const validatorCount = entries.filter((entry) => entry.classification === 'self_enforcement_validator').length;
  const releaseGateCount = entries.filter((entry) => entry.classification === 'release_evidence_gate').length;
  const architectureGuardCount = entries.filter((entry) => entry.classification === 'architecture_drift_guard').length;
  const denominator = scaffoldCount + runtimeCrutchCount;
  const runtimeEnforcementRatio = denominator === 0 ? validatorCount : Number((validatorCount / denominator).toFixed(3));
  const failCount = findings.filter((row) => row.severity === 'fail').length;
  const ok = failCount === 0;
  const scaffoldAudit = buildScaffoldAudit(entries, monitorStatuses);
  const retirementPayload = {
    ok: true,
    type: 'test_maturity_retirement_backlog',
    generated_at: new Date().toISOString(),
    monitor_history_path: historyPath,
    summary: {
      temporary_monitor_count: monitorCount,
      retirement_candidates: monitorStatuses.filter((row) => row.state === 'eligible_for_retirement' || row.state === 'retirement_backlog_created').length,
      blocked_by_regression: monitorStatuses.filter((row) => row.state === 'blocked_by_regression').length,
      needs_runtime_strengthening: monitorStatuses.filter((row) => row.state === 'needs_runtime_strengthening').length,
      observing: monitorStatuses.filter((row) => row.state === 'observing' || row.state === 'insufficient_signal').length,
    },
    monitor_statuses: monitorStatuses,
    backlog_items: Object.values(history.retirement_backlog),
    retirement_action_items: monitorStatuses
      .filter((status) => status.state === 'eligible_for_retirement' || status.state === 'retirement_backlog_created')
      .map((status) => retirementActionItem(entries.find((entry) => entry.gate_id === status.gate_id)!, status)),
  };
  const payload = {
    ok,
    type: 'test_maturity_registry_guard',
    generated_at: new Date().toISOString(),
    inputs: { registry_path: registryPath, suite_manifest_path: suiteManifestPath, artifact_envelope_contract_path: artifactEnvelopeContractPath, temporary_test_contract_path: temporaryTestContractPath, physical_placement_policy_path: physicalPlacementPolicyPath, tooling_registry_path: toolingRegistryPath, profiles_path: profilesPath, manifest_path: manifestPath, monitor_history_path: historyPath },
    summary: {
      classified_gates: entries.length,
      tooling_gate_count: toolingIds.size,
      classified_tooling_gate_count: entries.filter((entry) => toolingIds.has(entry.gate_id)).length,
      scaffold_count: scaffoldCount,
      temporary_monitor_count: monitorCount,
      runtime_crutch_count: runtimeCrutchCount,
      self_enforcement_validator_count: validatorCount,
      release_evidence_gate_count: releaseGateCount,
      architecture_drift_guard_count: architectureGuardCount,
      expired_scaffold_count: expiredScaffoldCount,
      unclassified_tooling_gate_count: unclassifiedToolingGates.length,
      unclassified_closure_like_gate_count: unclassifiedClosureLike.length,
      retirement_candidates: retirementPayload.summary.retirement_candidates,
      monitors_blocked_by_regression: retirementPayload.summary.blocked_by_regression,
      monitors_needing_runtime_strengthening: retirementPayload.summary.needs_runtime_strengthening,
      runtime_enforcement_ratio: runtimeEnforcementRatio,
      suite_manifest_count: suiteSummary.suite_count,
      suite_managed_gate_count: suiteSummary.managed_gate_count,
      lifecycle_artifact_envelope_count: lifecycleArtifactEnvelopes.length,
      lifecycle_artifact_envelope_unhealthy_count: lifecycleArtifactEnvelopes.filter((row) => !row.ok).length,
      runtime_strengthening_target_count: runtimeStrengtheningFeedback.total_targets,
      findings: findings.length,
      fail_findings: failCount,
      warn_findings: findings.filter((row) => row.severity === 'warn').length,
      info_findings: findings.filter((row) => row.severity === 'info').length,
    },
    classification_counts: classificationCounts,
    monitor_statuses: monitorStatuses,
    suite_lifecycle: suiteSummary,
    artifact_envelope_contract: artifactEnvelopeSummary,
    temporary_test_contract: temporaryTestContractSummary,
      physical_placement_policy: physicalPlacementSummary,
      unregistered_test_policy: unregisteredPolicySummary,
      lifecycle_artifact_envelopes: lifecycleArtifactEnvelopes,
    runtime_strengthening_feedback: runtimeStrengtheningFeedback,
    scaffold_audit: scaffoldAudit,
    strengthen_targets: classifyStrengthenTargets(entries),
    findings,
    artifact_paths: [outJson, outMarkdown, retirementJson, retirementMarkdown],
  };

  history.updated_at = new Date().toISOString();
  if (updateMonitorHistory) {
    ensureParent(historyPath);
    writeFileSync(historyPath, `${JSON.stringify(history, null, 2)}\n`);
  }
  ensureParent(outJson);
  ensureParent(retirementJson);
  writeFileSync(outJson, `${JSON.stringify(payload, null, 2)}\n`);
  writeFileSync(retirementJson, `${JSON.stringify(retirementPayload, null, 2)}\n`);
  writeMarkdown(outMarkdown, payload);
  writeRetirementBacklog(retirementMarkdown, monitorStatuses, entries);
  if (strict && !ok) process.exitCode = 1;
}

main();
