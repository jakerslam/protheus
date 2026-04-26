#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const STATE_DIR = path.join(ROOT, 'local', 'state', 'ops', 'workspace_tooling_context_soak');
const STATE_LATEST_PATH = path.join(STATE_DIR, 'latest.json');
const ARTIFACT_CONTEXT_LATEST_PATH = path.join(
  ARTIFACT_DIR,
  'workspace_tooling_context_soak_report_latest.json',
);
const ARTIFACT_ALIAS_LATEST_PATH = path.join(ARTIFACT_DIR, 'workspace_tooling_soak_report_latest.json');
const CORE_ARTIFACT_PATH = path.join(
  ROOT,
  'core',
  'local',
  'artifacts',
  'workspace_tooling_context_soak_current.json',
);
const CORE_ARTIFACT_CONTEXT_LATEST_PATH = path.join(
  ROOT,
  'core',
  'local',
  'artifacts',
  'workspace_tooling_context_soak_report_latest.json',
);
const CORE_ARTIFACT_ALIAS_PATH = path.join(
  ROOT,
  'core',
  'local',
  'artifacts',
  'workspace_tooling_soak_current.json',
);
const MARKDOWN_PATH = path.join(
  ROOT,
  'local',
  'workspace',
  'reports',
  'WORKSPACE_TOOLING_CONTEXT_SOAK_CURRENT.md',
);
const FIXTURE_PATH = path.join(
  ROOT,
  'tests',
  'tooling',
  'fixtures',
  'workspace_tooling_context_replay_matrix.json',
);
const PATH_TARGETING_FIXTURE_PATH = path.join(
  ROOT,
  'tests',
  'tooling',
  'fixtures',
  'workspace_path_targeting_replay_matrix.json',
);
const ARTIFACT_RELIABILITY_LATEST_PATH = path.join(
  ARTIFACT_DIR,
  'workspace_tooling_reliability_latest.json',
);
const CORE_RELIABILITY_PATH = path.join(
  ROOT,
  'core',
  'local',
  'artifacts',
  'workspace_tooling_reliability_current.json',
);
const STATE_RELIABILITY_PATH = path.join(STATE_DIR, 'reliability_latest.json');
const RELIABILITY_STRICT = process.env.INFRING_WORKSPACE_TOOLING_RELIABILITY_STRICT !== '0';
const TIMEOUT_MS = Math.max(
  30_000,
  Number.parseInt(process.env.INFRING_WORKSPACE_TOOLING_SOAK_TIMEOUT_MS || '900000', 10) || 900_000,
);

type SoakLane = 'routing' | 'hints' | 'synthesis' | 'replay';

type ReplayScenario =
  | 'file_read'
  | 'file_search'
  | 'repo_path_targeting'
  | 'mixed_workspace_tool_routing';

type PathTargetingOperation = 'file_read' | 'file_search' | 'repo_path_targeting';
type PathTargetingKind = 'relative' | 'absolute' | 'windows_style';

const EXPECTED_REQUIRED_REPLAY_SCENARIOS: ReplayScenario[] = [
  'file_read',
  'file_search',
  'repo_path_targeting',
  'mixed_workspace_tool_routing',
];
const EXPECTED_REPLAY_CASES: Record<ReplayScenario, { id: string; test: string }> = {
  file_read: {
    id: 'replay_file_read_contract',
    test: 'workflow_decision_tree_v2_classifies_file_edits_as_task_route',
  },
  file_search: {
    id: 'replay_file_search_contract',
    test: 'compare_workflow_hint_clusters_workspace_and_web_tools',
  },
  repo_path_targeting: {
    id: 'replay_repo_path_targeting_contract',
    test: 'natural_web_intent_does_not_force_plain_workspace_peer_compare_into_web',
  },
  mixed_workspace_tool_routing: {
    id: 'replay_mixed_workspace_tool_routing_contract',
    test: 'compare_workflow_harness_decomposes_local_and_web_evidence_before_final_synthesis',
  },
};
const EXPECTED_PATH_TARGETING_OPERATIONS: PathTargetingOperation[] = [
  'file_read',
  'file_search',
  'repo_path_targeting',
];
const EXPECTED_PATH_TARGETING_KINDS: PathTargetingKind[] = ['relative', 'absolute', 'windows_style'];

