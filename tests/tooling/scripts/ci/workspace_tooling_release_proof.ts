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
  const requiredReplayScenarios = toScenarioList(fixture?.required_replay_scenarios);
  const replayScenarioRows = Array.isArray(soakPayload?.replay_pack?.scenario_coverage)
    ? soakPayload.replay_pack.scenario_coverage
    : [];

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

  const replayMissing = normalizedScenarioRows
    .filter((row) => !row.covered)
    .map((row) => row.scenario);
  const replayFailed = normalizedScenarioRows
    .filter((row) => row.covered && !row.ok)
    .map((row) => row.scenario);

  const failures: Array<{ id: string; detail: string }> = [];
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
  }
  if (requiredReplayScenarios.length === 0) {
    failures.push({
      id: 'workspace_tooling_replay_fixture_required_scenarios_missing',
      detail: args.fixturePath,
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

  const report = {
    ok: failures.length === 0,
    type: 'workspace_tooling_release_proof',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    soak_sources: {
      primary: args.soakPath,
      fallback: args.fallbackSoakPath,
      selected: primarySoak ? args.soakPath : fallbackSoak ? args.fallbackSoakPath : '',
    },
    fixture_path: args.fixturePath,
    summary: {
      pass: failures.length === 0,
      soak_report_ok: soakPayload?.ok === true,
      required_replay_scenarios_total: requiredReplayScenarios.length,
      replay_missing_count: replayMissing.length,
      replay_failed_count: replayFailed.length,
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
