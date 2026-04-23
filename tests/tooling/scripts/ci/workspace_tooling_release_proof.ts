#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type ReplayScenario =
  | 'file_read'
  | 'file_search'
  | 'repo_path_targeting'
  | 'mixed_workspace_tool_routing';

const EXPECTED_REQUIRED_REPLAY_SCENARIOS: ReplayScenario[] = [
  'file_read',
  'file_search',
  'repo_path_targeting',
  'mixed_workspace_tool_routing',
];

const EXPECTED_SOAK_LANES = ['routing', 'hints', 'synthesis', 'replay'] as const;
type SoakLane = (typeof EXPECTED_SOAK_LANES)[number];

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/workspace_tooling_release_proof_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    soakPath: cleanText(
      readFlag(argv, 'soak') || 'core/local/artifacts/workspace_tooling_context_soak_current.json',
      400,
    ),
    fallbackSoakPath: cleanText(
      readFlag(argv, 'soak-fallback') || 'artifacts/workspace_tooling_context_soak_report_latest.json',
      400,
    ),
    fixturePath: cleanText(
      readFlag(argv, 'fixture') || 'tests/tooling/fixtures/workspace_tooling_context_replay_matrix.json',
      400,
    ),
    markdownPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/WORKSPACE_TOOLING_RELEASE_PROOF_CURRENT.md',
      400,
    ),
  };
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function duplicateValues(values: string[]): string[] {
  return values.filter((value, index, arr) => arr.indexOf(value) !== index);
}

function isCanonicalRelativePath(value: string, requiredPrefix = ''): boolean {
  const normalized = cleanText(value || '', 400);
  if (!normalized) return false;
  if (path.isAbsolute(normalized)) return false;
  if (normalized.includes('\\')) return false;
  if (normalized.includes('..')) return false;
  if (normalized.includes('//')) return false;
  if (normalized.endsWith('/')) return false;
  if (normalized.includes(' ')) return false;
  if (requiredPrefix && !normalized.startsWith(requiredPrefix)) return false;
  return true;
}

function parseIsoMillis(value: string): number {
  const normalized = cleanText(value || '', 80);
  if (!normalized) return Number.NaN;
  const parsed = Date.parse(normalized);
  if (!Number.isFinite(parsed)) return Number.NaN;
  return parsed;
}

