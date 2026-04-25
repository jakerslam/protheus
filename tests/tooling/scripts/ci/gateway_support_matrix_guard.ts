#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';

type JsonRecord = Record<string, any>;
type SupportLevel = 'experimental' | 'candidate' | 'graduated';
type ChecklistStatus = 'pending' | 'in_progress' | 'complete';

const ROOT = process.cwd();
const REQUIRED_GATEWAYS = ['ollama', 'llama_cpp', 'mcp_baseline', 'otlp_exporter', 'durable_memory_local'];
const REQUIRED_SCENARIOS = [
  'process_never_starts',
  'starts_then_hangs',
  'invalid_schema_response',
  'response_too_large',
  'repeated_flapping',
];
const REQUIRED_EVIDENCE = [
  'health_check',
  'chaos_coverage',
  'fail_closed_proof',
  'receipt_completeness',
  'fallback_degradation_declaration',
  'recovery_bounds',
];
const REQUIRED_CHECKLIST = [
  'health_checks',
  'fail_closed_behavior',
  'chaos_scenarios',
  'receipt_completeness',
  'fallback_degradation_declaration',
  'recovery_bounds',
];
const SUPPORT_LEVELS = new Set(['experimental', 'candidate', 'graduated']);
const CHECKLIST_STATUSES = new Set(['pending', 'in_progress', 'complete']);
const DEFAULTS = {
  manifest: 'tests/tooling/config/gateway_manifest.json',
  chaos: 'core/local/artifacts/gateway_runtime_chaos_gate_current.json',
  boundary: 'core/local/artifacts/gateway_boundary_guard_current.json',
  out: 'core/local/artifacts/gateway_support_matrix_current.json',
  quarantineOut: 'core/local/artifacts/gateway_quarantine_recovery_proof_current.json',
  markdown: 'local/workspace/reports/GATEWAY_SUPPORT_MATRIX_CURRENT.md',
};

type Failure = { id: string; detail: string; gateway_id?: string };

function flag(name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  const found = process.argv.slice(2).find((arg) => arg.startsWith(prefix));
  return found ? found.slice(prefix.length) : fallback;
}

function strictEnabled(): boolean {
  const raw = flag('strict', '0').toLowerCase();
  return raw === '1' || raw === 'true' || raw === 'yes';
}

function clean(raw: unknown, max = 240): string {
  return String(raw ?? '').replace(/[\u0000-\u001f\u007f]/g, '').trim().slice(0, max);
}

