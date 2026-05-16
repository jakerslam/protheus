import fs from 'node:fs';
import path from 'node:path';

type Json = Record<string, any>;
type LanePolicy = {
  id: string;
  purpose: string;
  work_class: string;
  capability_domain: string;
  user_visible?: boolean;
  live_proof?: boolean;
  max_age_hours?: number;
  evidence_paths_any?: string[];
  guard_artifact_paths_any?: string[];
  source_guard_paths_any?: string[];
  required_evidence_fields?: { path: string; equals: any }[];
  evidence_ok_equals?: boolean;
  guard_artifact_ok_equals?: boolean;
};

type Policy = {
  report_path?: string;
  guard_result_path?: string;
  markdown_report_path?: string;
  minimum_ready_lanes?: number;
  minimum_user_visible_lanes?: number;
  minimum_live_lanes?: number;
  minimum_distinct_work_classes?: number;
  minimum_distinct_capability_domains?: number;
  default_max_age_hours?: number;
  lanes?: LanePolicy[];
};

const root = process.cwd();
const policyRelPath = 'validation/proof_packs/real_work_workflow_proof_policy.json';
const policyPath = path.join(root, policyRelPath);
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Policy;

function readFlag(argv: string[], key: string): string | null {
  const prefix = `--${key}=`;
  const row = argv.find((arg) => arg.startsWith(prefix));
  if (row) return row.slice(prefix.length);
  const idx = argv.indexOf(`--${key}`);
  if (idx >= 0 && idx + 1 < argv.length) return argv[idx + 1];
  return null;
}

function firstExisting(paths: string[] = []): string | null {
  for (const rel of paths) {
    if (fs.existsSync(path.join(root, rel))) return rel;
  }
  return null;
}

function readJson(rel: string | null): Json | null {
  if (!rel) return null;
  try {
    return JSON.parse(fs.readFileSync(path.join(root, rel), 'utf8')) as Json;
  } catch {
    return null;
  }
}

function valueAtPath(payload: Json | null, dotted: string): any {
  if (!payload) return undefined;
  return dotted.split('.').reduce<any>((acc, key) => (acc && typeof acc === 'object' ? acc[key] : undefined), payload);
}

function generatedAtOf(rel: string | null, payload: Json | null): string | null {
  const generated = typeof payload?.generated_at === 'string' ? payload.generated_at : null;
  if (generated) return generated;
  if (!rel) return null;
  try {
    return fs.statSync(path.join(root, rel)).mtime.toISOString();
  } catch {
    return null;
  }
}

function ageHours(generatedAt: string | null): number | null {
  if (!generatedAt) return null;
  const parsed = Date.parse(generatedAt);
  if (!Number.isFinite(parsed)) return null;
  return Number(((Date.now() - parsed) / 3_600_000).toFixed(3));
}

function payloadOk(payload: Json | null, expectedOk: boolean | null = null): boolean {
  if (!payload) return false;
  if (typeof expectedOk === 'boolean') {
    return payload.ok === expectedOk || payload.pass === expectedOk || payload.summary?.pass === expectedOk || payload.summary?.ok === expectedOk;
  }
  if (payload.ok === true) return true;
  if (payload.pass === true) return true;
  if (payload.summary?.pass === true) return true;
  if (payload.summary?.ok === true) return true;
  return false;
}

function requiredFieldsSatisfied(payload: Json | null, required: LanePolicy['required_evidence_fields'] = []) {
  const failures: string[] = [];
  for (const row of required) {
    const actual = valueAtPath(payload, row.path);
    if (actual !== row.equals) failures.push(`${row.path} expected ${JSON.stringify(row.equals)} got ${JSON.stringify(actual)}`);
  }
  return { ok: failures.length === 0, failures };
}

function markdownEscape(raw: unknown): string {
  return String(raw ?? '').replace(/\\/g, '\\\\').replace(/\|/g, '\\|').replace(/\n/g, ' ');
}