const EXPECTED_SOAK_LANES: SoakLane[] = ['routing', 'hints', 'synthesis', 'replay'];

type SoakCase = {
  id: string;
  lane: SoakLane;
  scenario: ReplayScenario;
  test: string;
};

type SoakCaseResult = {
  id: string;
  lane: SoakLane;
  scenario: ReplayScenario;
  test: string;
  status: number;
  ok: boolean;
  duration_ms: number;
  timed_out: boolean;
  stdout_tail: string;
  stderr_tail: string;
};

type SoakReport = {
  type: 'workspace_tooling_context_soak_report';
  schema_version: 2;
  started_at: string;
  finished_at: string;
  ok: boolean;
  command: string;
  status: number;
  duration_ms: number;
  taxonomy: Record<string, unknown>;
  taxonomy_contract: Record<string, unknown>;
  lane_pack: Record<string, unknown>;
  replay_pack: Record<string, unknown>;
  reliability_pack: Record<string, unknown>;
  tests: SoakCaseResult[];
  stdout_tail: string;
  stderr_tail: string;
};

function nowIso(): string {
  return new Date().toISOString();
}

function tsSlug(iso: string): string {
  return iso.replaceAll(':', '-').replaceAll('.', '-');
}

function cleanText(raw: unknown, maxLen = 3200): string {
  return String(raw ?? '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function isWindowsStylePath(value: string): boolean {
  const normalized = cleanText(value, 400);
  return /^[a-zA-Z]:\\/.test(normalized) || normalized.startsWith('\\\\');
}

function isAbsoluteUnixPath(value: string): boolean {
  return cleanText(value, 400).startsWith('/');
}

function writeJson(pathname: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function readFixture(): { cases: SoakCase[]; requiredReplayScenarios: ReplayScenario[]; error: string } {
  let raw: any = null;
  try {
    raw = JSON.parse(fs.readFileSync(FIXTURE_PATH, 'utf8'));
  } catch (error) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_unavailable:${cleanText(error instanceof Error ? error.message : String(error), 240)}`,
    };
  }
  const schemaId = cleanText(raw?.schema_id || '', 120);
  const schemaVersion = Number(raw?.schema_version || 0);
  if (schemaId !== 'workspace_tooling_context_replay_matrix') {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_schema_id_invalid:${schemaId || 'missing'}`,
    };
  }
  if (schemaVersion !== 1) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_schema_version_invalid:${Number.isFinite(schemaVersion) ? schemaVersion : 'missing'}`,
    };
  }
  const rows = Array.isArray(raw?.cases) ? raw.cases : [];
  const cases: SoakCase[] = rows
    .map((row: any) => ({
      id: cleanText(row?.id || '', 120),
      lane: cleanText(row?.lane || '', 40) as SoakLane,
      scenario: cleanText(row?.scenario || '', 80) as ReplayScenario,
      test: cleanText(row?.test || '', 200),
    }))
    .filter((row) => row.id && row.test);
  const laneSet = new Set(['routing', 'hints', 'synthesis', 'replay']);
  const scenarioSet = new Set([
    'file_read',
    'file_search',
    'repo_path_targeting',
    'mixed_workspace_tool_routing',
  ]);
  const laneErrors = cases.filter((row) => !laneSet.has(row.lane));
  const scenarioErrors = cases.filter((row) => !scenarioSet.has(row.scenario));
  const requiredReplayRaw = Array.isArray(raw?.required_replay_scenarios)
    ? raw.required_replay_scenarios.map((value: any) => cleanText(value || '', 80))
    : [];
  const requiredReplayScenarios = requiredReplayRaw
    .filter((value): value is ReplayScenario => scenarioSet.has(value as ReplayScenario))
    .map((value) => value as ReplayScenario);
  const requiredReplayInvalid = requiredReplayRaw.filter(
    (value) => !scenarioSet.has(value as ReplayScenario),
  );
  const duplicateCaseIds = cases
    .map((row) => row.id)
    .filter((id, index, arr) => arr.indexOf(id) !== index);
  const duplicateRequiredReplayScenarios = requiredReplayScenarios.filter(
    (value, index, arr) => arr.indexOf(value) !== index,
  );
  const requiredReplayMissingExpected = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter(
    (scenario) => !requiredReplayScenarios.includes(scenario),
  );
  const requiredReplayUnexpected = requiredReplayScenarios.filter(
    (scenario) => !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(scenario),
  );
  const missingLaneCoverage = EXPECTED_SOAK_LANES.filter(
    (lane) => !cases.some((row) => row.lane === lane),
  );
  const replayRows = cases.filter((row) => row.lane === 'replay');
  const replayRequiredMissing = requiredReplayScenarios.filter(
    (scenario) => !replayRows.some((row) => row.scenario === scenario),
  );
  const replayScenarioUnexpected = replayRows
    .map((row) => row.scenario)
    .filter((scenario) => !requiredReplayScenarios.includes(scenario));
  const replayScenarioOrder = replayRows.map((row) => row.scenario);
  const replayExpectedScenarioOrder = [...EXPECTED_REQUIRED_REPLAY_SCENARIOS];
  const replayCaseOrder = replayRows.map((row) => row.id);
  const replayExpectedCaseOrder = replayExpectedScenarioOrder.map(
    (scenario) => EXPECTED_REPLAY_CASES[scenario].id,
  );
  if (cases.length === 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: 'fixture_cases_missing',
    };
  }
  if (laneErrors.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_lane_invalid:${laneErrors.map((row) => row.id).join(',')}`,
    };
  }
  if (scenarioErrors.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_scenario_invalid:${scenarioErrors.map((row) => row.id).join(',')}`,
    };
  }
  if (duplicateCaseIds.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_case_ids_duplicate:${Array.from(new Set(duplicateCaseIds)).join(',')}`,
    };
  }
  if (requiredReplayInvalid.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_required_replay_scenarios_invalid:${Array.from(new Set(requiredReplayInvalid)).join(',')}`,
    };
  }
  if (duplicateRequiredReplayScenarios.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_required_replay_scenarios_duplicate:${Array.from(new Set(duplicateRequiredReplayScenarios)).join(',')}`,
    };
  }
  if (requiredReplayMissingExpected.length > 0 || requiredReplayUnexpected.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_required_replay_scenarios_noncanonical:missing=${requiredReplayMissingExpected.join(',') || 'none'};unexpected=${requiredReplayUnexpected.join(',') || 'none'}`,
    };
  }
  if (missingLaneCoverage.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_lane_coverage_missing:${missingLaneCoverage.join(',')}`,
    };
  }
  if (requiredReplayScenarios.length === 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: 'fixture_required_replay_scenarios_missing',
    };
  }
  if (replayRows.length === 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: 'fixture_replay_lane_missing',
    };
  }
  if (replayRequiredMissing.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_replay_lane_required_scenario_missing:${replayRequiredMissing.join(',')}`,
    };
  }
  if (replayScenarioUnexpected.length > 0) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_replay_lane_scenario_unexpected:${Array.from(new Set(replayScenarioUnexpected)).join(',')}`,
    };
  }
  if (replayScenarioOrder.join('|') !== replayExpectedScenarioOrder.join('|')) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_replay_lane_scenario_order_noncanonical:expected=${replayExpectedScenarioOrder.join(',')};actual=${replayScenarioOrder.join(',')}`,
    };
  }
  if (replayCaseOrder.join('|') !== replayExpectedCaseOrder.join('|')) {
    return {
      cases: [],
      requiredReplayScenarios: [],
      error: `fixture_replay_lane_case_order_noncanonical:expected=${replayExpectedCaseOrder.join(',')};actual=${replayCaseOrder.join(',')}`,
    };
  }
  for (const scenario of replayExpectedScenarioOrder) {
    const scenarioRows = replayRows.filter((row) => row.scenario === scenario);
    if (scenarioRows.length !== 1) {
      return {
        cases: [],
        requiredReplayScenarios: [],
        error: `fixture_replay_lane_case_cardinality_invalid:${scenario}:${scenarioRows.length}`,
      };
    }
    const expected = EXPECTED_REPLAY_CASES[scenario];
    const row = scenarioRows[0];
    if (row.id !== expected.id) {
      return {
        cases: [],
        requiredReplayScenarios: [],
        error: `fixture_replay_lane_case_id_noncanonical:${scenario}:expected=${expected.id};actual=${row.id}`,
      };
    }
    if (row.test !== expected.test) {
      return {
        cases: [],
        requiredReplayScenarios: [],
        error: `fixture_replay_lane_case_test_noncanonical:${scenario}:expected=${expected.test};actual=${row.test}`,
      };
    }
  }
  return {
    cases,
    requiredReplayScenarios,
    error: '',
  };
}

function readPathTargetingFixture(): {
  cases: Array<{
    id: string;
    operation: PathTargetingOperation;
    path_kind: PathTargetingKind;
    workspace_path: string;
    test: string;
  }>;
  coverage: Record<string, boolean>;
  error: string;
} {
  let raw: any = null;
  try {
    raw = JSON.parse(fs.readFileSync(PATH_TARGETING_FIXTURE_PATH, 'utf8'));
  } catch (error) {
    return {
      cases: [],
      coverage: {},
      error: `path_fixture_unavailable:${cleanText(error instanceof Error ? error.message : String(error), 240)}`,
    };
  }
  const schemaId = cleanText(raw?.schema_id || '', 120);
  const schemaVersion = Number(raw?.schema_version || 0);
  if (schemaId !== 'workspace_path_targeting_replay_matrix') {
    return {
      cases: [],
      coverage: {},
      error: `path_fixture_schema_id_invalid:${schemaId || 'missing'}`,
    };
  }
  if (schemaVersion !== 1) {
    return {
      cases: [],
      coverage: {},
      error: `path_fixture_schema_version_invalid:${Number.isFinite(schemaVersion) ? schemaVersion : 'missing'}`,
    };
  }
  const rows = Array.isArray(raw?.cases) ? raw.cases : [];
  const operationSet = new Set<PathTargetingOperation>(EXPECTED_PATH_TARGETING_OPERATIONS);
  const kindSet = new Set<PathTargetingKind>(EXPECTED_PATH_TARGETING_KINDS);
  const cases = rows
    .map((row: any) => ({
      id: cleanText(row?.id || '', 120),
      operation: cleanText(row?.operation || '', 80) as PathTargetingOperation,
      path_kind: cleanText(row?.path_kind || '', 80) as PathTargetingKind,
      workspace_path: cleanText(row?.workspace_path || '', 400),
      test: cleanText(row?.test || '', 200),
    }))
    .filter((row) => row.id && row.workspace_path && row.test);
  if (cases.length === 0) {
    return {
      cases: [],
      coverage: {},
      error: 'path_fixture_cases_missing',
    };
  }
  const invalidOperationRows = cases.filter((row) => !operationSet.has(row.operation));
  if (invalidOperationRows.length > 0) {
    return {
      cases: [],
      coverage: {},
      error: `path_fixture_operation_invalid:${invalidOperationRows.map((row) => row.id).join(',')}`,
    };
  }
  const invalidKindRows = cases.filter((row) => !kindSet.has(row.path_kind));
  if (invalidKindRows.length > 0) {
    return {
      cases: [],
      coverage: {},
      error: `path_fixture_kind_invalid:${invalidKindRows.map((row) => row.id).join(',')}`,
    };
  }
  const duplicateIds = cases
    .map((row) => row.id)
    .filter((id, index, arr) => arr.indexOf(id) !== index);
  if (duplicateIds.length > 0) {
    return {
      cases: [],
      coverage: {},
      error: `path_fixture_case_ids_duplicate:${Array.from(new Set(duplicateIds)).join(',')}`,
    };
  }
  const invalidPathFormRows = cases.filter((row) => {
    if (row.path_kind === 'relative') {
      return (
        row.workspace_path.includes('\\') ||
        isAbsoluteUnixPath(row.workspace_path) ||
        isWindowsStylePath(row.workspace_path)
      );
    }
    if (row.path_kind === 'absolute') {
      return !isAbsoluteUnixPath(row.workspace_path);
    }
    return !isWindowsStylePath(row.workspace_path);
  });
  if (invalidPathFormRows.length > 0) {
    return {
      cases: [],
      coverage: {},
      error: `path_fixture_path_form_invalid:${invalidPathFormRows.map((row) => row.id).join(',')}`,
    };
  }
  const coverage: Record<string, boolean> = {};
  for (const operation of EXPECTED_PATH_TARGETING_OPERATIONS) {
    for (const kind of EXPECTED_PATH_TARGETING_KINDS) {
      const key = `${operation}:${kind}`;
      coverage[key] = cases.some((row) => row.operation === operation && row.path_kind === kind);
    }
  }
  const missing = Object.entries(coverage)
    .filter(([, ok]) => !ok)
    .map(([key]) => key);
  if (missing.length > 0) {
    return {
      cases: [],
      coverage: {},
      error: `path_fixture_coverage_missing:${missing.join(',')}`,
    };
  }
  return {
    cases,
    coverage,
    error: '',
  };
}

function runCargoTestWithTimeoutKill(testName: string): SoakCaseResult {
  const started = Date.now();
  const commandArgs = [
    'test',
    '-p',
    'infring-ops-core',
    '--lib',
    testName,
    '--',
    '--nocapture',
  ];
  const run = spawnSync('cargo', commandArgs, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: TIMEOUT_MS,
    killSignal: 'SIGKILL',
  });
  const timeoutMessage = String(run.error?.message || '').toLowerCase();
  const timedOut =
    !!run.error && (timeoutMessage.includes('timed out') || timeoutMessage.includes('etimedout'));
  if (timedOut) {
    const pattern = `cargo test -p infring-ops-core --lib ${testName}`;
    spawnSync('pkill', ['-TERM', '-f', pattern], { cwd: ROOT, stdio: 'ignore' });
    spawnSync('pkill', ['-KILL', '-f', pattern], { cwd: ROOT, stdio: 'ignore' });
  }
  const status = timedOut ? 124 : Number.isFinite(run.status) ? Number(run.status) : 1;
  const stdoutRaw = String(run.stdout || '');
  const stderrRaw = String(run.stderr || '');
  return {
    id: '',
    lane: 'routing',
    scenario: 'file_read',
    test: testName,
    status,
    ok: status === 0,
    duration_ms: Date.now() - started,
    timed_out: timedOut,
    stdout_tail: cleanText(stdoutRaw, 4_000),
    stderr_tail: cleanText(
      `${stderrRaw} ${timedOut ? `timeout_after_ms_${TIMEOUT_MS}` : ''}`.trim(),
      4_000,
    ),
  };
}

function laneSummary(results: SoakCaseResult[]): Record<string, unknown> {
  const lanes: SoakLane[] = ['routing', 'hints', 'synthesis', 'replay'];
  const out: Record<string, unknown> = {};
  for (const lane of lanes) {
    const laneRows = results.filter((row) => row.lane === lane);
    const failed = laneRows.filter((row) => !row.ok);
    out[lane] = {
      total: laneRows.length,
      passed: laneRows.length - failed.length,
      failed: failed.length,
      failed_ids: failed.map((row) => row.id),
      ok: failed.length === 0,
    };
  }
  return out;
}

function caseOk(results: SoakCaseResult[], id: string): boolean {
  return results.find((row) => row.id === id)?.ok === true;
}

function renderMarkdown(report: SoakReport): string {
  const lines: string[] = [];
  lines.push('# Workspace Tooling Context Soak (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.finished_at, 80)}`);
  lines.push(`- ok: ${report.ok ? 'true' : 'false'}`);
  lines.push(`- duration_ms: ${report.duration_ms}`);
  const replayPack = (report.replay_pack || {}) as any;
  const reliabilityPack = (report.reliability_pack || {}) as any;
  lines.push(`- replay_required_missing: ${Number(replayPack.required_missing_count || 0)}`);
  lines.push(`- replay_required_failed: ${Number(replayPack.required_failed_count || 0)}`);
  lines.push(`- reliability_strict_mode: ${reliabilityPack.strict_mode === true ? 'true' : 'false'}`);
  lines.push(`- reliability_misroute_count: ${Number(reliabilityPack.misroute_count || 0)}`);
  lines.push(`- reliability_timeout_count: ${Number(reliabilityPack.timeout_count || 0)}`);
  lines.push(`- reliability_parse_failure_count: ${Number(reliabilityPack.parse_failure_count || 0)}`);
  lines.push('');
  lines.push('## Replay Scenario Coverage');
  const coverage = Array.isArray(replayPack.scenario_coverage) ? replayPack.scenario_coverage : [];
  for (const row of coverage) {
    lines.push(
      `- ${cleanText(row?.scenario || 'unknown', 80)}: covered=${row?.covered === true ? 'true' : 'false'} ok=${row?.ok === true ? 'true' : 'false'} passed=${Number(row?.passed || 0)}/${Number(row?.total || 0)} failed_ids=${(Array.isArray(row?.failed_ids) ? row.failed_ids : []).join(',') || 'none'}`,
    );
  }
  lines.push('');
  lines.push('## Test Results');
  for (const test of report.tests) {
    lines.push(
      `- ${cleanText(test.id, 120)} (${cleanText(test.lane, 40)} / ${cleanText(test.scenario, 80)}): ${test.ok ? 'pass' : 'fail'} (status=${test.status})`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

const startedAt = nowIso();
const startedMs = Date.now();
const results: SoakCaseResult[] = [];
const fixture = readFixture();
const pathTargetingFixture = readPathTargetingFixture();

if (fixture.error || pathTargetingFixture.error) {
  const fixtureError = fixture.error || pathTargetingFixture.error;
  const report: SoakReport = {
    type: 'workspace_tooling_context_soak_report',
    schema_version: 2,
    started_at: startedAt,
    finished_at: nowIso(),
    ok: false,
    command: 'cargo test -p infring-ops-core --lib <workspace-workflow-test-name> -- --nocapture',
    status: 1,
    duration_ms: Date.now() - startedMs,
    taxonomy: {
      family: 'workspace_file_tooling',
      fixture_error: fixtureError,
      path_targeting_fixture_path: path.relative(ROOT, PATH_TARGETING_FIXTURE_PATH),
    },
    taxonomy_contract: {
      ok: false,
      failures: [fixtureError],
    },
    lane_pack: {},
    replay_pack: {
      fixture_error: fixtureError,
    },
    reliability_pack: {
      strict_mode: RELIABILITY_STRICT,
      misroute_count: 1,
      timeout_count: 0,
      parse_failure_count: 0,
      failure_reason: fixtureError,
      ok: !RELIABILITY_STRICT,
    },
    tests: [],
    stdout_tail: '',
    stderr_tail: fixtureError,
  };
  const reliabilityReport = {
    type: 'workspace_tooling_reliability',
    schema_version: 1,
    generated_at: report.finished_at,
    source_report: 'core/local/artifacts/workspace_tooling_context_soak_current.json',
    strict_mode: RELIABILITY_STRICT,
    ok: !RELIABILITY_STRICT,
    counters: {
      misroute_count: 1,
      timeout_count: 0,
      parse_failure_count: 0,
      total_cases: 0,
      failed_cases: 0,
    },
    thresholds: {
      misroute_count_max: 0,
    },
    failure_reason: fixtureError,
  };
  writeJson(CORE_ARTIFACT_PATH, report);
  writeJson(CORE_ARTIFACT_CONTEXT_LATEST_PATH, report);
  writeJson(CORE_ARTIFACT_ALIAS_PATH, report);
  writeJson(CORE_RELIABILITY_PATH, reliabilityReport);
  writeJson(ARTIFACT_RELIABILITY_LATEST_PATH, reliabilityReport);
  writeJson(STATE_RELIABILITY_PATH, reliabilityReport);
  writeJson(STATE_LATEST_PATH, report);
  writeJson(ARTIFACT_CONTEXT_LATEST_PATH, report);
  writeJson(ARTIFACT_ALIAS_LATEST_PATH, report);
  fs.mkdirSync(path.dirname(MARKDOWN_PATH), { recursive: true });
  fs.writeFileSync(MARKDOWN_PATH, renderMarkdown(report), 'utf8');
  process.stdout.write(`${JSON.stringify(report)}\n`);
  process.exit(1);
}

for (const row of fixture.cases) {
  const run = runCargoTestWithTimeoutKill(row.test);
  results.push({
    ...run,
    id: row.id,
    lane: row.lane,
    scenario: row.scenario,
  });
}

const allTestsPassed = results.every((row) => row.ok);
const lanePack = laneSummary(results);
const replayRows = results.filter((row) => row.lane === 'replay');
const replayFailedRows = replayRows.filter((row) => !row.ok);
const replayScenarioCoverage = fixture.requiredReplayScenarios.map((scenario) => {
  const rows = replayRows.filter((row) => row.scenario === scenario);
  const failed = rows.filter((row) => !row.ok);
  return {
    scenario,
    covered: rows.length > 0,
    total: rows.length,
    passed: rows.length - failed.length,
    failed: failed.length,
    failed_ids: failed.map((row) => row.id),
    ok: rows.length > 0 && failed.length === 0,
  };
});
const replayRequiredMissing = replayScenarioCoverage
  .filter((row) => !row.covered)
  .map((row) => row.scenario);
const replayRequiredFailed = replayScenarioCoverage
  .filter((row) => row.covered && !row.ok)
  .map((row) => row.scenario);
const localPathScenarios = new Set<ReplayScenario>(['file_read', 'file_search', 'repo_path_targeting']);
const misrouteRows = results.filter((row) => !row.ok && localPathScenarios.has(row.scenario));
const timeoutRows = results.filter((row) => row.timed_out);
const parseFailurePattern =
  /(parse|parsererror|syntax error|unexpected token|failed to parse|unrecognized escape sequence|deserialize)/i;
const parseFailureRows = results.filter(
  (row) => !row.ok && parseFailurePattern.test(`${row.stderr_tail} ${row.stdout_tail}`),
);
const reliabilityPack = {
  strict_mode: RELIABILITY_STRICT,
  total_cases: results.length,
  failed_cases: results.filter((row) => !row.ok).length,
  misroute_count: misrouteRows.length,
  misroute_ids: misrouteRows.map((row) => row.id),
  timeout_count: timeoutRows.length,
  timeout_ids: timeoutRows.map((row) => row.id),
  parse_failure_count: parseFailureRows.length,
  parse_failure_ids: parseFailureRows.map((row) => row.id),
  thresholds: {
    misroute_count_max: 0,
  },
  ok: RELIABILITY_STRICT ? misrouteRows.length === 0 : true,
};
const replayPack = {
  total: replayRows.length,
  passed: replayRows.length - replayFailedRows.length,
  failed: replayFailedRows.length,
  failed_ids: replayFailedRows.map((row) => row.id),
  required_scenarios: fixture.requiredReplayScenarios,
  required_missing_count: replayRequiredMissing.length,
  required_missing: replayRequiredMissing,
  required_failed_count: replayRequiredFailed.length,
  required_failed: replayRequiredFailed,
  scenario_coverage: replayScenarioCoverage,
};
const taxonomy = {
  family: 'workspace_file_tooling',
  fixture_path: path.relative(ROOT, FIXTURE_PATH),
  path_targeting_fixture_path: path.relative(ROOT, PATH_TARGETING_FIXTURE_PATH),
  path_targeting_total_cases: pathTargetingFixture.cases.length,
  path_targeting_coverage: pathTargetingFixture.coverage,
  total_cases: results.length,
  passed_cases: results.filter((row) => row.ok).length,
  failed_cases: results.filter((row) => !row.ok).length,
  failed_ids: results.filter((row) => !row.ok).map((row) => row.id),
  invariants: {
    file_edit_routes_to_task_lane: caseOk(results, 'routing_file_edit_classifies_to_task_route'),
    workspace_compare_not_forced_to_web: caseOk(results, 'routing_workspace_compare_not_forced_to_web'),
    compare_hint_cluster_includes_workspace_and_web: caseOk(
      results,
      'hints_compare_clusters_workspace_and_web_tools',
    ),
    workspace_web_decomposition_preserves_final_synthesis: caseOk(
      results,
      'synthesis_decomposes_workspace_and_web_evidence',
    ),
    replay_file_read_contract_ok:
      replayScenarioCoverage.find((row) => row.scenario === 'file_read')?.ok === true,
    replay_file_search_contract_ok:
      replayScenarioCoverage.find((row) => row.scenario === 'file_search')?.ok === true,
    replay_repo_path_targeting_contract_ok:
      replayScenarioCoverage.find((row) => row.scenario === 'repo_path_targeting')?.ok === true,
    replay_mixed_workspace_tool_routing_contract_ok:
      replayScenarioCoverage.find((row) => row.scenario === 'mixed_workspace_tool_routing')?.ok === true,
  },
};
const taxonomyInvariantRows = Object.entries(
  (taxonomy.invariants && typeof taxonomy.invariants === 'object' ? taxonomy.invariants : {}) as Record<
    string,
    unknown
  >,
).map(([id, value]) => ({
  id: cleanText(id, 120),
  ok: value === true,
}));
const taxonomyContractFailures = taxonomyInvariantRows
  .filter((row) => !row.ok)
  .map((row) => row.id);
const taxonomyContract = {
  ok: taxonomyContractFailures.length === 0,
  total: taxonomyInvariantRows.length,
  failures: taxonomyContractFailures,
};

const report: SoakReport = {
  type: 'workspace_tooling_context_soak_report',
  schema_version: 2,
  started_at: startedAt,
  finished_at: nowIso(),
  ok:
    allTestsPassed &&
    replayRequiredMissing.length === 0 &&
    replayRequiredFailed.length === 0 &&
    reliabilityPack.ok,
  command: 'cargo test -p infring-ops-core --lib <workspace-workflow-test-name> -- --nocapture',
  status:
    allTestsPassed &&
    replayRequiredMissing.length === 0 &&
    replayRequiredFailed.length === 0 &&
    reliabilityPack.ok
      ? 0
      : 1,
  duration_ms: Date.now() - startedMs,
  taxonomy,
  taxonomy_contract: taxonomyContract,
  lane_pack: lanePack,
  replay_pack: replayPack,
  reliability_pack: reliabilityPack,
  tests: results,
  stdout_tail: cleanText(
    results.map((row) => `${row.id}: ${row.stdout_tail}`).join('\n'),
    4_000,
  ),
  stderr_tail: cleanText(
    results.map((row) => `${row.id}: ${row.stderr_tail}`).join('\n'),
    4_000,
  ),
};

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
const stamp = tsSlug(report.finished_at);
const stampedPath = path.join(ARTIFACT_DIR, `workspace_tooling_context_soak_report_${stamp}.json`);
writeJson(stampedPath, report);
writeJson(ARTIFACT_CONTEXT_LATEST_PATH, report);
writeJson(ARTIFACT_ALIAS_LATEST_PATH, report);
writeJson(STATE_LATEST_PATH, report);
writeJson(CORE_ARTIFACT_PATH, report);
writeJson(CORE_ARTIFACT_CONTEXT_LATEST_PATH, report);
writeJson(CORE_ARTIFACT_ALIAS_PATH, report);
const reliabilityReport = {
  type: 'workspace_tooling_reliability',
  schema_version: 1,
  generated_at: report.finished_at,
  source_report: 'core/local/artifacts/workspace_tooling_context_soak_current.json',
  strict_mode: RELIABILITY_STRICT,
  ok: report.reliability_pack.ok === true,
  counters: {
    misroute_count: Number(report.reliability_pack.misroute_count || 0),
    timeout_count: Number(report.reliability_pack.timeout_count || 0),
    parse_failure_count: Number(report.reliability_pack.parse_failure_count || 0),
    total_cases: Number(report.reliability_pack.total_cases || 0),
    failed_cases: Number(report.reliability_pack.failed_cases || 0),
  },
  thresholds: {
    misroute_count_max: 0,
  },
  failure_ids: {
    misroute: Array.isArray(report.reliability_pack.misroute_ids)
      ? report.reliability_pack.misroute_ids
      : [],
    timeout: Array.isArray(report.reliability_pack.timeout_ids)
      ? report.reliability_pack.timeout_ids
      : [],
    parse_failure: Array.isArray(report.reliability_pack.parse_failure_ids)
      ? report.reliability_pack.parse_failure_ids
      : [],
  },
};
writeJson(CORE_RELIABILITY_PATH, reliabilityReport);
writeJson(ARTIFACT_RELIABILITY_LATEST_PATH, reliabilityReport);
writeJson(STATE_RELIABILITY_PATH, reliabilityReport);
fs.mkdirSync(path.dirname(MARKDOWN_PATH), { recursive: true });
fs.writeFileSync(MARKDOWN_PATH, renderMarkdown(report), 'utf8');

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(report.ok ? 0 : 1);