function relPath(raw: string): string {
  return clean(raw, 500).replace(/^\.\//, '');
}

function readJson(rel: string): JsonRecord {
  const filePath = path.resolve(ROOT, relPath(rel));
  return JSON.parse(fs.readFileSync(filePath, 'utf8')) as JsonRecord;
}

function writeJson(rel: string, payload: JsonRecord): void {
  const filePath = path.resolve(ROOT, relPath(rel));
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

function writeText(rel: string, payload: string): void {
  const filePath = path.resolve(ROOT, relPath(rel));
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, payload.endsWith('\n') ? payload : `${payload}\n`);
}

function existsRel(raw: unknown): boolean {
  const target = relPath(clean(raw, 500));
  return target.length > 0 && fs.existsSync(path.resolve(ROOT, target));
}

function rowsById(rows: unknown): Map<string, JsonRecord> {
  const result = new Map<string, JsonRecord>();
  if (!Array.isArray(rows)) return result;
  for (const row of rows) {
    const id = clean((row as JsonRecord)?.id, 80);
    if (id && !result.has(id)) result.set(id, row as JsonRecord);
  }
  return result;
}

function supportContract(manifest: JsonRecord, level: SupportLevel): ChecklistStatus[] {
  const row = manifest.support_level_contract?.[level];
  const rawStatuses = Array.isArray(row?.allowed_checklist_statuses) ? row.allowed_checklist_statuses : [];
  const normalized = rawStatuses.map((status: unknown) => clean(status, 40)).filter(Boolean) as ChecklistStatus[];
  if (normalized.length > 0) return normalized;
  if (level === 'graduated') return ['complete'];
  if (level === 'candidate') return ['in_progress', 'complete'];
  return ['pending', 'in_progress', 'complete'];
}

function evidencePathOk(row: JsonRecord): boolean {
  return existsRel(row.artifact) || existsRel(row.doc) || existsRel(row.path);
}

function requiredScenariosOk(row: JsonRecord): boolean {
  const found = new Set((Array.isArray(row.required_scenarios) ? row.required_scenarios : []).map((value) => clean(value, 80)));
  return REQUIRED_SCENARIOS.every((scenario) => found.has(scenario));
}

function chaosScenarioOk(chaos: JsonRecord, gatewayId: string, scenario: string): boolean {
  const rows = Array.isArray(chaos.chaos_results) ? chaos.chaos_results : [];
  return rows.some((row: JsonRecord) => clean(row.adapter, 80) === gatewayId && clean(row.scenario, 80) === scenario && row.ok === true);
}

function chaosTransitionOk(chaos: JsonRecord, gatewayId: string): boolean {
  const rows = Array.isArray(chaos.chaos_transition_results) ? chaos.chaos_transition_results : [];
  return rows.some((row: JsonRecord) => {
    const adapter = clean(row.adapter, 80);
    const scenario = clean(row.scenario, 80);
    return adapter === gatewayId && (!scenario || scenario === 'repeated_flapping') && (row.transition_ok === true || row.ok === true);
  });
}

function boundaryQuarantineOk(boundary: JsonRecord): boolean {
  const checks = Array.isArray(boundary.checks) ? boundary.checks : [];
  const checkMap = new Map(checks.map((row: JsonRecord) => [clean(row.id, 120), row.ok === true]));
  return (
    boundary.ok === true &&
    Number(boundary.summary?.quarantine_event_receipt_count || 0) > 0 &&
    Number(boundary.summary?.route_around_count || 0) > 0 &&
    checkMap.get('gateway_repeated_failure_quarantine_contract') === true &&
    checkMap.get('gateway_quarantine_recovery_receipt_contract') === true
  );
}

function renderMarkdown(matrix: JsonRecord, quarantine: JsonRecord): string {
  const lines: string[] = [];
  lines.push('# Gateway Support Matrix');
  lines.push('');
  lines.push(`- generated_at: ${matrix.generated_at}`);
  lines.push(`- pass: ${matrix.ok}`);
  lines.push(`- gateways: ${matrix.summary?.gateway_count ?? 0}`);
  lines.push(`- failure_count: ${matrix.summary?.failure_count ?? 0}`);
  lines.push('');
  lines.push('## Gateway Rows');
  lines.push('| gateway | support_level | checklist_ready | evidence_ready | quarantine_ready | owner |');
  lines.push('| --- | --- | --- | --- | --- | --- |');
  for (const row of matrix.gateway_support_matrix || []) {
    lines.push(`| ${row.gateway_id} | ${row.support_level} | ${row.checklist_ready} | ${row.evidence_ready} | ${row.quarantine_recovery_ready} | ${row.owner} |`);
  }
  lines.push('');
  lines.push('## Quarantine Recovery Proof');
  lines.push(`- pass: ${quarantine.ok}`);
  lines.push(`- boundary_contract_ok: ${quarantine.boundary_contract_ok}`);
  lines.push(`- proof_rows: ${quarantine.summary?.proof_row_count ?? 0}`);
  if ((matrix.failures || []).length > 0 || (quarantine.failures || []).length > 0) {
    lines.push('');
    lines.push('## Failures');
    for (const failure of [...(matrix.failures || []), ...(quarantine.failures || [])]) {
      lines.push(`- ${failure.id}${failure.gateway_id ? `:${failure.gateway_id}` : ''}: ${failure.detail}`);
    }
  }
  return lines.join('\n');
}

function main(): void {
  const strict = strictEnabled();
  const manifestPath = flag('manifest', DEFAULTS.manifest);
  const chaosPath = flag('chaos', DEFAULTS.chaos);
  const boundaryPath = flag('boundary', DEFAULTS.boundary);
  const outPath = flag('out', DEFAULTS.out);
  const quarantineOutPath = flag('quarantine-out', DEFAULTS.quarantineOut);
  const markdownPath = flag('out-markdown', DEFAULTS.markdown);
  const generatedAt = new Date().toISOString();

  const manifest = readJson(manifestPath);
  const chaos = readJson(chaosPath);
  const boundary = readJson(boundaryPath);
  const adapterRows = rowsById(manifest.adapters);
  const gatewayRows = rowsById(manifest.gateways);
  const failures: Failure[] = [];
  const supportMatrix: JsonRecord[] = [];
  const quarantineRows: JsonRecord[] = [];
  const expected = Array.isArray(manifest.production_gateway_targets)
    ? manifest.production_gateway_targets.map((row: unknown) => clean(row, 80)).filter(Boolean)
    : REQUIRED_GATEWAYS;

  for (const gatewayId of REQUIRED_GATEWAYS) {
    if (!expected.includes(gatewayId)) {
      failures.push({ id: 'gateway_missing_from_target_set', gateway_id: gatewayId, detail: 'production gateway target is not declared' });
    }
    const adapter = adapterRows.get(gatewayId);
    const gateway = gatewayRows.get(gatewayId);
    if (!adapter || !gateway) {
      failures.push({ id: 'gateway_matrix_row_missing', gateway_id: gatewayId, detail: 'gateway must be present in both adapters[] and gateways[] manifest sections' });
      continue;
    }
    const supportLevel = clean(gateway.support_level || adapter.support_level, 40) as SupportLevel;
    const owner = clean(gateway.owner || adapter.owner, 120);
    if (!SUPPORT_LEVELS.has(supportLevel)) {
      failures.push({ id: 'gateway_invalid_support_level', gateway_id: gatewayId, detail: `invalid support level: ${supportLevel}` });
    }
    if (!owner) {
      failures.push({ id: 'gateway_owner_missing', gateway_id: gatewayId, detail: 'gateway support row must declare an owner' });
    }
    const allowed = new Set(supportContract(manifest, SUPPORT_LEVELS.has(supportLevel) ? supportLevel : 'experimental'));
    const checklist = adapter.checklist || {};
    const checklistFailures = REQUIRED_CHECKLIST.filter((key) => {
      const status = clean(checklist[key], 40) as ChecklistStatus;
      return !CHECKLIST_STATUSES.has(status) || !allowed.has(status);
    });
    if (checklistFailures.length > 0) {
      failures.push({ id: 'gateway_checklist_not_ready_for_support_level', gateway_id: gatewayId, detail: checklistFailures.join(',') });
    }
    const evidenceFailures = REQUIRED_EVIDENCE.filter((key) => {
      const evidence = gateway[key] || {};
      const status = clean(evidence.status, 40) as ChecklistStatus;
      if (!CHECKLIST_STATUSES.has(status) || !allowed.has(status)) return true;
      if (!evidencePathOk(evidence)) return true;
      if (key === 'chaos_coverage' && !requiredScenariosOk(evidence)) return true;
      return false;
    });
    if (evidenceFailures.length > 0) {
      failures.push({ id: 'gateway_required_evidence_missing_or_invalid', gateway_id: gatewayId, detail: evidenceFailures.join(',') });
    }
    const runtimeRepeatedFlappingOk = chaosScenarioOk(chaos, gatewayId, 'repeated_flapping');
    const runtimeTransitionOk = chaosTransitionOk(chaos, gatewayId);
    const declaredRepeatedFlappingRequired = requiredScenariosOk(gateway.chaos_coverage || {});
    const graduationRuntimeRequired = supportLevel === 'graduated';
    const quarantineRecoveryReady =
      boundaryQuarantineOk(boundary) &&
      declaredRepeatedFlappingRequired &&
      (!graduationRuntimeRequired || (runtimeRepeatedFlappingOk && runtimeTransitionOk));
    if (!quarantineRecoveryReady) {
      failures.push({
        id: 'gateway_quarantine_recovery_not_proven',
        gateway_id: gatewayId,
        detail: `declared_repeated_flapping=${declaredRepeatedFlappingRequired}; runtime_repeated_flapping=${runtimeRepeatedFlappingOk}; runtime_transition=${runtimeTransitionOk}; boundary=${boundaryQuarantineOk(boundary)}; graduated=${graduationRuntimeRequired}`,
      });
    }
    supportMatrix.push({
      gateway_id: gatewayId,
      support_level: supportLevel,
      readiness_track: clean(gateway.readiness_track || adapter.readiness_track, 100),
      owner,
      blocker: clean(gateway.blocker || adapter.blocker, 160),
      checklist_ready: checklistFailures.length === 0,
      evidence_ready: evidenceFailures.length === 0,
      quarantine_recovery_ready: quarantineRecoveryReady,
      required_evidence: REQUIRED_EVIDENCE,
      required_scenarios: REQUIRED_SCENARIOS,
    });
    quarantineRows.push({
      gateway_id: gatewayId,
      declared_repeated_flapping_required: declaredRepeatedFlappingRequired,
      runtime_repeated_flapping_fail_closed: runtimeRepeatedFlappingOk,
      runtime_quarantine_transition_recovery: runtimeTransitionOk,
      graduation_runtime_required: graduationRuntimeRequired,
      boundary_contract_ok: boundaryQuarantineOk(boundary),
      support_level: supportLevel,
      proof_artifacts: [manifestPath, chaosPath, boundaryPath],
    });
  }

  // SRS: V12-SYS-HL-031
  // SRS: V12-SYS-HL-032
  // SRS: V12-SYS-HL-033
  const matrixPayload = {
    ok: failures.length === 0,
    type: 'gateway_support_matrix_v1',
    generated_at: generatedAt,
    manifest_path: manifestPath,
    chaos_artifact_path: chaosPath,
    boundary_artifact_path: boundaryPath,
    summary: {
      gateway_count: supportMatrix.length,
      support_levels: Object.fromEntries([...SUPPORT_LEVELS].map((level) => [level, supportMatrix.filter((row) => row.support_level === level).length])),
      checklist_ready_count: supportMatrix.filter((row) => row.checklist_ready === true).length,
      evidence_ready_count: supportMatrix.filter((row) => row.evidence_ready === true).length,
      quarantine_recovery_ready_count: supportMatrix.filter((row) => row.quarantine_recovery_ready === true).length,
      failure_count: failures.length,
    },
    gateway_support_matrix: supportMatrix,
    failures,
  };
  const quarantinePayload = {
    ok: failures.filter((failure) => failure.id === 'gateway_quarantine_recovery_not_proven').length === 0,
    type: 'gateway_quarantine_recovery_proof_v1',
    generated_at: generatedAt,
    chaos_artifact_path: chaosPath,
    boundary_artifact_path: boundaryPath,
    boundary_contract_ok: boundaryQuarantineOk(boundary),
    summary: {
      proof_row_count: quarantineRows.length,
      declared_repeated_flapping_count: quarantineRows.filter((row) => row.declared_repeated_flapping_required === true).length,
      runtime_repeated_flapping_passed_count: quarantineRows.filter((row) => row.runtime_repeated_flapping_fail_closed === true).length,
      runtime_transition_recovery_passed_count: quarantineRows.filter((row) => row.runtime_quarantine_transition_recovery === true).length,
      graduation_runtime_required_count: quarantineRows.filter((row) => row.graduation_runtime_required === true).length,
      boundary_contract_passed_count: quarantineRows.filter((row) => row.boundary_contract_ok === true).length,
      failure_count: failures.filter((failure) => failure.id === 'gateway_quarantine_recovery_not_proven').length,
    },
    quarantine_recovery_rows: quarantineRows,
    failures: failures.filter((failure) => failure.id === 'gateway_quarantine_recovery_not_proven'),
  };
  writeJson(outPath, matrixPayload);
  writeJson(quarantineOutPath, quarantinePayload);
  writeText(markdownPath, renderMarkdown(matrixPayload, quarantinePayload));
  console.log(JSON.stringify({ ok: matrixPayload.ok && quarantinePayload.ok, type: 'gateway_support_matrix_guard', out: outPath, quarantine_out: quarantineOutPath, failures: failures.length }));
  if (strict && (!matrixPayload.ok || !quarantinePayload.ok)) process.exitCode = 1;
}

main();