function toScenarioList(raw: unknown): ReplayScenario[] {
  const allowed = new Set<ReplayScenario>([
    'file_read',
    'file_search',
    'repo_path_targeting',
    'mixed_workspace_tool_routing',
  ]);
  if (!Array.isArray(raw)) return [];
  const out: ReplayScenario[] = [];
  const seen = new Set<string>();
  for (const value of raw) {
    const normalized = cleanText(value || '', 80) as ReplayScenario;
    if (!allowed.has(normalized)) continue;
    if (seen.has(normalized)) continue;
    seen.add(normalized);
    out.push(normalized);
  }
  return out;
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Workspace Tooling Release Proof (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(report?.revision || '', 120)}`);
  lines.push(`- pass: ${report?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- soak_report_ok: ${report?.summary?.soak_report_ok === true ? 'true' : 'false'}`);
  lines.push(
    `- required_replay_scenarios_total: ${Number(report?.summary?.required_replay_scenarios_total || 0)}`,
  );
  lines.push(`- replay_missing_count: ${Number(report?.summary?.replay_missing_count || 0)}`);
  lines.push(`- replay_failed_count: ${Number(report?.summary?.replay_failed_count || 0)}`);
  lines.push('');
  lines.push('## Scenario Coverage');
  const rows = Array.isArray(report?.replay_scenarios) ? report.replay_scenarios : [];
  for (const row of rows) {
    lines.push(
      `- ${cleanText(row?.scenario || 'unknown', 80)}: covered=${row?.covered === true ? 'true' : 'false'} ok=${row?.ok === true ? 'true' : 'false'} passed=${Number(row?.passed || 0)}/${Number(row?.total || 0)} failed_ids=${(Array.isArray(row?.failed_ids) ? row.failed_ids : []).join(',') || 'none'}`,
    );
  }
  const failures = Array.isArray(report?.failures) ? report.failures : [];
  if (failures.length > 0) {
    lines.push('');
    lines.push('## Failures');
    for (const failure of failures) {
      lines.push(
        `- ${cleanText(failure?.id || 'unknown', 120)}: ${cleanText(failure?.detail || '', 240)}`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function writeMarkdown(filePath: string, body: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, body, 'utf8');
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const primarySoakAbs = path.resolve(root, args.soakPath);
  const fallbackSoakAbs = path.resolve(root, args.fallbackSoakPath);
  const fixtureAbs = path.resolve(root, args.fixturePath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  const primarySoak = readJsonBestEffort(primarySoakAbs);
  const fallbackSoak = readJsonBestEffort(fallbackSoakAbs);
  const soakPayload = primarySoak || fallbackSoak;
  const fixture = readJsonBestEffort(fixtureAbs);
  const requiredReplayRaw = Array.isArray(fixture?.required_replay_scenarios)
    ? fixture.required_replay_scenarios.map((value: any) => cleanText(value || '', 80))
    : [];
  const requiredReplayScenarios = toScenarioList(fixture?.required_replay_scenarios);
  const replayScenarioRows = Array.isArray(soakPayload?.replay_pack?.scenario_coverage)
    ? soakPayload.replay_pack.scenario_coverage
    : [];
  const soakSelectedSource = primarySoak
    ? args.soakPath
    : fallbackSoak
      ? args.fallbackSoakPath
      : '';
  const soakReplayRequiredScenarioRaw = Array.isArray(soakPayload?.replay_pack?.required_scenarios)
    ? soakPayload.replay_pack.required_scenarios.map((value: any) => cleanText(value || '', 80))
    : [];
  const soakReplayRequiredScenarios = toScenarioList(soakPayload?.replay_pack?.required_scenarios);
  const soakReplayRequiredScenarioDuplicates = duplicateValues(soakReplayRequiredScenarioRaw);
  const soakReplayRequiredScenarioMissingExpected = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter(
    (scenario) => !soakReplayRequiredScenarios.includes(scenario),
  );
  const soakReplayRequiredScenarioUnexpected = soakReplayRequiredScenarios.filter(
    (scenario) => !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(scenario),
  );
  const soakReplayRequiredScenarioOrderCanonical =
    soakReplayRequiredScenarios.join('|') === EXPECTED_REQUIRED_REPLAY_SCENARIOS.join('|');
  const soakReplayScenarioCoverageRaw = Array.isArray(soakPayload?.replay_pack?.scenario_coverage)
    ? soakPayload.replay_pack.scenario_coverage
    : [];
  const soakReplayScenarioCoverageNames = soakReplayScenarioCoverageRaw
    .map((row: any) => cleanText(row?.scenario || '', 80))
    .filter(Boolean);
  const soakReplayScenarioCoverageMissingExpected = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter(
    (scenario) => !soakReplayScenarioCoverageNames.includes(scenario),
  );
  const soakReplayScenarioCoverageUnexpected = soakReplayScenarioCoverageNames.filter(
    (scenario) => !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(scenario as ReplayScenario),
  );
  const soakReplayScenarioCoverageOrderCanonical =
    soakReplayScenarioCoverageNames.join('|') === EXPECTED_REQUIRED_REPLAY_SCENARIOS.join('|');
  const replayScenarioNames = replayScenarioRows
    .map((row: any) => cleanText(row?.scenario || '', 80))
    .filter(Boolean);
  const requiredReplayRawDuplicates = duplicateValues(requiredReplayRaw);
  const requiredReplayMissingExpected = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter(
    (scenario) => !requiredReplayScenarios.includes(scenario),
  );
  const requiredReplayUnexpected = requiredReplayScenarios.filter(
    (scenario) => !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(scenario),
  );
  const requiredReplayOrderCanonical =
    requiredReplayScenarios.join('|') === EXPECTED_REQUIRED_REPLAY_SCENARIOS.join('|');
  const replayScenarioRowDuplicates = duplicateValues(replayScenarioNames);
  const replayScenarioRowsMissingRequired = requiredReplayScenarios.filter(
    (scenario) => !replayScenarioNames.includes(scenario),
  );
  const replayScenarioRowsUnexpected = replayScenarioNames.filter(
    (scenario) => !requiredReplayScenarios.includes(scenario as ReplayScenario),
  );
  const replayScenarioRowsOrderCanonical =
    replayScenarioNames.join('|') === requiredReplayScenarios.join('|');
  const replayScenarioRowsCountMatchesRequired =
    replayScenarioRows.length === requiredReplayScenarios.length;

  const outPathCanonical = isCanonicalRelativePath(args.outPath, 'core/local/artifacts/');
  const outPathCurrentContract = cleanText(args.outPath, 400).endsWith('_current.json');
  const soakPathCanonical = isCanonicalRelativePath(args.soakPath, 'core/local/artifacts/');
  const soakPathCurrentContract = cleanText(args.soakPath, 400).endsWith('_current.json');
  const fallbackSoakPathCanonical = isCanonicalRelativePath(args.fallbackSoakPath, 'artifacts/');
  const fallbackSoakPathLatestContract = cleanText(args.fallbackSoakPath, 400).endsWith(
    '_latest.json',
  );
  const fixturePathCanonical = isCanonicalRelativePath(args.fixturePath, 'tests/tooling/fixtures/');
  const fixturePathJsonContract = cleanText(args.fixturePath, 400).endsWith('.json');
  const markdownPathCanonical = isCanonicalRelativePath(
    args.markdownPath,
    'local/workspace/reports/',
  );
  const markdownPathContract =
    cleanText(args.markdownPath, 400) ===
    'local/workspace/reports/WORKSPACE_TOOLING_RELEASE_PROOF_CURRENT.md';
  const soakSourcePathDistinct = primarySoakAbs !== fallbackSoakAbs;
  const selectedSourceDeclared =
    !soakSelectedSource ||
    soakSelectedSource === args.soakPath ||
    soakSelectedSource === args.fallbackSoakPath;
  const selectedSourceConsistent = primarySoak
    ? soakSelectedSource === args.soakPath
    : fallbackSoak
      ? soakSelectedSource === args.fallbackSoakPath
      : soakSelectedSource === '';
  const outputPathUniqueFromInputs =
    path.resolve(root, args.outPath) !== primarySoakAbs &&
    path.resolve(root, args.outPath) !== fallbackSoakAbs &&
    path.resolve(root, args.outPath) !== fixtureAbs &&
    path.resolve(root, args.outPath) !== markdownAbs;
  const markdownPathUniqueFromInputs =
    markdownAbs !== primarySoakAbs && markdownAbs !== fallbackSoakAbs && markdownAbs !== fixtureAbs;

  const replayByScenario = new Map<string, any>(
    replayScenarioRows.map((row: any) => [cleanText(row?.scenario || '', 80), row]),
  );

  const normalizedScenarioRows = requiredReplayScenarios.map((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const failedIds = Array.isArray(row?.failed_ids)
      ? row.failed_ids.map((value: any) => cleanText(value || '', 120)).filter(Boolean)
      : [];
    return {
      scenario,
      covered: row?.covered === true || Number(row?.total || 0) > 0,
      total: Number(row?.total || 0),
      passed: Number(row?.passed || 0),
      failed: Number(row?.failed || failedIds.length),
      failed_ids: failedIds,
      ok:
        (row?.ok === true || (Number(row?.total || 0) > 0 && Number(row?.failed || failedIds.length) === 0)) &&
        (row?.covered === true || Number(row?.total || 0) > 0),
    };
  });
  const normalizedTotalInvalid = normalizedScenarioRows
    .filter((row) => !Number.isInteger(row.total) || row.total < 0)
    .map((row) => row.scenario);
  const normalizedPassedInvalid = normalizedScenarioRows
    .filter((row) => !Number.isInteger(row.passed) || row.passed < 0)
    .map((row) => row.scenario);
  const normalizedPartitionMismatch = normalizedScenarioRows
    .filter((row) => row.total !== row.passed + row.failed)
    .map((row) => row.scenario);
  const normalizedFailedIdCountMismatch = normalizedScenarioRows
    .filter((row) => row.failed !== row.failed_ids.length)
    .map((row) => row.scenario);
  const normalizedFailedIdDuplicates = normalizedScenarioRows
    .filter((row) => duplicateValues(row.failed_ids).length > 0)
    .map((row) => row.scenario);
  const normalizedOkStateInconsistent = normalizedScenarioRows
    .filter((row) => row.ok && (!row.covered || row.total <= 0 || row.failed > 0))
    .map((row) => row.scenario);
  const normalizedUncoveredNonZero = normalizedScenarioRows
    .filter((row) => !row.covered && (row.total > 0 || row.passed > 0 || row.failed > 0))
    .map((row) => row.scenario);

  const replayMissing = normalizedScenarioRows
    .filter((row) => !row.covered)
    .map((row) => row.scenario);
  const replayFailed = normalizedScenarioRows
    .filter((row) => row.covered && !row.ok)
    .map((row) => row.scenario);
  const requiredReplayScenarioMissingExpected = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter(
    (scenario) => !requiredReplayScenarios.includes(scenario),
  );
  const requiredReplayScenarioUnexpected = requiredReplayScenarios.filter(
    (scenario) => !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(scenario),
  );
  const requiredReplayRawDuplicates = duplicateValues(requiredReplayRaw);
  const soakReplayScenarioDuplicates = duplicateValues(replayScenarioNames);
  const soakReplayScenarioMissingExpected = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter(
    (scenario) => !replayScenarioNames.includes(scenario),
  );
  const soakReplayScenarioUnexpected = replayScenarioNames.filter(
    (scenario) => !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(scenario as ReplayScenario),
  );
  const soakReplayRequiredMissingList = toScenarioList(soakPayload?.replay_pack?.required_missing);
  const soakReplayRequiredFailedList = toScenarioList(soakPayload?.replay_pack?.required_failed);
  const soakReplayRequiredMissingRaw = Array.isArray(soakPayload?.replay_pack?.required_missing)
    ? soakPayload.replay_pack.required_missing.map((value: any) => cleanText(value || '', 80))
    : [];
  const soakReplayRequiredFailedRaw = Array.isArray(soakPayload?.replay_pack?.required_failed)
    ? soakPayload.replay_pack.required_failed.map((value: any) => cleanText(value || '', 80))
    : [];
  const soakReplayRequiredMissingRawDuplicates = duplicateValues(soakReplayRequiredMissingRaw);
  const soakReplayRequiredFailedRawDuplicates = duplicateValues(soakReplayRequiredFailedRaw);
  const soakReplayRequiredMissingUnexpected = soakReplayRequiredMissingList.filter(
    (scenario) => !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(scenario),
  );
  const soakReplayRequiredFailedUnexpected = soakReplayRequiredFailedList.filter(
    (scenario) => !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(scenario),
  );
  const soakReplayRequiredOverlap = soakReplayRequiredMissingList.filter((scenario) =>
    soakReplayRequiredFailedList.includes(scenario),
  );
  const soakReplayRequiredMissingListMatches = soakReplayRequiredMissingList.join('|') === replayMissing.join('|');
  const soakReplayRequiredFailedListMatches = soakReplayRequiredFailedList.join('|') === replayFailed.join('|');
  const soakReplayRequiredMissingCount = Number(soakPayload?.replay_pack?.required_missing_count || 0);
  const soakReplayRequiredFailedCount = Number(soakPayload?.replay_pack?.required_failed_count || 0);
  const soakReplayRequiredMissingCountValid =
    Number.isInteger(soakReplayRequiredMissingCount) && soakReplayRequiredMissingCount >= 0;
  const soakReplayRequiredFailedCountValid =
    Number.isInteger(soakReplayRequiredFailedCount) && soakReplayRequiredFailedCount >= 0;
  const soakReplayRequiredTotal = Array.isArray(soakPayload?.replay_pack?.required_scenarios)
    ? soakPayload.replay_pack.required_scenarios.length
    : 0;
  const soakStatusRaw = soakPayload?.status;
  const soakStatus = Number(soakStatusRaw);
  const soakStatusValid =
    Number.isInteger(soakStatus) && (soakStatus === 0 || soakStatus === 1);
  const soakCommand = cleanText(soakPayload?.command || '', 240);
  const soakCommandValid =
    soakCommand.length > 0 && soakCommand.includes('cargo test -p protheus-ops-core --lib');
  const soakLanePack =
    soakPayload?.lane_pack && typeof soakPayload.lane_pack === 'object'
      ? (soakPayload.lane_pack as Record<string, any>)
      : null;
  const soakLanePackKeys = soakLanePack
    ? Object.keys(soakLanePack)
      .map((value) => cleanText(value || '', 40))
      .filter(Boolean)
    : [];
  const soakLanePackMissingExpected = EXPECTED_SOAK_LANES.filter(
    (lane) => !soakLanePackKeys.includes(lane),
  );
  const soakLanePackUnexpected = soakLanePackKeys.filter(
    (lane) => !EXPECTED_SOAK_LANES.includes(lane as SoakLane),
  );
  const soakLaneRows = EXPECTED_SOAK_LANES.map((lane) => {
    const laneRow = soakLanePack ? soakLanePack[lane] : null;
    const row = laneRow && typeof laneRow === 'object' ? laneRow : null;
    const failedIds = Array.isArray((row as any)?.failed_ids)
      ? (row as any).failed_ids.map((value: any) => cleanText(value || '', 120)).filter(Boolean)
      : [];
    return {
      lane,
      row,
      total: Number((row as any)?.total || 0),
      passed: Number((row as any)?.passed || 0),
      failed: Number((row as any)?.failed || 0),
      ok: (row as any)?.ok === true,
      failed_ids: failedIds,
    };
  });
  const soakLaneRowsMissing = soakLaneRows
    .filter((entry) => !entry.row)
    .map((entry) => entry.lane);
  const soakLaneRowsTotalsInvalid = soakLaneRows
    .filter(
      (entry) =>
        entry.row
        && (!Number.isInteger(entry.total)
          || entry.total < 0
          || !Number.isInteger(entry.passed)
          || entry.passed < 0
          || !Number.isInteger(entry.failed)
          || entry.failed < 0),
    )
    .map((entry) => entry.lane);
  const soakLaneRowsPartitionMismatch = soakLaneRows
    .filter((entry) => entry.row && entry.total !== entry.passed + entry.failed)
    .map((entry) => entry.lane);
  const soakLaneRowsFailedIdsCountMismatch = soakLaneRows
    .filter((entry) => entry.row && entry.failed !== entry.failed_ids.length)
    .map((entry) => entry.lane);
  const soakLaneRowsOkStateInconsistent = soakLaneRows
    .filter(
      (entry) =>
        entry.row
        && ((entry.ok && entry.failed > 0) || (!entry.ok && entry.total > 0 && entry.failed === 0)),
    )
    .map((entry) => entry.lane);
  const fixtureCaseRows = Array.isArray(fixture?.cases) ? fixture.cases : [];
  const fixtureCaseRowsNormalized = fixtureCaseRows.map((row: any) => ({
    id: cleanText(row?.id || '', 120),
    lane: cleanText(row?.lane || '', 40),
    scenario: cleanText(row?.scenario || '', 80),
    test: cleanText(row?.test || '', 200),
  }));
  const fixtureCaseIds = fixtureCaseRowsNormalized.map((row) => row.id).filter(Boolean);
  const fixtureCaseIdPattern = /^[a-z0-9][a-z0-9_-]*$/;
  const fixtureCaseIdDuplicates = duplicateValues(fixtureCaseIds);
  const fixtureCaseIdNoncanonical = fixtureCaseRowsNormalized
    .filter((row) => row.id.length === 0 || !fixtureCaseIdPattern.test(row.id))
    .map((row) => row.id || 'missing_id');
  const fixtureCaseLaneNoncanonical = fixtureCaseRowsNormalized
    .filter((row) => row.id && !EXPECTED_SOAK_LANES.includes(row.lane as SoakLane))
    .map((row) => row.id);
  const fixtureCaseScenarioNoncanonical = fixtureCaseRowsNormalized
    .filter((row) => row.id && !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(row.scenario as ReplayScenario))
    .map((row) => row.id);
  const fixtureCaseTestNameMissing = fixtureCaseRowsNormalized
    .filter((row) => row.id && row.test.length === 0)
    .map((row) => row.id);
  const fixtureMinimumCaseCount = EXPECTED_REQUIRED_REPLAY_SCENARIOS.length;
  const fixtureCaseCountBelowExpectedMinimum = fixtureCaseRowsNormalized.length < fixtureMinimumCaseCount;
  const fixtureCaseIdsSortedCanonical = fixtureCaseIds.join('|') === [...fixtureCaseIds].sort().join('|');
  const fixtureCaseSignatures = fixtureCaseRowsNormalized
    .filter((row) => row.id)
    .map((row) => `${row.lane}|${row.scenario}|${row.test}`);
  const fixtureCaseSignatureDuplicates = duplicateValues(fixtureCaseSignatures);
  const fixtureScenarioCounts = new Map<ReplayScenario, number>();
  for (const scenario of EXPECTED_REQUIRED_REPLAY_SCENARIOS) fixtureScenarioCounts.set(scenario, 0);
  for (const row of fixtureCaseRowsNormalized) {
    if (EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(row.scenario as ReplayScenario)) {
      const scenario = row.scenario as ReplayScenario;
      fixtureScenarioCounts.set(scenario, Number(fixtureScenarioCounts.get(scenario) || 0) + 1);
    }
  }
  const fixtureScenarioMissingCases = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter(
    (scenario) => Number(fixtureScenarioCounts.get(scenario) || 0) <= 0,
  );
  const fixtureById = new Map<string, { id: string; lane: string; scenario: string; test: string }>(
    fixtureCaseRowsNormalized.filter((row) => row.id).map((row) => [row.id, row]),
  );
  const soakTestsRaw = Array.isArray(soakPayload?.tests) ? soakPayload.tests : [];
  const soakTests = soakTestsRaw.map((row: any) => ({
    id: cleanText(row?.id || '', 120),
    lane: cleanText(row?.lane || '', 40),
    scenario: cleanText(row?.scenario || '', 80),
    test: cleanText(row?.test || '', 200),
    status: Number(row?.status),
    ok: row?.ok === true,
    duration_ms: Number(row?.duration_ms),
    timed_out: row?.timed_out === true,
  }));
  const soakTestIds = soakTests.map((row) => row.id).filter(Boolean);
  const soakTestIdDuplicates = duplicateValues(soakTestIds);
  const soakTestsLaneUnexpected = soakTests
    .filter((row) => row.id && !EXPECTED_SOAK_LANES.includes(row.lane as typeof EXPECTED_SOAK_LANES[number]))
    .map((row) => row.id);
  const soakTestsScenarioUnexpected = soakTests
    .filter((row) => row.id && !EXPECTED_REQUIRED_REPLAY_SCENARIOS.includes(row.scenario as ReplayScenario))
    .map((row) => row.id);
  const soakTestsMissingFixtureIds = fixtureCaseIds.filter((id) => !soakTestIds.includes(id));
  const soakTestsUnexpectedFixtureIds = soakTestIds.filter((id) => !fixtureCaseIds.includes(id));
  const soakTestsCountVsFixtureMismatch = soakTests.length !== fixtureCaseRowsNormalized.length;
  const soakTestsIdsSortedCanonical = soakTestIds.join('|') === [...soakTestIds].sort().join('|');
  const soakTestSignatures = soakTests
    .filter((row) => row.id)
    .map((row) => `${row.lane}|${row.scenario}|${row.test}`);
  const soakTestSignatureDuplicates = duplicateValues(soakTestSignatures);
  const soakTestsScenarioVsFixtureCountMismatch = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const fixtureCount = Number(fixtureScenarioCounts.get(scenario) || 0);
    const soakCount = soakTests.filter((row) => row.scenario === scenario).length;
    return fixtureCount !== soakCount;
  });
  const soakTestsById = new Map<string, typeof soakTests[number]>(
    soakTests.filter((row) => row.id).map((row) => [row.id, row]),
  );
  const soakTestsFixtureLaneMismatch = fixtureCaseRowsNormalized
    .filter((row) => row.id && soakTestsById.get(row.id)?.lane !== row.lane)
    .map((row) => row.id);
  const soakTestsFixtureScenarioMismatch = fixtureCaseRowsNormalized
    .filter((row) => row.id && soakTestsById.get(row.id)?.scenario !== row.scenario)
    .map((row) => row.id);
  const soakTestsFixtureTestNameMismatch = fixtureCaseRowsNormalized
    .filter((row) => row.id && soakTestsById.get(row.id)?.test !== row.test)
    .map((row) => row.id);
  const replaySoakTests = soakTests.filter((row) => row.lane === 'replay' && row.id.length > 0);
  const replaySoakTestIds = replaySoakTests.map((row) => row.id);
  const replaySoakPassedCount = replaySoakTests.filter((row) => row.status === 0).length;
  const replaySoakFailedCount = replaySoakTests.filter((row) => row.status !== 0).length;
  const replaySoakFailedIds = replaySoakTests
    .filter((row) => row.status !== 0)
    .map((row) => row.id)
    .filter(Boolean)
    .sort();
  const replayPackTotal = Number(soakPayload?.replay_pack?.total);
  const replayPackPassed = Number(soakPayload?.replay_pack?.passed);
  const replayPackFailed = Number(soakPayload?.replay_pack?.failed);
  const replayPackTotalValid = Number.isInteger(replayPackTotal) && replayPackTotal >= 0;
  const replayPackPassedValid = Number.isInteger(replayPackPassed) && replayPackPassed >= 0;
  const replayPackFailedValid = Number.isInteger(replayPackFailed) && replayPackFailed >= 0;
  const replayPackPartitionMismatch =
    replayPackTotalValid
    && replayPackPassedValid
    && replayPackFailedValid
    && replayPackTotal !== replayPackPassed + replayPackFailed;
  const replayPackTotalVsReplayLaneMismatch =
    replayPackTotalValid && replayPackTotal !== replaySoakTests.length;
  const replayPackPassedVsReplayLaneMismatch =
    replayPackPassedValid && replayPackPassed !== replaySoakPassedCount;
  const replayPackFailedVsReplayLaneMismatch =
    replayPackFailedValid && replayPackFailed !== replaySoakFailedCount;
  const replayPackFailedIdsRaw = Array.isArray(soakPayload?.replay_pack?.failed_ids)
    ? soakPayload.replay_pack.failed_ids.map((value: any) => cleanText(value || '', 120)).filter(Boolean)
    : [];
  const replayPackFailedIdsDuplicates = duplicateValues(replayPackFailedIdsRaw);
  const replayPackFailedIdsSorted = [...replayPackFailedIdsRaw].sort();
  const replayPackFailedIdsVsReplayLaneMismatch =
    replayPackFailedIdsSorted.join('|') !== replaySoakFailedIds.join('|');
  const replayPackFailedIdsUnknownToReplayLane = replayPackFailedIdsRaw.filter(
    (value) => !replaySoakTestIds.includes(value),
  );
  const replayPackFailedIdsCountMismatch =
    replayPackFailedValid && replayPackFailedIdsRaw.length !== replayPackFailed;
  const replayPackFailedIdsMissingWithFailedCount =
    replayPackFailedValid && replayPackFailed > 0 && replayPackFailedIdsRaw.length === 0;
  const replayScenarioTotalVsSoakTestsMismatch = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const rowTotal = Number(row?.total || 0);
    const soakTotal = soakTests.filter((entry) => entry.scenario === scenario).length;
    return rowTotal !== soakTotal;
  });
  const replayScenarioPassedVsSoakTestsMismatch = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const rowPassed = Number(row?.passed || 0);
    const soakPassed = soakTests.filter((entry) => entry.scenario === scenario && entry.status === 0).length;
    return rowPassed !== soakPassed;
  });
  const replayScenarioFailedVsSoakTestsMismatch = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const rowFailed = Number(row?.failed || 0);
    const soakFailed = soakTests.filter((entry) => entry.scenario === scenario && entry.status !== 0).length;
    return rowFailed !== soakFailed;
  });
  const replayScenarioFailedIdsVsSoakTestsMismatch = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const rowFailedIds = Array.isArray(row?.failed_ids)
      ? row.failed_ids.map((value: any) => cleanText(value || '', 120)).filter(Boolean).sort()
      : [];
    const soakFailedIds = soakTests
      .filter((entry) => entry.scenario === scenario && entry.status !== 0)
      .map((entry) => entry.id)
      .filter(Boolean)
      .sort();
    return rowFailedIds.join('|') !== soakFailedIds.join('|');
  });
  const replayScenarioFailedIdsUnknownFixture = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const rowFailedIds = Array.isArray(row?.failed_ids)
      ? row.failed_ids.map((value: any) => cleanText(value || '', 120)).filter(Boolean)
      : [];
    const fixtureIds = fixtureCaseRowsNormalized
      .filter((entry) => entry.scenario === scenario)
      .map((entry) => entry.id)
      .filter(Boolean);
    return rowFailedIds.some((value: string) => !fixtureIds.includes(value));
  });
  const replayScenarioFailedIdsUnknownSoak = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const rowFailedIds = Array.isArray(row?.failed_ids)
      ? row.failed_ids.map((value: any) => cleanText(value || '', 120)).filter(Boolean)
      : [];
    const soakIds = soakTests
      .filter((entry) => entry.scenario === scenario)
      .map((entry) => entry.id)
      .filter(Boolean);
    return rowFailedIds.some((value: string) => !soakIds.includes(value));
  });
  const replayScenarioCoveredVsSoakTestsMismatch = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const rowCovered = row?.covered === true;
    const soakTotal = soakTests.filter((entry) => entry.scenario === scenario).length;
    return rowCovered !== (soakTotal > 0);
  });
  const replayScenarioOkVsSoakFailedMismatch = EXPECTED_REQUIRED_REPLAY_SCENARIOS.filter((scenario) => {
    const row = replayByScenario.get(scenario) || {};
    const rowOk = row?.ok === true;
    const soakFailed = soakTests.filter((entry) => entry.scenario === scenario && entry.status !== 0).length;
    return rowOk !== (soakFailed === 0);
  });
  const replayLaneRow = soakLaneRows.find((entry) => entry.lane === 'replay');
  const replayLanePackVsReplayPackMismatch =
    !!replayLaneRow?.row
    && replayPackTotalValid
    && replayPackPassedValid
    && replayPackFailedValid
    && (replayLaneRow.total !== replayPackTotal
      || replayLaneRow.passed !== replayPackPassed
      || replayLaneRow.failed !== replayPackFailed);
  const replayLanePackFailedIdsVsReplayPackMismatch =
    !!replayLaneRow?.row
    && [...replayLaneRow.failed_ids].sort().join('|') !== replayPackFailedIdsSorted.join('|');
  const soakTestsStatusInvalid = soakTests
    .filter((row) => row.id && (!Number.isInteger(row.status) || row.status < 0))
    .map((row) => row.id);
  const soakTestsTimedOutStatusInconsistent = soakTests
    .filter((row) => row.id && row.timed_out && row.status !== 124)
    .map((row) => row.id);
  const soakTestsOkStatusInconsistent = soakTests
    .filter((row) => row.id && row.ok !== (row.status === 0))
    .map((row) => row.id);
  const soakTestsDurationInvalid = soakTests
    .filter((row) => row.id && (!Number.isInteger(row.duration_ms) || row.duration_ms < 0))
    .map((row) => row.id);
  const soakTestsMissingTestName = soakTests
    .filter((row) => row.id && row.test.length === 0)
    .map((row) => row.id);
  const soakLaneTotalsVsTestsMismatch = EXPECTED_SOAK_LANES.filter((lane) => {
    const laneTests = soakTests.filter((row) => row.lane === lane);
    const laneRow = soakLaneRows.find((row) => row.lane === lane);
    return !!laneRow?.row && laneRow.total !== laneTests.length;
  });
  const soakLaneFailedIdsVsTestsMismatch = EXPECTED_SOAK_LANES.filter((lane) => {
    const laneFailedTestIds = soakTests
      .filter((row) => row.lane === lane && row.status !== 0)
      .map((row) => row.id)
      .filter(Boolean)
      .sort();
    const laneRow = soakLaneRows.find((row) => row.lane === lane);
    const laneFailedIds = laneRow ? [...laneRow.failed_ids].sort() : [];
    return !!laneRow?.row && laneFailedTestIds.join('|') !== laneFailedIds.join('|');
  });
  const soakTaxonomy =
    soakPayload?.taxonomy && typeof soakPayload.taxonomy === 'object'
      ? (soakPayload.taxonomy as Record<string, any>)
      : null;
  const soakTaxonomyTotalCases = Number(soakTaxonomy?.total_cases);
  const soakTaxonomyPassedCases = Number(soakTaxonomy?.passed_cases);
  const soakTaxonomyFailedCases = Number(soakTaxonomy?.failed_cases);
  const soakTaxonomyFailedIds = Array.isArray(soakTaxonomy?.failed_ids)
    ? soakTaxonomy.failed_ids.map((value: any) => cleanText(value || '', 120)).filter(Boolean)
    : [];
  const soakTaxonomyCountsInvalid =
    !Number.isInteger(soakTaxonomyTotalCases)
    || soakTaxonomyTotalCases < 0
    || !Number.isInteger(soakTaxonomyPassedCases)
    || soakTaxonomyPassedCases < 0
    || !Number.isInteger(soakTaxonomyFailedCases)
    || soakTaxonomyFailedCases < 0
    || soakTaxonomyTotalCases !== soakTaxonomyPassedCases + soakTaxonomyFailedCases;
  const soakTaxonomyFailedIdsExpected = soakTests
    .filter((row) => row.status !== 0)
    .map((row) => row.id)
    .filter(Boolean)
    .sort();
  const soakTaxonomyFailedIdsActual = [...soakTaxonomyFailedIds].sort();
  const soakPathCanonical = isCanonicalRelativePath(args.soakPath, 'core/local/artifacts/');
  const fallbackSoakPathCanonical = isCanonicalRelativePath(
    args.fallbackSoakPath,
    'artifacts/',
  );
  const fixturePathCanonical = isCanonicalRelativePath(
    args.fixturePath,
    'tests/tooling/fixtures/',
  );
  const markdownPathCanonical = isCanonicalRelativePath(
    args.markdownPath,
    'local/workspace/reports/',
  );
  const outPathCanonical =
    isCanonicalRelativePath(args.outPath, 'core/local/artifacts/')
    && args.outPath.endsWith('_current.json');
  const soakSourcePathsDistinct = args.soakPath !== args.fallbackSoakPath;
  const selectedSoakSourceExists = soakSelectedSource
    ? fs.existsSync(path.resolve(root, soakSelectedSource))
    : false;
  const selectedSoakSourceMatchesPayloadOrigin =
    (soakSelectedSource === args.soakPath && !!primarySoak)
    || (soakSelectedSource === args.fallbackSoakPath && !primarySoak && !!fallbackSoak);
  const soakStartedAt = cleanText(soakPayload?.started_at || '', 80);
  const soakFinishedAt = cleanText(soakPayload?.finished_at || '', 80);
  const soakStartedMs = parseIsoMillis(soakStartedAt);
  const soakFinishedMs = parseIsoMillis(soakFinishedAt);
  const soakStartedIsoValid = Number.isFinite(soakStartedMs);
  const soakFinishedIsoValid = Number.isFinite(soakFinishedMs);
  const soakTimestampOrderValid =
    Number.isFinite(soakStartedMs)
    && Number.isFinite(soakFinishedMs)
    && soakFinishedMs >= soakStartedMs;
  const soakDurationMsRaw = Number(soakPayload?.duration_ms);
  const soakDurationMsValid = Number.isInteger(soakDurationMsRaw) && soakDurationMsRaw >= 0;
  const soakTimestampDeltaMs =
    soakTimestampOrderValid && Number.isFinite(soakStartedMs) && Number.isFinite(soakFinishedMs)
      ? soakFinishedMs - soakStartedMs
      : Number.NaN;
  const soakDurationMatchesTimestampWindow =
    soakDurationMsValid
    && Number.isFinite(soakTimestampDeltaMs)
    && Math.abs(soakDurationMsRaw - soakTimestampDeltaMs) <= 120_000;
  const soakOkIsBoolean = typeof soakPayload?.ok === 'boolean';
  const soakStatusMatchesOkState =
    soakStatusValid && soakOkIsBoolean
      ? (soakStatus === 0 && soakPayload?.ok === true) || (soakStatus === 1 && soakPayload?.ok === false)
      : false;
  const soakStdoutTailIsString = typeof soakPayload?.stdout_tail === 'string';
  const soakStderrTailIsString = typeof soakPayload?.stderr_tail === 'string';
  const soakCoverageRows = Array.isArray(soakPayload?.replay_pack?.scenario_coverage)
    ? soakPayload.replay_pack.scenario_coverage
    : [];
  const soakCoverageFailedIdDuplicateRows = soakCoverageRows
    .filter((row: any) => {
      const failedIds = Array.isArray(row?.failed_ids)
        ? row.failed_ids.map((value: any) => cleanText(value || '', 120)).filter(Boolean)
        : [];
      return duplicateValues(failedIds).length > 0;
    })
    .map((row: any) => cleanText(row?.scenario || 'unknown', 80));
  const soakCoverageNumericInvalidRows = soakCoverageRows
    .filter((row: any) => {
      const total = Number(row?.total);
      const passed = Number(row?.passed);
      const failed = Number(row?.failed);
      return (
        !Number.isInteger(total)
        || total < 0
        || !Number.isInteger(passed)
        || passed < 0
        || !Number.isInteger(failed)
        || failed < 0
      );
    })
    .map((row: any) => cleanText(row?.scenario || 'unknown', 80));
  const soakCoverageCoveredStateMismatchRows = soakCoverageRows
    .filter((row: any) => {
      const covered = row?.covered === true;
      const total = Number(row?.total || 0);
      const passed = Number(row?.passed || 0);
      const failed = Number(row?.failed || 0);
      return (covered && total <= 0) || (!covered && (total > 0 || passed > 0 || failed > 0));
    })
    .map((row: any) => cleanText(row?.scenario || 'unknown', 80));

  const failures: Array<{ id: string; detail: string }> = [];
  if (!soakPathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_soak_path_noncanonical',
      detail: args.soakPath,
    });
  }
  if (!fallbackSoakPathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_fallback_soak_path_noncanonical',
      detail: args.fallbackSoakPath,
    });
  }
  if (!fixturePathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_fixture_path_noncanonical',
      detail: args.fixturePath,
    });
  }
  if (!markdownPathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_markdown_path_noncanonical',
      detail: args.markdownPath,
    });
  }
  if (!outPathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_out_path_noncanonical',
      detail: args.outPath,
    });
  }
  if (!soakSourcePathsDistinct) {
    failures.push({
      id: 'workspace_tooling_release_proof_soak_source_paths_not_distinct',
      detail: `${args.soakPath}|${args.fallbackSoakPath}`,
    });
  }
  if (!soakPayload) {
    failures.push({
      id: 'workspace_tooling_soak_report_missing',
      detail: `${args.soakPath}|${args.fallbackSoakPath}`,
    });
  } else if (soakPayload?.ok !== true) {
    failures.push({
      id: 'workspace_tooling_soak_report_not_ok',
      detail: cleanText(soakPayload?.status || 'status_unknown', 80),
    });
  }
  if (!fixture) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_missing',
      detail: args.fixturePath,
    });
  } else {
    const fixtureSchemaId = cleanText(fixture?.schema_id || '', 120);
    const fixtureSchemaVersion = Number(fixture?.schema_version || 0);
    if (fixtureSchemaId !== 'workspace_tooling_context_replay_matrix') {
      failures.push({
        id: 'workspace_tooling_replay_fixture_schema_id_invalid',
        detail: fixtureSchemaId || 'missing',
      });
    }
    if (fixtureSchemaVersion !== 1) {
      failures.push({
        id: 'workspace_tooling_replay_fixture_schema_version_invalid',
        detail: Number.isFinite(fixtureSchemaVersion) ? String(fixtureSchemaVersion) : 'missing',
      });
    }
  }
  if (requiredReplayScenarios.length === 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_required_scenarios_missing',
      detail: args.fixturePath,
    });
  }
  if (requiredReplayScenarios.length !== EXPECTED_REQUIRED_REPLAY_SCENARIOS.length) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_required_scenarios_count_noncanonical',
      detail: `actual=${requiredReplayScenarios.length};expected=${EXPECTED_REQUIRED_REPLAY_SCENARIOS.length}`,
    });
  }
  if (requiredReplayScenarios.join('|') !== EXPECTED_REQUIRED_REPLAY_SCENARIOS.join('|')) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_required_scenarios_order_noncanonical',
      detail: `actual=${requiredReplayScenarios.join(',') || 'none'};expected=${EXPECTED_REQUIRED_REPLAY_SCENARIOS.join(',')}`,
    });
  }
  if (requiredReplayRawDuplicates.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_required_scenarios_duplicate',
      detail: Array.from(new Set(requiredReplayRawDuplicates)).join(','),
    });
  }
  if (requiredReplayScenarioMissingExpected.length > 0 || requiredReplayScenarioUnexpected.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_required_scenarios_noncanonical',
      detail: `missing=${requiredReplayScenarioMissingExpected.join(',') || 'none'};unexpected=${requiredReplayScenarioUnexpected.join(',') || 'none'}`,
    });
  }
  if (fixtureCaseIdDuplicates.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_case_ids_duplicate',
      detail: Array.from(new Set(fixtureCaseIdDuplicates)).join(','),
    });
  }
  if (fixtureCaseIdNoncanonical.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_case_id_noncanonical',
      detail: Array.from(new Set(fixtureCaseIdNoncanonical)).join(','),
    });
  }
  if (fixtureCaseLaneNoncanonical.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_case_lane_noncanonical',
      detail: Array.from(new Set(fixtureCaseLaneNoncanonical)).join(','),
    });
  }
  if (fixtureCaseScenarioNoncanonical.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_case_scenario_noncanonical',
      detail: Array.from(new Set(fixtureCaseScenarioNoncanonical)).join(','),
    });
  }
  if (fixtureCaseTestNameMissing.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_case_test_name_missing',
      detail: Array.from(new Set(fixtureCaseTestNameMissing)).join(','),
    });
  }
  if (fixtureCaseCountBelowExpectedMinimum) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_case_count_below_expected_minimum',
      detail: `actual=${fixtureCaseRowsNormalized.length};expected_min=${fixtureMinimumCaseCount}`,
    });
  }
  if (!fixtureCaseIdsSortedCanonical) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_case_ids_order_noncanonical',
      detail: fixtureCaseIds.join(','),
    });
  }
  if (fixtureCaseSignatureDuplicates.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_case_signature_duplicate',
      detail: Array.from(new Set(fixtureCaseSignatureDuplicates)).join(','),
    });
  }
  if (fixtureScenarioMissingCases.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_scenario_missing_cases',
      detail: fixtureScenarioMissingCases.join(','),
    });
  }
  if (!soakPayload) {
    failures.push({
      id: 'workspace_tooling_soak_schema_missing',
      detail: 'workspace_tooling_context_soak_report',
    });
  } else {
    if (!selectedSoakSourceExists) {
      failures.push({
        id: 'workspace_tooling_release_proof_selected_soak_source_missing_on_disk',
        detail: soakSelectedSource || 'selected_source_missing',
      });
    }
    if (!selectedSoakSourceMatchesPayloadOrigin) {
      failures.push({
        id: 'workspace_tooling_release_proof_selected_soak_source_origin_mismatch',
        detail: `selected=${soakSelectedSource || 'none'};primary_loaded=${primarySoak ? 'true' : 'false'};fallback_loaded=${fallbackSoak ? 'true' : 'false'}`,
      });
    }
    if (!soakStartedIsoValid) {
      failures.push({
        id: 'workspace_tooling_soak_started_at_invalid',
        detail: soakStartedAt || 'missing',
      });
    }
    if (!soakFinishedIsoValid) {
      failures.push({
        id: 'workspace_tooling_soak_finished_at_invalid',
        detail: soakFinishedAt || 'missing',
      });
    }
    if (!soakTimestampOrderValid) {
      failures.push({
        id: 'workspace_tooling_soak_timestamp_order_invalid',
        detail: `started_at=${soakStartedAt || 'missing'};finished_at=${soakFinishedAt || 'missing'}`,
      });
    }
    if (!soakDurationMsValid) {
      failures.push({
        id: 'workspace_tooling_soak_duration_invalid',
        detail: String(soakDurationMsRaw),
      });
    }
    if (soakDurationMsValid && soakTimestampOrderValid && !soakDurationMatchesTimestampWindow) {
      failures.push({
        id: 'workspace_tooling_soak_duration_timestamp_delta_mismatch',
        detail: `duration_ms=${soakDurationMsRaw};delta_ms=${String(soakTimestampDeltaMs)}`,
      });
    }
    if (!soakOkIsBoolean) {
      failures.push({
        id: 'workspace_tooling_soak_ok_field_not_boolean',
        detail: cleanText(soakPayload?.ok ?? 'missing', 80),
      });
    }
    if (soakStatusValid && soakOkIsBoolean && !soakStatusMatchesOkState) {
      failures.push({
        id: 'workspace_tooling_soak_status_ok_alignment_invalid',
        detail: `status=${soakStatus};ok=${soakPayload?.ok === true ? 'true' : 'false'}`,
      });
    }
    if (!soakStdoutTailIsString) {
      failures.push({
        id: 'workspace_tooling_soak_stdout_tail_not_string',
        detail: typeof soakPayload?.stdout_tail,
      });
    }
    if (!soakStderrTailIsString) {
      failures.push({
        id: 'workspace_tooling_soak_stderr_tail_not_string',
        detail: typeof soakPayload?.stderr_tail,
      });
    }
    if (soakCoverageFailedIdDuplicateRows.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_coverage_failed_ids_duplicate',
        detail: Array.from(new Set(soakCoverageFailedIdDuplicateRows)).join(','),
      });
    }
    if (soakCoverageNumericInvalidRows.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_coverage_numeric_fields_invalid',
        detail: Array.from(new Set(soakCoverageNumericInvalidRows)).join(','),
      });
    }
    if (soakCoverageCoveredStateMismatchRows.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_coverage_covered_state_mismatch',
        detail: Array.from(new Set(soakCoverageCoveredStateMismatchRows)).join(','),
      });
    }
    if (!soakSelectedSource) {
      failures.push({
        id: 'workspace_tooling_soak_selected_source_missing',
        detail: 'selected',
      });
    }
    if (soakSelectedSource && soakSelectedSource !== args.soakPath && soakSelectedSource !== args.fallbackSoakPath) {
      failures.push({
        id: 'workspace_tooling_soak_selected_source_invalid',
        detail: soakSelectedSource,
      });
    }
    const soakType = cleanText(soakPayload?.type || '', 120);
    const soakSchemaVersion = Number(soakPayload?.schema_version || 0);
    if (soakType !== 'workspace_tooling_context_soak_report') {
      failures.push({
        id: 'workspace_tooling_soak_schema_type_invalid',
        detail: soakType || 'missing',
      });
    }
    if (soakSchemaVersion < 2) {
      failures.push({
        id: 'workspace_tooling_soak_schema_version_invalid',
        detail: Number.isFinite(soakSchemaVersion) ? String(soakSchemaVersion) : 'missing',
      });
    }
    if (cleanText(soakPayload?.taxonomy?.family || '', 80) !== 'workspace_file_tooling') {
      failures.push({
        id: 'workspace_tooling_soak_taxonomy_family_invalid',
        detail: cleanText(soakPayload?.taxonomy?.family || 'missing', 120),
      });
    }
    const soakInvariantEntries = Object.entries(soakPayload?.taxonomy?.invariants || {});
    if (soakInvariantEntries.length === 0) {
      failures.push({
        id: 'workspace_tooling_soak_invariants_missing',
        detail: 'taxonomy.invariants',
      });
    }
    const failedInvariantKeys = soakInvariantEntries
      .filter(([, value]) => value !== true)
      .map(([key]) => cleanText(key, 120));
    if (failedInvariantKeys.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_invariants_failed',
        detail: failedInvariantKeys.join(','),
      });
    }
    if (!Array.isArray(soakPayload?.replay_pack?.required_scenarios)) {
      failures.push({
        id: 'workspace_tooling_soak_required_scenarios_missing',
        detail: 'replay_pack.required_scenarios',
      });
    }
    if (soakReplayRequiredScenarioDuplicates.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_required_scenarios_duplicate',
        detail: Array.from(new Set(soakReplayRequiredScenarioDuplicates)).join(','),
      });
    }
    if (
      soakReplayRequiredScenarioMissingExpected.length > 0
      || soakReplayRequiredScenarioUnexpected.length > 0
    ) {
      failures.push({
        id: 'workspace_tooling_soak_required_scenarios_noncanonical',
        detail: `missing=${soakReplayRequiredScenarioMissingExpected.join(',') || 'none'};unexpected=${soakReplayRequiredScenarioUnexpected.join(',') || 'none'}`,
      });
    }
    if (!soakReplayRequiredScenarioOrderCanonical) {
      failures.push({
        id: 'workspace_tooling_soak_required_scenarios_order_noncanonical',
        detail: `actual=${soakReplayRequiredScenarios.join(',') || 'none'};expected=${EXPECTED_REQUIRED_REPLAY_SCENARIOS.join(',')}`,
      });
    }
    if (!Array.isArray(soakPayload?.replay_pack?.scenario_coverage)) {
      failures.push({
        id: 'workspace_tooling_soak_scenario_coverage_missing',
        detail: 'replay_pack.scenario_coverage',
      });
    }
    if (
      Array.isArray(soakPayload?.replay_pack?.scenario_coverage)
      && soakReplayScenarioCoverageNames.length !== EXPECTED_REQUIRED_REPLAY_SCENARIOS.length
    ) {
      failures.push({
        id: 'workspace_tooling_soak_scenario_coverage_count_mismatch',
        detail: `actual=${soakReplayScenarioCoverageNames.length};expected=${EXPECTED_REQUIRED_REPLAY_SCENARIOS.length}`,
      });
    }
    if (
      soakReplayScenarioCoverageMissingExpected.length > 0
      || soakReplayScenarioCoverageUnexpected.length > 0
    ) {
      failures.push({
        id: 'workspace_tooling_soak_scenario_coverage_noncanonical',
        detail: `missing=${soakReplayScenarioCoverageMissingExpected.join(',') || 'none'};unexpected=${soakReplayScenarioCoverageUnexpected.join(',') || 'none'}`,
      });
    }
    if (Array.isArray(soakPayload?.replay_pack?.scenario_coverage) && !soakReplayScenarioCoverageOrderCanonical) {
      failures.push({
        id: 'workspace_tooling_soak_scenario_coverage_order_noncanonical',
        detail: `actual=${soakReplayScenarioCoverageNames.join(',') || 'none'};expected=${EXPECTED_REQUIRED_REPLAY_SCENARIOS.join(',')}`,
      });
    }
    if (!Array.isArray(soakPayload?.replay_pack?.required_missing)) {
      failures.push({
        id: 'workspace_tooling_soak_required_missing_list_missing',
        detail: 'replay_pack.required_missing',
      });
    }
    if (!Array.isArray(soakPayload?.replay_pack?.required_failed)) {
      failures.push({
        id: 'workspace_tooling_soak_required_failed_list_missing',
        detail: 'replay_pack.required_failed',
      });
    }
    if (soakReplayRequiredMissingRawDuplicates.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_required_missing_list_duplicate',
        detail: Array.from(new Set(soakReplayRequiredMissingRawDuplicates)).join(','),
      });
    }
    if (soakReplayRequiredFailedRawDuplicates.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_required_failed_list_duplicate',
        detail: Array.from(new Set(soakReplayRequiredFailedRawDuplicates)).join(','),
      });
    }
    if (soakReplayRequiredMissingUnexpected.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_required_missing_list_noncanonical',
        detail: soakReplayRequiredMissingUnexpected.join(','),
      });
    }
    if (soakReplayRequiredFailedUnexpected.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_required_failed_list_noncanonical',
        detail: soakReplayRequiredFailedUnexpected.join(','),
      });
    }
    if (soakReplayRequiredOverlap.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_required_missing_failed_overlap',
        detail: Array.from(new Set(soakReplayRequiredOverlap)).join(','),
      });
    }
    if (!soakReplayRequiredMissingCountValid) {
      failures.push({
        id: 'workspace_tooling_soak_required_missing_count_invalid',
        detail: String(soakReplayRequiredMissingCount),
      });
    }
    if (!soakReplayRequiredFailedCountValid) {
      failures.push({
        id: 'workspace_tooling_soak_required_failed_count_invalid',
        detail: String(soakReplayRequiredFailedCount),
      });
    }
    if (soakReplayRequiredMissingCountValid && soakReplayRequiredMissingCount > EXPECTED_REQUIRED_REPLAY_SCENARIOS.length) {
      failures.push({
        id: 'workspace_tooling_soak_required_missing_count_exceeds_expected',
        detail: `count=${soakReplayRequiredMissingCount};expected_max=${EXPECTED_REQUIRED_REPLAY_SCENARIOS.length}`,
      });
    }
    if (soakReplayRequiredFailedCountValid && soakReplayRequiredFailedCount > EXPECTED_REQUIRED_REPLAY_SCENARIOS.length) {
      failures.push({
        id: 'workspace_tooling_soak_required_failed_count_exceeds_expected',
        detail: `count=${soakReplayRequiredFailedCount};expected_max=${EXPECTED_REQUIRED_REPLAY_SCENARIOS.length}`,
      });
    }
    if (!soakStatusValid) {
      failures.push({
        id: 'workspace_tooling_soak_status_invalid',
        detail: cleanText(soakStatusRaw || 'missing', 80),
      });
    }
    if (!soakCommandValid) {
      failures.push({
        id: 'workspace_tooling_soak_command_invalid',
        detail: soakCommand || 'missing',
      });
    }
    if (!soakLanePack) {
      failures.push({
        id: 'workspace_tooling_soak_lane_pack_missing',
        detail: 'lane_pack',
      });
    }
    if (soakLanePackMissingExpected.length > 0 || soakLanePackUnexpected.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_lane_pack_keyset_noncanonical',
        detail: `missing=${soakLanePackMissingExpected.join(',') || 'none'};unexpected=${soakLanePackUnexpected.join(',') || 'none'}`,
      });
    }
    if (soakLaneRowsMissing.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_lane_rows_missing',
        detail: soakLaneRowsMissing.join(','),
      });
    }
    if (soakLaneRowsTotalsInvalid.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_lane_rows_totals_invalid',
        detail: soakLaneRowsTotalsInvalid.join(','),
      });
    }
    if (soakLaneRowsPartitionMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_lane_rows_partition_mismatch',
        detail: soakLaneRowsPartitionMismatch.join(','),
      });
    }
    if (soakLaneRowsFailedIdsCountMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_lane_rows_failed_ids_count_mismatch',
        detail: soakLaneRowsFailedIdsCountMismatch.join(','),
      });
    }
    if (soakLaneRowsOkStateInconsistent.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_lane_rows_ok_state_inconsistent',
        detail: soakLaneRowsOkStateInconsistent.join(','),
      });
    }
    if (!Array.isArray(soakPayload?.tests)) {
      failures.push({
        id: 'workspace_tooling_soak_tests_missing',
        detail: 'tests',
      });
    }
    if (Array.isArray(soakPayload?.tests) && soakTests.length === 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_empty',
        detail: 'tests',
      });
    }
    if (soakTestIdDuplicates.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_id_duplicate',
        detail: Array.from(new Set(soakTestIdDuplicates)).join(','),
      });
    }
    if (soakTestsLaneUnexpected.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_lane_noncanonical',
        detail: Array.from(new Set(soakTestsLaneUnexpected)).join(','),
      });
    }
    if (soakTestsScenarioUnexpected.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_scenario_noncanonical',
        detail: Array.from(new Set(soakTestsScenarioUnexpected)).join(','),
      });
    }
    if (soakTestsMissingFixtureIds.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_fixture_id_missing',
        detail: soakTestsMissingFixtureIds.join(','),
      });
    }
    if (soakTestsUnexpectedFixtureIds.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_unexpected_fixture_id',
        detail: soakTestsUnexpectedFixtureIds.join(','),
      });
    }
    if (soakTestsCountVsFixtureMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_tests_count_vs_fixture_mismatch',
        detail: `tests=${soakTests.length};fixture_cases=${fixtureCaseRowsNormalized.length}`,
      });
    }
    if (!soakTestsIdsSortedCanonical) {
      failures.push({
        id: 'workspace_tooling_soak_tests_ids_order_noncanonical',
        detail: soakTestIds.join(','),
      });
    }
    if (soakTestSignatureDuplicates.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_signature_duplicate',
        detail: Array.from(new Set(soakTestSignatureDuplicates)).join(','),
      });
    }
    if (soakTestsScenarioVsFixtureCountMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_scenario_vs_fixture_count_mismatch',
        detail: soakTestsScenarioVsFixtureCountMismatch.join(','),
      });
    }
    if (soakTestsFixtureLaneMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_fixture_lane_mismatch',
        detail: Array.from(new Set(soakTestsFixtureLaneMismatch)).join(','),
      });
    }
    if (soakTestsFixtureScenarioMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_fixture_scenario_mismatch',
        detail: Array.from(new Set(soakTestsFixtureScenarioMismatch)).join(','),
      });
    }
    if (soakTestsFixtureTestNameMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_fixture_test_name_mismatch',
        detail: Array.from(new Set(soakTestsFixtureTestNameMismatch)).join(','),
      });
    }
    if (!replayPackTotalValid) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_total_invalid',
        detail: String(replayPackTotal),
      });
    }
    if (!replayPackPassedValid) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_passed_invalid',
        detail: String(replayPackPassed),
      });
    }
    if (!replayPackFailedValid) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_failed_invalid',
        detail: String(replayPackFailed),
      });
    }
    if (replayPackPartitionMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_partition_mismatch',
        detail: `total=${replayPackTotal};passed=${replayPackPassed};failed=${replayPackFailed}`,
      });
    }
    if (replayPackTotalVsReplayLaneMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_total_vs_replay_lane_mismatch',
        detail: `pack_total=${replayPackTotal};replay_lane_tests=${replaySoakTests.length}`,
      });
    }
    if (replayPackPassedVsReplayLaneMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_passed_vs_replay_lane_mismatch',
        detail: `pack_passed=${replayPackPassed};replay_lane_passed=${replaySoakPassedCount}`,
      });
    }
    if (replayPackFailedVsReplayLaneMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_failed_vs_replay_lane_mismatch',
        detail: `pack_failed=${replayPackFailed};replay_lane_failed=${replaySoakFailedCount}`,
      });
    }
    if (replayPackFailedIdsDuplicates.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_failed_ids_duplicate',
        detail: Array.from(new Set(replayPackFailedIdsDuplicates)).join(','),
      });
    }
    if (replayPackFailedIdsVsReplayLaneMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_failed_ids_vs_replay_lane_mismatch',
        detail: `pack=${replayPackFailedIdsSorted.join(',') || 'none'};replay_lane=${replaySoakFailedIds.join(',') || 'none'}`,
      });
    }
    if (replayPackFailedIdsUnknownToReplayLane.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_failed_ids_unknown_to_replay_lane',
        detail: Array.from(new Set(replayPackFailedIdsUnknownToReplayLane)).join(','),
      });
    }
    if (replayPackFailedIdsCountMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_failed_ids_count_mismatch',
        detail: `failed_ids=${replayPackFailedIdsRaw.length};failed=${replayPackFailed}`,
      });
    }
    if (replayPackFailedIdsMissingWithFailedCount) {
      failures.push({
        id: 'workspace_tooling_soak_replay_pack_failed_ids_missing_with_failed_count',
        detail: `failed=${replayPackFailed};failed_ids=0`,
      });
    }
    if (replayScenarioTotalVsSoakTestsMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_scenario_total_vs_tests_mismatch',
        detail: replayScenarioTotalVsSoakTestsMismatch.join(','),
      });
    }
    if (replayScenarioPassedVsSoakTestsMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_scenario_passed_vs_tests_mismatch',
        detail: replayScenarioPassedVsSoakTestsMismatch.join(','),
      });
    }
    if (replayScenarioFailedVsSoakTestsMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_scenario_failed_vs_tests_mismatch',
        detail: replayScenarioFailedVsSoakTestsMismatch.join(','),
      });
    }
    if (replayScenarioFailedIdsVsSoakTestsMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_scenario_failed_ids_vs_tests_mismatch',
        detail: replayScenarioFailedIdsVsSoakTestsMismatch.join(','),
      });
    }
    if (replayScenarioFailedIdsUnknownFixture.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_scenario_failed_ids_unknown_fixture',
        detail: replayScenarioFailedIdsUnknownFixture.join(','),
      });
    }
    if (replayScenarioFailedIdsUnknownSoak.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_scenario_failed_ids_unknown_soak',
        detail: replayScenarioFailedIdsUnknownSoak.join(','),
      });
    }
    if (replayScenarioCoveredVsSoakTestsMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_scenario_covered_vs_tests_mismatch',
        detail: replayScenarioCoveredVsSoakTestsMismatch.join(','),
      });
    }
    if (replayScenarioOkVsSoakFailedMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_replay_scenario_ok_vs_failed_mismatch',
        detail: replayScenarioOkVsSoakFailedMismatch.join(','),
      });
    }
    if (replayLanePackVsReplayPackMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_replay_lane_pack_vs_replay_pack_mismatch',
        detail: `lane_total=${replayLaneRow?.total ?? 'missing'};pack_total=${replayPackTotal};lane_passed=${replayLaneRow?.passed ?? 'missing'};pack_passed=${replayPackPassed};lane_failed=${replayLaneRow?.failed ?? 'missing'};pack_failed=${replayPackFailed}`,
      });
    }
    if (replayLanePackFailedIdsVsReplayPackMismatch) {
      failures.push({
        id: 'workspace_tooling_soak_replay_lane_pack_failed_ids_vs_replay_pack_mismatch',
        detail: `lane=${(replayLaneRow?.failed_ids || []).join(',') || 'none'};pack=${replayPackFailedIdsSorted.join(',') || 'none'}`,
      });
    }
    if (soakTestsStatusInvalid.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_status_invalid',
        detail: Array.from(new Set(soakTestsStatusInvalid)).join(','),
      });
    }
    if (soakTestsTimedOutStatusInconsistent.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_timed_out_status_inconsistent',
        detail: Array.from(new Set(soakTestsTimedOutStatusInconsistent)).join(','),
      });
    }
    if (soakTestsOkStatusInconsistent.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_ok_status_inconsistent',
        detail: Array.from(new Set(soakTestsOkStatusInconsistent)).join(','),
      });
    }
    if (soakTestsDurationInvalid.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_duration_invalid',
        detail: Array.from(new Set(soakTestsDurationInvalid)).join(','),
      });
    }
    if (soakTestsMissingTestName.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_tests_test_name_missing',
        detail: Array.from(new Set(soakTestsMissingTestName)).join(','),
      });
    }
    if (soakLaneTotalsVsTestsMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_lane_pack_totals_vs_tests_mismatch',
        detail: soakLaneTotalsVsTestsMismatch.join(','),
      });
    }
    if (soakLaneFailedIdsVsTestsMismatch.length > 0) {
      failures.push({
        id: 'workspace_tooling_soak_lane_pack_failed_ids_vs_tests_mismatch',
        detail: soakLaneFailedIdsVsTestsMismatch.join(','),
      });
    }
    if (!soakTaxonomy) {
      failures.push({
        id: 'workspace_tooling_soak_taxonomy_missing',
        detail: 'taxonomy',
      });
    }
    if (soakTaxonomy && soakTaxonomyCountsInvalid) {
      failures.push({
        id: 'workspace_tooling_soak_taxonomy_counts_invalid',
        detail: `total=${String(soakTaxonomyTotalCases)};passed=${String(soakTaxonomyPassedCases)};failed=${String(soakTaxonomyFailedCases)}`,
      });
    }
    if (soakTaxonomy && soakTaxonomyTotalCases !== soakTests.length) {
      failures.push({
        id: 'workspace_tooling_soak_taxonomy_total_cases_mismatch',
        detail: `taxonomy=${String(soakTaxonomyTotalCases)};tests=${soakTests.length}`,
      });
    }
    if (soakTaxonomy && soakTaxonomyPassedCases !== soakTests.filter((row) => row.status === 0).length) {
      failures.push({
        id: 'workspace_tooling_soak_taxonomy_passed_cases_mismatch',
        detail: `taxonomy=${String(soakTaxonomyPassedCases)};tests=${soakTests.filter((row) => row.status === 0).length}`,
      });
    }
    if (soakTaxonomy && soakTaxonomyFailedCases !== soakTests.filter((row) => row.status !== 0).length) {
      failures.push({
        id: 'workspace_tooling_soak_taxonomy_failed_cases_mismatch',
        detail: `taxonomy=${String(soakTaxonomyFailedCases)};tests=${soakTests.filter((row) => row.status !== 0).length}`,
      });
    }
    if (soakTaxonomy && soakTaxonomyFailedIdsActual.join('|') !== soakTaxonomyFailedIdsExpected.join('|')) {
      failures.push({
        id: 'workspace_tooling_soak_taxonomy_failed_ids_mismatch',
        detail: `taxonomy=${soakTaxonomyFailedIdsActual.join(',') || 'none'};tests=${soakTaxonomyFailedIdsExpected.join(',') || 'none'}`,
      });
    }
  }
  if (soakReplayScenarioDuplicates.length > 0) {
    failures.push({
      id: 'workspace_tooling_soak_replay_scenario_duplicate',
      detail: Array.from(new Set(soakReplayScenarioDuplicates)).join(','),
    });
  }
  if (soakReplayScenarioMissingExpected.length > 0 || soakReplayScenarioUnexpected.length > 0) {
    failures.push({
      id: 'workspace_tooling_soak_replay_scenario_noncanonical',
      detail: `missing=${soakReplayScenarioMissingExpected.join(',') || 'none'};unexpected=${soakReplayScenarioUnexpected.join(',') || 'none'}`,
    });
  }
  if (soakReplayRequiredMissingCount !== replayMissing.length) {
    failures.push({
      id: 'workspace_tooling_soak_replay_missing_count_mismatch',
      detail: `soak=${soakReplayRequiredMissingCount};proof=${replayMissing.length}`,
    });
  }
  if (soakReplayRequiredFailedCount !== replayFailed.length) {
    failures.push({
      id: 'workspace_tooling_soak_replay_failed_count_mismatch',
      detail: `soak=${soakReplayRequiredFailedCount};proof=${replayFailed.length}`,
    });
  }
  if (soakReplayRequiredTotal > 0 && soakReplayRequiredTotal !== requiredReplayScenarios.length) {
    failures.push({
      id: 'workspace_tooling_soak_required_scenario_count_mismatch',
      detail: `soak=${soakReplayRequiredTotal};fixture=${requiredReplayScenarios.length}`,
    });
  }
  if (normalizedTotalInvalid.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_normalized_total_invalid',
      detail: normalizedTotalInvalid.join(','),
    });
  }
  if (normalizedPassedInvalid.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_normalized_passed_invalid',
      detail: normalizedPassedInvalid.join(','),
    });
  }
  if (normalizedPartitionMismatch.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_normalized_partition_mismatch',
      detail: normalizedPartitionMismatch.join(','),
    });
  }
  if (normalizedFailedIdCountMismatch.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_normalized_failed_id_count_mismatch',
      detail: normalizedFailedIdCountMismatch.join(','),
    });
  }
  if (normalizedFailedIdDuplicates.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_normalized_failed_ids_duplicate',
      detail: normalizedFailedIdDuplicates.join(','),
    });
  }
  if (normalizedOkStateInconsistent.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_normalized_ok_state_inconsistent',
      detail: normalizedOkStateInconsistent.join(','),
    });
  }
  if (normalizedUncoveredNonZero.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_normalized_uncovered_nonzero',
      detail: normalizedUncoveredNonZero.join(','),
    });
  }
  if (!soakReplayRequiredMissingListMatches) {
    failures.push({
      id: 'workspace_tooling_soak_required_missing_list_mismatch',
      detail: `soak=${soakReplayRequiredMissingList.join(',') || 'none'};proof=${replayMissing.join(',') || 'none'}`,
    });
  }
  if (!soakReplayRequiredFailedListMatches) {
    failures.push({
      id: 'workspace_tooling_soak_required_failed_list_mismatch',
      detail: `soak=${soakReplayRequiredFailedList.join(',') || 'none'};proof=${replayFailed.join(',') || 'none'}`,
    });
  }
  if (replayMissing.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_required_scenarios_missing',
      detail: replayMissing.join(','),
    });
  }
  if (replayFailed.length > 0) {
    failures.push({
      id: 'workspace_tooling_replay_required_scenarios_failed',
      detail: replayFailed.join(','),
    });
  }
  if (!outPathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_out_path_noncanonical',
      detail: args.outPath,
    });
  }
  if (!outPathCurrentContract) {
    failures.push({
      id: 'workspace_tooling_release_proof_out_path_current_suffix_required',
      detail: args.outPath,
    });
  }
  if (!soakPathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_soak_path_noncanonical',
      detail: args.soakPath,
    });
  }
  if (!soakPathCurrentContract) {
    failures.push({
      id: 'workspace_tooling_release_proof_soak_path_current_suffix_required',
      detail: args.soakPath,
    });
  }
  if (!fallbackSoakPathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_fallback_soak_path_noncanonical',
      detail: args.fallbackSoakPath,
    });
  }
  if (!fallbackSoakPathLatestContract) {
    failures.push({
      id: 'workspace_tooling_release_proof_fallback_soak_path_latest_suffix_required',
      detail: args.fallbackSoakPath,
    });
  }
  if (!fixturePathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_fixture_path_noncanonical',
      detail: args.fixturePath,
    });
  }
  if (!fixturePathJsonContract) {
    failures.push({
      id: 'workspace_tooling_release_proof_fixture_path_json_required',
      detail: args.fixturePath,
    });
  }
  if (!markdownPathCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_markdown_path_noncanonical',
      detail: args.markdownPath,
    });
  }
  if (!markdownPathContract) {
    failures.push({
      id: 'workspace_tooling_release_proof_markdown_path_contract_drift',
      detail: args.markdownPath,
    });
  }
  if (!soakSourcePathDistinct) {
    failures.push({
      id: 'workspace_tooling_release_proof_soak_paths_must_be_distinct',
      detail: `${args.soakPath}=${args.fallbackSoakPath}`,
    });
  }
  if (!selectedSourceDeclared) {
    failures.push({
      id: 'workspace_tooling_release_proof_selected_source_not_declared',
      detail: soakSelectedSource || 'none',
    });
  }
  if (!selectedSourceConsistent) {
    failures.push({
      id: 'workspace_tooling_release_proof_selected_source_inconsistent',
      detail: `selected=${soakSelectedSource || 'none'};primary=${primarySoak ? 'present' : 'missing'};fallback=${fallbackSoak ? 'present' : 'missing'}`,
    });
  }
  if (!outputPathUniqueFromInputs) {
    failures.push({
      id: 'workspace_tooling_release_proof_out_path_collides_with_input_artifact',
      detail: args.outPath,
    });
  }
  if (!markdownPathUniqueFromInputs) {
    failures.push({
      id: 'workspace_tooling_release_proof_markdown_path_collides_with_input_artifact',
      detail: args.markdownPath,
    });
  }
  if (requiredReplayRawDuplicates.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_fixture_required_replay_duplicates',
      detail: requiredReplayRawDuplicates.join(','),
    });
  }
  if (requiredReplayMissingExpected.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_fixture_required_replay_missing_expected',
      detail: requiredReplayMissingExpected.join(','),
    });
  }
  if (requiredReplayUnexpected.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_fixture_required_replay_unexpected',
      detail: requiredReplayUnexpected.join(','),
    });
  }
  if (!requiredReplayOrderCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_fixture_required_replay_order_noncanonical',
      detail: requiredReplayScenarios.join(','),
    });
  }
  if (replayScenarioRowDuplicates.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_replay_rows_duplicate',
      detail: replayScenarioRowDuplicates.join(','),
    });
  }
  if (replayScenarioRowsMissingRequired.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_replay_rows_missing_required',
      detail: replayScenarioRowsMissingRequired.join(','),
    });
  }
  if (replayScenarioRowsUnexpected.length > 0) {
    failures.push({
      id: 'workspace_tooling_release_proof_replay_rows_unexpected',
      detail: replayScenarioRowsUnexpected.join(','),
    });
  }
  if (!replayScenarioRowsOrderCanonical) {
    failures.push({
      id: 'workspace_tooling_release_proof_replay_rows_order_noncanonical',
      detail: replayScenarioNames.join(','),
    });
  }
  if (!replayScenarioRowsCountMatchesRequired) {
    failures.push({
      id: 'workspace_tooling_release_proof_replay_rows_count_mismatch',
      detail: `rows=${replayScenarioRows.length};required=${requiredReplayScenarios.length}`,
    });
  }

  const report = {
    ok: failures.length === 0,
    type: 'workspace_tooling_release_proof',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    soak_sources: {
      primary: args.soakPath,
      fallback: args.fallbackSoakPath,
      selected: soakSelectedSource,
    },
    fixture_path: args.fixturePath,
    summary: {
      pass: failures.length === 0,
      soak_report_ok: soakPayload?.ok === true,
      out_path_noncanonical: !outPathCanonical,
      out_path_current_suffix_drift: !outPathCurrentContract,
      soak_path_noncanonical: !soakPathCanonical,
      soak_path_current_suffix_drift: !soakPathCurrentContract,
      fallback_soak_path_noncanonical: !fallbackSoakPathCanonical,
      fallback_soak_path_latest_suffix_drift: !fallbackSoakPathLatestContract,
      fixture_path_noncanonical: !fixturePathCanonical,
      fixture_path_json_suffix_drift: !fixturePathJsonContract,
      markdown_path_noncanonical: !markdownPathCanonical,
      markdown_path_contract_drift: !markdownPathContract,
      soak_source_paths_not_distinct: !soakSourcePathDistinct,
      selected_source_not_declared: !selectedSourceDeclared,
      selected_source_inconsistent: !selectedSourceConsistent,
      output_path_collides_with_input_artifact: !outputPathUniqueFromInputs,
      markdown_path_collides_with_input_artifact: !markdownPathUniqueFromInputs,
      fixture_required_replay_duplicate_count: requiredReplayRawDuplicates.length,
      fixture_required_replay_missing_expected_count: requiredReplayMissingExpected.length,
      fixture_required_replay_unexpected_count: requiredReplayUnexpected.length,
      fixture_required_replay_order_noncanonical: !requiredReplayOrderCanonical,
      replay_rows_duplicate_count: replayScenarioRowDuplicates.length,
      replay_rows_missing_required_count: replayScenarioRowsMissingRequired.length,
      replay_rows_unexpected_count: replayScenarioRowsUnexpected.length,
      replay_rows_order_noncanonical: !replayScenarioRowsOrderCanonical,
      replay_rows_count_mismatch: !replayScenarioRowsCountMatchesRequired,
      required_replay_scenarios_total: requiredReplayScenarios.length,
      required_replay_scenarios_expected_total: EXPECTED_REQUIRED_REPLAY_SCENARIOS.length,
      required_replay_scenarios_noncanonical_count:
        requiredReplayScenarioMissingExpected.length + requiredReplayScenarioUnexpected.length,
      replay_missing_count: replayMissing.length,
      replay_failed_count: replayFailed.length,
      soak_replay_scenario_duplicate_count: soakReplayScenarioDuplicates.length,
      soak_replay_scenarios_noncanonical_count:
        soakReplayScenarioMissingExpected.length + soakReplayScenarioUnexpected.length,
      soak_required_missing_list_duplicate_count: soakReplayRequiredMissingRawDuplicates.length,
      soak_required_failed_list_duplicate_count: soakReplayRequiredFailedRawDuplicates.length,
      soak_required_missing_list_noncanonical_count: soakReplayRequiredMissingUnexpected.length,
      soak_required_failed_list_noncanonical_count: soakReplayRequiredFailedUnexpected.length,
      soak_required_list_overlap_count: soakReplayRequiredOverlap.length,
      soak_lane_pack_missing_lane_count: soakLaneRowsMissing.length,
      soak_lane_pack_totals_invalid_count: soakLaneRowsTotalsInvalid.length,
      soak_lane_pack_partition_mismatch_count: soakLaneRowsPartitionMismatch.length,
      soak_lane_pack_failed_ids_mismatch_count: soakLaneRowsFailedIdsCountMismatch.length,
      soak_lane_pack_ok_state_inconsistent_count: soakLaneRowsOkStateInconsistent.length,
      soak_tests_total: soakTests.length,
      fixture_case_total: fixtureCaseRowsNormalized.length,
      fixture_case_id_duplicate_count: fixtureCaseIdDuplicates.length,
      fixture_case_id_noncanonical_count: fixtureCaseIdNoncanonical.length,
      fixture_case_lane_noncanonical_count: fixtureCaseLaneNoncanonical.length,
      fixture_case_scenario_noncanonical_count: fixtureCaseScenarioNoncanonical.length,
      fixture_case_test_name_missing_count: fixtureCaseTestNameMissing.length,
      fixture_case_count_below_expected_minimum: fixtureCaseCountBelowExpectedMinimum,
      fixture_case_ids_order_noncanonical: !fixtureCaseIdsSortedCanonical,
      fixture_case_signature_duplicate_count: fixtureCaseSignatureDuplicates.length,
      fixture_scenario_missing_cases_count: fixtureScenarioMissingCases.length,
      soak_tests_duplicate_id_count: soakTestIdDuplicates.length,
      soak_tests_lane_noncanonical_count: soakTestsLaneUnexpected.length,
      soak_tests_scenario_noncanonical_count: soakTestsScenarioUnexpected.length,
      soak_tests_count_vs_fixture_mismatch: soakTestsCountVsFixtureMismatch,
      soak_tests_ids_order_noncanonical: !soakTestsIdsSortedCanonical,
      soak_tests_signature_duplicate_count: soakTestSignatureDuplicates.length,
      soak_tests_scenario_vs_fixture_count_mismatch_count: soakTestsScenarioVsFixtureCountMismatch.length,
      soak_tests_fixture_lane_mismatch_count: soakTestsFixtureLaneMismatch.length,
      soak_tests_fixture_scenario_mismatch_count: soakTestsFixtureScenarioMismatch.length,
      soak_tests_fixture_test_name_mismatch_count: soakTestsFixtureTestNameMismatch.length,
      soak_tests_status_invalid_count: soakTestsStatusInvalid.length,
      soak_tests_timed_out_status_inconsistent_count: soakTestsTimedOutStatusInconsistent.length,
      soak_tests_ok_status_inconsistent_count: soakTestsOkStatusInconsistent.length,
      soak_tests_duration_invalid_count: soakTestsDurationInvalid.length,
      soak_tests_missing_test_name_count: soakTestsMissingTestName.length,
      soak_tests_fixture_id_missing_count: soakTestsMissingFixtureIds.length,
      soak_tests_unexpected_fixture_id_count: soakTestsUnexpectedFixtureIds.length,
      soak_replay_pack_total_invalid: !replayPackTotalValid,
      soak_replay_pack_passed_invalid: !replayPackPassedValid,
      soak_replay_pack_failed_invalid: !replayPackFailedValid,
      soak_replay_pack_partition_mismatch: replayPackPartitionMismatch,
      soak_replay_pack_total_vs_replay_lane_mismatch: replayPackTotalVsReplayLaneMismatch,
      soak_replay_pack_passed_vs_replay_lane_mismatch: replayPackPassedVsReplayLaneMismatch,
      soak_replay_pack_failed_vs_replay_lane_mismatch: replayPackFailedVsReplayLaneMismatch,
      soak_replay_pack_failed_ids_duplicate_count: replayPackFailedIdsDuplicates.length,
      soak_replay_pack_failed_ids_vs_replay_lane_mismatch: replayPackFailedIdsVsReplayLaneMismatch,
      soak_replay_pack_failed_ids_unknown_to_replay_lane_count: replayPackFailedIdsUnknownToReplayLane.length,
      soak_replay_pack_failed_ids_count_mismatch: replayPackFailedIdsCountMismatch,
      soak_replay_pack_failed_ids_missing_with_failed_count: replayPackFailedIdsMissingWithFailedCount,
      soak_replay_scenario_total_vs_tests_mismatch_count: replayScenarioTotalVsSoakTestsMismatch.length,
      soak_replay_scenario_passed_vs_tests_mismatch_count: replayScenarioPassedVsSoakTestsMismatch.length,
      soak_replay_scenario_failed_vs_tests_mismatch_count: replayScenarioFailedVsSoakTestsMismatch.length,
      soak_replay_scenario_failed_ids_vs_tests_mismatch_count: replayScenarioFailedIdsVsSoakTestsMismatch.length,
      soak_replay_scenario_failed_ids_unknown_fixture_count: replayScenarioFailedIdsUnknownFixture.length,
      soak_replay_scenario_failed_ids_unknown_soak_count: replayScenarioFailedIdsUnknownSoak.length,
      soak_replay_scenario_covered_vs_tests_mismatch_count: replayScenarioCoveredVsSoakTestsMismatch.length,
      soak_replay_scenario_ok_vs_failed_mismatch_count: replayScenarioOkVsSoakFailedMismatch.length,
      soak_replay_lane_pack_vs_replay_pack_mismatch: replayLanePackVsReplayPackMismatch,
      soak_replay_lane_pack_failed_ids_vs_replay_pack_mismatch: replayLanePackFailedIdsVsReplayPackMismatch,
      soak_taxonomy_missing: !soakTaxonomy,
      soak_taxonomy_counts_invalid: soakTaxonomy ? soakTaxonomyCountsInvalid : true,
      soak_invariant_failed_count: Object.entries(soakPayload?.taxonomy?.invariants || {}).filter(
        ([, value]) => value !== true,
      ).length,
    },
    replay_scenarios: normalizedScenarioRows,
    failures,
  };

  writeMarkdown(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
