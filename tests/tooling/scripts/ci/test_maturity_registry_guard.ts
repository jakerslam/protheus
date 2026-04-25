#!/usr/bin/env tsx

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'fs';
import { dirname } from 'path';

const REGISTRY_PATH = 'tests/tooling/config/test_maturity_registry.json';
const TOOLING_REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const PROFILES = 'tests/tooling/config/verify_profiles.json';
const MANIFEST = 'tests/tooling/config/release_proof_pack_manifest.json';
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
    if (payload?.ok === false) return { at: new Date().toISOString(), ok: false, observation: 'fail', source_artifact: path, detail: 'source artifact ok=false' };
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

function retirementText(entry: Entry, status: MonitorStatus): string {
  return [
    `- [ ] \`${status.retirement_backlog_id}\` Retire or merge temporary monitor \`${entry.gate_id}\`.`,
    `  Criteria met: ${status.passes}/${status.window_runs} passes in the observation window, success_rate=${status.success_rate}, consecutive_passes=${status.consecutive_passes}.`,
    `  Migration target: ${entry.migration_target}.`,
    `  Delete when: ${entry.delete_when}.`,
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
    '## Runtime Strengthening Targets',
    '',
    '| Target | Scaffolds | Temporary Monitors | Runtime Crutches | Gates |',
    '| --- | ---: | ---: | ---: | --- |',
    ...Object.entries(payload.strengthen_targets).map(([target, value]: [string, any]) => `| ${target} | ${value.scaffolds} | ${value.temporary_monitors} | ${value.runtime_crutches} | ${value.gates.join(', ')} |`),
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

  const closureLike = gateIds(tooling).filter((id) => /closure|scaffold|assimilation/i.test(id));
  const unclassifiedClosureLike = closureLike.filter((id) => !classifiedIds.has(id));
  for (const id of unclassifiedClosureLike.slice(0, 50)) {
    addFinding(findings, 'unclassified_closure_like_gate', 'info', 'Closure/scaffold-like gate should be classified before it becomes permanent debt.', id);
  }

  const ownArtifacts = list(tooling?.gates?.[GATE_ID]?.artifact_paths);
  const required = requiredArtifacts(manifest);
  const releaseGovernance = releaseGovernanceArtifacts(manifest);
  const reports = optionalReports(manifest);
  const expectedArtifacts = [outJson, outMarkdown, retirementJson, retirementMarkdown];
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
  };
  const payload = {
    ok,
    type: 'test_maturity_registry_guard',
    generated_at: new Date().toISOString(),
    inputs: { registry_path: registryPath, tooling_registry_path: toolingRegistryPath, profiles_path: profilesPath, manifest_path: manifestPath, monitor_history_path: historyPath },
    summary: {
      classified_gates: entries.length,
      tooling_gate_count: toolingIds.size,
      scaffold_count: scaffoldCount,
      temporary_monitor_count: monitorCount,
      runtime_crutch_count: runtimeCrutchCount,
      self_enforcement_validator_count: validatorCount,
      release_evidence_gate_count: releaseGateCount,
      architecture_drift_guard_count: architectureGuardCount,
      expired_scaffold_count: expiredScaffoldCount,
      unclassified_closure_like_gate_count: unclassifiedClosureLike.length,
      retirement_candidates: retirementPayload.summary.retirement_candidates,
      monitors_blocked_by_regression: retirementPayload.summary.blocked_by_regression,
      monitors_needing_runtime_strengthening: retirementPayload.summary.needs_runtime_strengthening,
      runtime_enforcement_ratio: runtimeEnforcementRatio,
      findings: findings.length,
      fail_findings: failCount,
      warn_findings: findings.filter((row) => row.severity === 'warn').length,
      info_findings: findings.filter((row) => row.severity === 'info').length,
    },
    classification_counts: classificationCounts,
    monitor_statuses: monitorStatuses,
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