function markdownFor(report: Json): string {
  const lines: string[] = [];
  lines.push('# Real Work Workflow Proof');
  lines.push('');
  lines.push(`- generated_at: ${report.generated_at}`);
  lines.push(`- ok: ${report.ok}`);
  lines.push(`- ready_lane_count: ${report.ready_lane_count}/${report.total_lane_count}`);
  lines.push(`- user_visible_ready_lane_count: ${report.user_visible_ready_lane_count}`);
  lines.push(`- live_ready_lane_count: ${report.live_ready_lane_count}`);
  lines.push(`- distinct_ready_work_class_count: ${report.distinct_ready_work_class_count}`);
  lines.push(`- distinct_ready_capability_domain_count: ${report.distinct_ready_capability_domain_count}`);
  lines.push('');
  lines.push('## Capability Outcomes');
  lines.push('');
  for (const outcome of report.capability_outcomes || []) {
    lines.push(`- ${markdownEscape(outcome.capability_domain)}: ${outcome.ready_lane_count} ready lanes; work_classes=${markdownEscape((outcome.work_classes || []).join(', '))}`);
  }
  lines.push('');
  lines.push('## Operator Journeys');
  lines.push('');
  for (const journey of report.operator_journeys || []) {
    lines.push(`- ${markdownEscape(journey.id)}: ${markdownEscape(journey.outcome)}; lanes=${markdownEscape((journey.lane_ids || []).join(', '))}`);
  }
  lines.push('');
  lines.push('## Lanes');
  lines.push('');
  lines.push('| Lane | Class | Domain | Ready | Evidence | Guard | Next action |');
  lines.push('|---|---|---|---:|---|---|---|');
  for (const lane of report.lanes || []) {
    lines.push(`| ${markdownEscape(lane.id)} | ${markdownEscape(lane.work_class)} | ${markdownEscape(lane.capability_domain)} | ${lane.ready === true ? 'true' : 'false'} | ${markdownEscape(lane.evidence_path || '')} | ${markdownEscape(lane.guard_artifact_path || lane.source_guard_path || '')} | ${markdownEscape(lane.next_action || '')} |`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function compactEvidence(payload: Json | null): Json {
  if (!payload) return {};
  const out: Json = {};
  for (const key of ['type', 'ok', 'pass', 'diagnostic', 'live_proof_complete', 'apply', 'failure_count', 'violation_count']) {
    if (key in payload) out[key] = payload[key];
  }
  if (payload.summary && typeof payload.summary === 'object') {
    out.summary = {
      pass: payload.summary.pass,
      replay_missing_count: payload.summary.replay_missing_count,
      replay_failed_count: payload.summary.replay_failed_count,
      failure_count: payload.summary.failure_count,
    };
  }
  return out;
}

function chainForLane(lane: LanePolicy, evidencePath: string | null, guardArtifactPath: string | null, sourceGuardPath: string | null, ready: boolean) {
  return {
    operator_trigger: `${lane.work_class}:${lane.id}`,
    execution_surface: sourceGuardPath || guardArtifactPath || evidencePath || null,
    evidence_artifact: evidencePath,
    guard_verification: guardArtifactPath,
    user_value: lane.purpose,
    release_confidence: ready ? 'fresh_passing_guarded_evidence' : 'incomplete_or_stale_evidence',
  };
}

function userValueStatement(lane: LanePolicy): string {
  const visibility = lane.user_visible === true ? 'user-visible' : 'system-maintenance';
  return `${visibility} ${lane.capability_domain} proof: ${lane.purpose}`;
}

function laneReport(lane: LanePolicy) {
  const evidencePath = firstExisting(lane.evidence_paths_any);
  const evidencePayload = readJson(evidencePath);
  const guardArtifactPath = firstExisting(lane.guard_artifact_paths_any);
  const guardArtifactPayload = readJson(guardArtifactPath);
  const sourceGuardPath = firstExisting(lane.source_guard_paths_any);
  const expectedEvidenceOk = typeof lane.evidence_ok_equals === 'boolean' ? lane.evidence_ok_equals : null;
  const expectedGuardArtifactOk = typeof lane.guard_artifact_ok_equals === 'boolean' ? lane.guard_artifact_ok_equals : null;
  const evidenceOk = payloadOk(evidencePayload, expectedEvidenceOk);
  const guardArtifactOk = payloadOk(guardArtifactPayload, expectedGuardArtifactOk);
  const evidenceGeneratedAt = generatedAtOf(evidencePath, evidencePayload);
  const guardGeneratedAt = generatedAtOf(guardArtifactPath, guardArtifactPayload);
  const evidenceAge = ageHours(evidenceGeneratedAt);
  const guardAge = ageHours(guardGeneratedAt);
  const maxAge = Number(lane.max_age_hours ?? policy.default_max_age_hours ?? 360);
  const evidenceFresh = evidenceAge !== null && evidenceAge <= maxAge;
  const guardFresh = guardAge !== null && guardAge <= maxAge;
  const required = requiredFieldsSatisfied(evidencePayload, lane.required_evidence_fields);
  const ready = Boolean(
    evidencePath &&
      evidenceOk &&
      evidenceFresh &&
      guardArtifactPath &&
      guardArtifactOk &&
      guardFresh &&
      sourceGuardPath &&
      required.ok,
  );
  const reasons: string[] = [];
  if (!evidencePath) reasons.push('missing_evidence_artifact');
  if (evidencePath && !evidenceOk) reasons.push('evidence_not_passing');
  if (evidencePath && !evidenceFresh) reasons.push('evidence_stale_or_missing_timestamp');
  if (!guardArtifactPath) reasons.push('missing_guard_artifact');
  if (guardArtifactPath && !guardArtifactOk) reasons.push('guard_artifact_not_passing');
  if (guardArtifactPath && !guardFresh) reasons.push('guard_artifact_stale_or_missing_timestamp');
  if (!sourceGuardPath) reasons.push('missing_source_guard');
  for (const failure of required.failures) reasons.push(`required_field:${failure}`);
  return {
    id: lane.id,
    purpose: lane.purpose,
    work_class: lane.work_class,
    capability_domain: lane.capability_domain,
    user_visible: lane.user_visible === true,
    live_proof: lane.live_proof === true,
    ready,
    evidence_path: evidencePath,
    evidence_ok: evidenceOk,
    evidence_generated_at: evidenceGeneratedAt,
    evidence_age_hours: evidenceAge,
    evidence_compact: compactEvidence(evidencePayload),
    expected_evidence_ok: expectedEvidenceOk,
    guard_artifact_path: guardArtifactPath,
    guard_artifact_ok: guardArtifactOk,
    expected_guard_artifact_ok: expectedGuardArtifactOk,
    guard_generated_at: guardGeneratedAt,
    guard_age_hours: guardAge,
    source_guard_path: sourceGuardPath,
    fresh: evidenceFresh && guardFresh,
    end_to_end_chain: chainForLane(lane, evidencePath, guardArtifactPath, sourceGuardPath, ready),
    user_value_statement: userValueStatement(lane),
    required_field_failures: required.failures,
    reasons,
    next_action: ready ? null : `Repair ${lane.id}: ${reasons.join('; ') || 'unknown readiness failure'}.`,
  };
}

function run(argv: string[]) {
  const outJson = readFlag(argv, 'out-json') || policy.report_path || 'core/local/artifacts/real_work_workflow_proof_current.json';
  const outMarkdown = readFlag(argv, 'out-markdown') || policy.markdown_report_path || 'local/workspace/reports/REAL_WORK_WORKFLOW_PROOF_CURRENT.md';
  const lanes = (policy.lanes || []).map(laneReport);
  const ready = lanes.filter((lane) => lane.ready);
  const userVisibleReady = ready.filter((lane) => lane.user_visible);
  const liveReady = ready.filter((lane) => lane.live_proof);
  const minimumReady = Number(policy.minimum_ready_lanes || 1);
  const minimumUserVisible = Number(policy.minimum_user_visible_lanes || 1);
  const minimumLive = Number(policy.minimum_live_lanes || 0);
  const readyWorkClasses = Array.from(new Set(ready.map((lane) => lane.work_class))).sort();
  const readyCapabilityDomains = Array.from(new Set(ready.map((lane) => lane.capability_domain))).sort();
  const minimumDistinctWorkClasses = Number(policy.minimum_distinct_work_classes || 1);
  const minimumDistinctCapabilityDomains = Number(policy.minimum_distinct_capability_domains || 1);
  const ok =
    ready.length >= minimumReady &&
    userVisibleReady.length >= minimumUserVisible &&
    liveReady.length >= minimumLive &&
    readyWorkClasses.length >= minimumDistinctWorkClasses &&
    readyCapabilityDomains.length >= minimumDistinctCapabilityDomains;
  const generatedAt = new Date().toISOString();
  const traceId = `validation:${generatedAt}:real-work-workflow-proof`;
  const capabilityOutcomes = readyCapabilityDomains.map((domain) => {
    const domainLanes = ready.filter((lane) => lane.capability_domain === domain);
    return {
      capability_domain: domain,
      ready_lane_count: domainLanes.length,
      work_classes: Array.from(new Set(domainLanes.map((lane) => lane.work_class))).sort(),
      user_visible_lane_count: domainLanes.filter((lane) => lane.user_visible).length,
      live_lane_count: domainLanes.filter((lane) => lane.live_proof).length,
      evidence_paths: domainLanes.map((lane) => lane.evidence_path).filter(Boolean),
    };
  });
  const operatorJourneys = [
    {
      id: 'recover_windows_install',
      outcome: 'Operator can diagnose a Windows install failure and receive runtime-pending recovery guidance instead of false success.',
      lane_ids: lanes.filter((lane) => ['windows_installer_repair'].includes(lane.id) && lane.ready).map((lane) => lane.id),
      ready: lanes.some((lane) => lane.id === 'windows_installer_repair' && lane.ready),
    },
    {
      id: 'operate_gateway',
      outcome: 'Operator can prove Gateway lifecycle/status behavior with guarded evidence.',
      lane_ids: lanes.filter((lane) => ['gateway_disposable_live_lifecycle', 'gateway_status_diagnosis'].includes(lane.id) && lane.ready).map((lane) => lane.id),
      ready: lanes.some((lane) => lane.id === 'gateway_disposable_live_lifecycle' && lane.ready),
    },
    {
      id: 'navigate_large_repo_safely',
      outcome: 'Agent/operator can use a compressed command entrypoint and safe commit/worktree signals instead of guessing among package scripts.',
      lane_ids: lanes.filter((lane) => ['command_navigation', 'safe_commit_workflow', 'sentinel_worktree_danger'].includes(lane.id) && lane.ready).map((lane) => lane.id),
      ready: lanes.some((lane) => lane.id === 'command_navigation' && lane.ready) && lanes.some((lane) => lane.id === 'sentinel_worktree_danger' && lane.ready),
    },
  ];
  const report = {
    trace_id: traceId,
    span_id: `span:${traceId}`,
    parent_span_id: null,
    source_domain: 'validation',
    type: 'real_work_workflow_proof',
    generated_at: generatedAt,
    policy_path: policyRelPath,
    ok,
    ready_lane_count: ready.length,
    minimum_ready_lanes: minimumReady,
    user_visible_ready_lane_count: userVisibleReady.length,
    minimum_user_visible_lanes: minimumUserVisible,
    live_ready_lane_count: liveReady.length,
    minimum_live_lanes: minimumLive,
    distinct_ready_work_class_count: readyWorkClasses.length,
    minimum_distinct_work_classes: minimumDistinctWorkClasses,
    distinct_ready_capability_domain_count: readyCapabilityDomains.length,
    minimum_distinct_capability_domains: minimumDistinctCapabilityDomains,
    ready_work_classes: readyWorkClasses,
    ready_capability_domains: readyCapabilityDomains,
    total_lane_count: lanes.length,
    capability_outcomes: capabilityOutcomes,
    operator_journeys: operatorJourneys,
    ready_operator_journey_count: operatorJourneys.filter((journey) => journey.ready).length,
    lanes,
    summary: ok
      ? 'Real-work proof has fresh passing evidence across live Gateway operation, workspace/tooling work, installer recovery, security remediation, and operator workflows.'
      : 'Real-work proof is not ready; one or more useful-work lanes are missing fresh passing evidence or guard coverage.',
    artifact_paths: [outJson, outMarkdown],
  };
  const outAbs = path.join(root, outJson);
  const mdAbs = path.join(root, outMarkdown);
  fs.mkdirSync(path.dirname(outAbs), { recursive: true });
  fs.mkdirSync(path.dirname(mdAbs), { recursive: true });
  fs.writeFileSync(outAbs, `${JSON.stringify(report, null, 2)}\n`);
  fs.writeFileSync(mdAbs, markdownFor(report));
  console.log(JSON.stringify(report, null, 2));
  return 0;
}

process.exit(run(process.argv.slice(2)));
