#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type FallbackLoopPolicy = {
  max_repeat_in_window: number;
  window_turns: number;
  repetitive_patterns: string[];
};

type RecoveryPolicy = {
  max_retry: number;
  require_degraded_on_finalization_failure: boolean;
  allowed_terminal_reasons: string[];
};

type Turn = {
  turn_id: string;
  assistant_text: string;
};

type RecoveryCase = {
  id: string;
  turns: Turn[];
  loop_blocked: boolean;
  loop_block_reason?: string;
  finalization_failure: boolean;
  retry_count: number;
  degraded_synthesis_forced: boolean;
  terminal_state_reason: string;
};

type RecoveryFixture = {
  schema_id: string;
  schema_version: number;
  fallback_loop_policy: FallbackLoopPolicy;
  recovery_policy: RecoveryPolicy;
  cases: RecoveryCase[];
};

type CaseEvaluation = {
  id: string;
  ok: boolean;
  loop_detected: boolean;
  loop_detection_source: 'none' | 'pattern_or_heuristic' | 'exact_repeat' | 'alternating_repeat';
  repeated_phrase: string;
  max_consecutive_repeats: number;
  loop_blocked: boolean;
  retry_count: number;
  retry_bounded: boolean;
  finalization_failure: boolean;
  degraded_synthesis_forced: boolean;
  terminal_state_reason: string;
  terminal_reason_allowed: boolean;
  failures: string[];
};

const DEFAULT_FIXTURE_PATH = 'tests/tooling/fixtures/workflow_failure_recovery_matrix.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/workflow_failure_recovery_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/workflow_failure_recovery_latest.json';
const DEFAULT_STATE_PATH = 'local/state/ops/workflow_failure_recovery/latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/WORKFLOW_FAILURE_RECOVERY_CURRENT.md';
const CANONICAL_TOKEN_PATTERN = /^[a-z0-9][a-z0-9._:-]*$/;

function readJson<T>(pathname: string): T | null {
  try {
    return JSON.parse(fs.readFileSync(pathname, 'utf8')) as T;
  } catch {
    return null;
  }
}

function writeJson(pathname: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function writeMarkdown(pathname: string, body: string): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, body, 'utf8');
}

function canonicalStringList(raw: unknown, maxLen = 220): string[] {
  const rows = Array.isArray(raw) ? raw : [];
  return rows.map((value) => cleanText(value || '', maxLen)).filter(Boolean);
}

function duplicateValues(values: string[]): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const value of values) {
    if (!value) continue;
    if (seen.has(value)) {
      duplicates.add(value);
    } else {
      seen.add(value);
    }
  }
  return Array.from(duplicates);
}

function isCanonicalRelativePath(value: string, requiredPrefix = ''): boolean {
  const normalized = cleanText(value || '', 400);
  if (!normalized) return false;
  if (path.isAbsolute(normalized)) return false;
  if (normalized.includes('\\')) return false;
  if (normalized.includes('..')) return false;
  if (normalized.includes('//')) return false;
  if (normalized.endsWith('/')) return false;
  if (requiredPrefix && !normalized.startsWith(requiredPrefix)) return false;
  return true;
}

function normalizeText(raw: string): string {
  return cleanText(raw || '', 8000).toLowerCase();
}

function detectLoopMacroTemplateTag(raw: string): string {
  const text = normalizeText(raw);
  if (!text) return '';
  const policyGateOutageTemplate =
    (text.includes('policy gate') && text.includes('web-provider outage'))
    || text.includes('file list step was blocked before i could finish the answer')
    || text.includes('`file_list` was blocked by ingress delivery policy in this runtime lane')
    || text.includes('lease_denied:client_ingress_domain_boundary')
    || (text.includes('tool trace complete1 done') && text.includes('blocked'));
  if (policyGateOutageTemplate) return 'policy_gate_outage_template';
  const runtimeCapabilitySurfaceTemplate =
    text.includes('i can access runtime telemetry')
    && text.includes('persistent memory')
    && text.includes('workspace files')
    && text.includes('approved command surfaces in this session');
  if (runtimeCapabilitySurfaceTemplate) return 'runtime_capability_surface_template';
  const workflowRetryMacroTemplate =
    (text.includes('workflow gate') && text.includes('unexpected'))
    || (text.includes('final workflow state was unexpected')
      && text.includes('please retry so i can rerun the chain cleanly'))
    || (text.includes('final workflow state was unexpected')
      && text.includes('next actions'))
    || (text.includes('next actions')
      && text.includes('targeted tool call')
      && text.includes('concise answer from current context'));
  if (workflowRetryMacroTemplate) return 'workflow_retry_macro_template';
  const finalReplyRetryTemplate =
    text.includes('final reply did not render')
    || (text.includes('ask me to continue')
      && text.includes('synthesize')
      && text.includes('recorded workflow state'));
  if (finalReplyRetryTemplate) return 'final_reply_retry_template';
  return '';
}

function longestConsecutiveRun(tokens: string[]): { phrase: string; count: number } {
  if (tokens.length === 0) {
    return { phrase: '', count: 0 };
  }
  let bestPhrase = tokens[0];
  let bestCount = 1;
  let currentPhrase = tokens[0];
  let currentCount = 1;
  for (let index = 1; index < tokens.length; index += 1) {
    const next = tokens[index];
    if (next === currentPhrase) {
      currentCount += 1;
    } else {
      if (currentCount > bestCount) {
        bestPhrase = currentPhrase;
        bestCount = currentCount;
      }
      currentPhrase = next;
      currentCount = 1;
    }
  }
  if (currentCount > bestCount) {
    bestPhrase = currentPhrase;
    bestCount = currentCount;
  }
  return { phrase: bestPhrase, count: bestCount };
}

function alternatingRepeatRun(tokens: string[]): { phrase: string; count: number } {
  if (tokens.length < 4) {
    return { phrase: '', count: 0 };
  }
  let bestPhrase = '';
  let bestCount = 0;
  for (let start = 0; start <= tokens.length - 4; start += 1) {
    const first = tokens[start] || '';
    const second = tokens[start + 1] || '';
    if (!first || !second || first === second) continue;
    let count = 2;
    for (let index = start + 2; index < tokens.length; index += 1) {
      const expected = (index - start) % 2 === 0 ? first : second;
      if (tokens[index] !== expected) break;
      count += 1;
    }
    if (count >= 4 && count > bestCount) {
      bestCount = count;
      bestPhrase = `${first} || ${second}`;
    }
  }
  return { phrase: bestPhrase, count: bestCount };
}

function evaluateCase(row: RecoveryCase, fixture: RecoveryFixture): CaseEvaluation {
  const fallbackPolicy = fixture.fallback_loop_policy;
  const recoveryPolicy = fixture.recovery_policy;
  const patterns = canonicalStringList(fallbackPolicy.repetitive_patterns, 600).map((value) =>
    normalizeText(value),
  );
  const turns = Array.isArray(row.turns) ? row.turns : [];
  const recentTurns =
    fallbackPolicy.window_turns > 0 && turns.length > fallbackPolicy.window_turns
      ? turns.slice(turns.length - fallbackPolicy.window_turns)
      : turns;

  const repeatedTokens = recentTurns
    .map((turn) => normalizeText(turn.assistant_text || ''))
    .map((text) => {
      for (const pattern of patterns) {
        if (pattern.length > 0 && text.includes(pattern)) {
          return pattern;
        }
      }
      const heuristicTag = detectLoopMacroTemplateTag(text);
      if (heuristicTag.length > 0) {
        return `heuristic:${heuristicTag}`;
      }
      return '';
    })
    .filter(Boolean);

  const repeatedPatternRun = longestConsecutiveRun(repeatedTokens);
  const repeatedExactRun = longestConsecutiveRun(
    recentTurns
      .map((turn) => normalizeText(turn.assistant_text || ''))
      .filter((text) => text.length >= 40),
  );
  const repeatedAlternatingRun = alternatingRepeatRun(
    recentTurns
      .map((turn) => normalizeText(turn.assistant_text || ''))
      .filter((text) => text.length >= 40),
  );
  let repeatedRun = repeatedPatternRun;
  let loopDetectionSource: 'none' | 'pattern_or_heuristic' | 'exact_repeat' | 'alternating_repeat' =
    repeatedPatternRun.count > 0 ? 'pattern_or_heuristic' : 'none';
  if (repeatedExactRun.count > repeatedRun.count) {
    repeatedRun = repeatedExactRun;
    loopDetectionSource = 'exact_repeat';
  }
  if (repeatedAlternatingRun.count > repeatedRun.count) {
    repeatedRun = repeatedAlternatingRun;
    loopDetectionSource = 'alternating_repeat';
  }
  const loopDetected =
    repeatedRun.count > 0
    && repeatedRun.count > Math.max(0, Number(fallbackPolicy.max_repeat_in_window || 0));

  const retryCount = Math.max(0, Number(row.retry_count || 0));
  const retryBounded = retryCount <= Math.max(0, Number(recoveryPolicy.max_retry || 0));
  const finalizationFailure = row.finalization_failure === true;
  const degradedForced = row.degraded_synthesis_forced === true;
  const terminalReason = cleanText(row.terminal_state_reason || '', 140);
  const allowedReasons = canonicalStringList(recoveryPolicy.allowed_terminal_reasons, 140);
  const terminalReasonAllowed = allowedReasons.includes(terminalReason);

  const failures: string[] = [];
  if (loopDetected && row.loop_blocked !== true) {
    failures.push('repetitive_fallback_loop_not_blocked');
  }
  if (loopDetected && cleanText(row.loop_block_reason || '', 160).length === 0) {
    failures.push('repetitive_fallback_loop_missing_block_reason');
  }
  if (finalizationFailure && !retryBounded) {
    failures.push('finalization_failure_retry_unbounded');
  }
  if (
    finalizationFailure
    && recoveryPolicy.require_degraded_on_finalization_failure === true
    && !degradedForced
  ) {
    failures.push('finalization_failure_missing_forced_degraded_synthesis');
  }
  if (finalizationFailure && !terminalReasonAllowed) {
    failures.push('finalization_failure_terminal_reason_noncanonical');
  }

  return {
    id: cleanText(row.id || '', 120),
    ok: failures.length === 0,
    loop_detected: loopDetected,
    loop_detection_source: loopDetectionSource,
    repeated_phrase: repeatedRun.phrase,
    max_consecutive_repeats: repeatedRun.count,
    loop_blocked: row.loop_blocked === true,
    retry_count: retryCount,
    retry_bounded: retryBounded,
    finalization_failure: finalizationFailure,
    degraded_synthesis_forced: degradedForced,
    terminal_state_reason: terminalReason,
    terminal_reason_allowed: terminalReasonAllowed,
    failures,
  };
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Workflow Failure Recovery (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report?.generated_at || '', 80)}`);
  lines.push(`- strict_mode: ${report?.strict_mode === true ? 'true' : 'false'}`);
  lines.push(`- ok: ${report?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- total_cases: ${Number(report?.summary?.total_cases || 0)}`);
  lines.push(`- loop_detection_count: ${Number(report?.summary?.loop_detection_count || 0)}`);
  lines.push(`- blocked_loop_count: ${Number(report?.summary?.blocked_loop_count || 0)}`);
  lines.push(
    `- finalization_failure_count: ${Number(report?.summary?.finalization_failure_count || 0)}`,
  );
  lines.push(
    `- bounded_retry_violations: ${Number(report?.summary?.bounded_retry_violations || 0)}`,
  );
  lines.push(
    `- missing_forced_degraded_synthesis_count: ${Number(report?.summary?.missing_forced_degraded_synthesis_count || 0)}`,
  );
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    fixturePath: cleanText(readFlag(argv, 'fixture') || DEFAULT_FIXTURE_PATH, 400),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 400),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 400),
    statePath: cleanText(readFlag(argv, 'state') || DEFAULT_STATE_PATH, 400),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 400),
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const fixtureAbs = path.resolve(root, args.fixturePath);
  const fixture = readJson<RecoveryFixture>(fixtureAbs);

  const failures: Array<{ id: string; detail: string }> = [];
  if (!isCanonicalRelativePath(args.fixturePath, 'tests/tooling/fixtures/')) {
    failures.push({
      id: 'workflow_failure_recovery_fixture_path_noncanonical',
      detail: args.fixturePath,
    });
  }
  if (!isCanonicalRelativePath(args.outPath, 'core/local/artifacts/')) {
    failures.push({
      id: 'workflow_failure_recovery_out_path_noncanonical',
      detail: args.outPath,
    });
  }
  if (!cleanText(args.outPath, 400).endsWith('_current.json')) {
    failures.push({
      id: 'workflow_failure_recovery_out_path_current_suffix_required',
      detail: args.outPath,
    });
  }
  if (!isCanonicalRelativePath(args.outLatestPath, 'artifacts/')) {
    failures.push({
      id: 'workflow_failure_recovery_out_latest_path_noncanonical',
      detail: args.outLatestPath,
    });
  }
  if (!cleanText(args.outLatestPath, 400).endsWith('_latest.json')) {
    failures.push({
      id: 'workflow_failure_recovery_out_latest_path_latest_suffix_required',
      detail: args.outLatestPath,
    });
  }
  if (!isCanonicalRelativePath(args.statePath, 'local/state/ops/workflow_failure_recovery/')) {
    failures.push({
      id: 'workflow_failure_recovery_state_path_noncanonical',
      detail: args.statePath,
    });
  }
  if (!isCanonicalRelativePath(args.markdownPath, 'local/workspace/reports/')) {
    failures.push({
      id: 'workflow_failure_recovery_markdown_path_noncanonical',
      detail: args.markdownPath,
    });
  }
  if (
    cleanText(args.markdownPath, 400) !==
    'local/workspace/reports/WORKFLOW_FAILURE_RECOVERY_CURRENT.md'
  ) {
    failures.push({
      id: 'workflow_failure_recovery_markdown_path_contract_drift',
      detail: args.markdownPath,
    });
  }
  const outputPaths = [args.outPath, args.outLatestPath, args.statePath, args.markdownPath];
  if (new Set(outputPaths).size !== outputPaths.length) {
    failures.push({
      id: 'workflow_failure_recovery_output_paths_must_be_distinct',
      detail: outputPaths.join('|'),
    });
  }
  if (!fixture) {
    failures.push({ id: 'fixture_missing', detail: args.fixturePath });
  } else {
    if (cleanText(fixture.schema_id || '', 120) !== 'workflow_failure_recovery_matrix') {
      failures.push({
        id: 'fixture_schema_id_invalid',
        detail: cleanText(fixture.schema_id || 'missing', 120),
      });
    }
    if (Number(fixture.schema_version || 0) !== 1) {
      failures.push({
        id: 'fixture_schema_version_invalid',
        detail: cleanText(String(fixture.schema_version ?? 'missing'), 40),
      });
    }
    if (!Array.isArray(fixture.cases) || fixture.cases.length === 0) {
      failures.push({ id: 'fixture_cases_missing', detail: 'no cases present' });
    }
    const fallbackPolicy =
      fixture.fallback_loop_policy && typeof fixture.fallback_loop_policy === 'object'
        ? fixture.fallback_loop_policy
        : null;
    if (!fallbackPolicy) {
      failures.push({
        id: 'workflow_failure_recovery_fallback_policy_missing',
        detail: 'fallback_loop_policy',
      });
    } else {
      const maxRepeat = Number(fallbackPolicy.max_repeat_in_window);
      const windowTurns = Number(fallbackPolicy.window_turns);
      const repetitivePatterns = canonicalStringList(fallbackPolicy.repetitive_patterns, 600).map(
        (value) => normalizeText(value),
      );
      const repetitivePatternDuplicates = Array.from(new Set(duplicateValues(repetitivePatterns)));
      if (!Number.isInteger(maxRepeat) || maxRepeat < 1) {
        failures.push({
          id: 'workflow_failure_recovery_fallback_max_repeat_invalid',
          detail: cleanText(String(fallbackPolicy.max_repeat_in_window ?? 'missing'), 40),
        });
      }
      if (!Number.isInteger(windowTurns) || windowTurns < 1) {
        failures.push({
          id: 'workflow_failure_recovery_fallback_window_turns_invalid',
          detail: cleanText(String(fallbackPolicy.window_turns ?? 'missing'), 40),
        });
      }
      if (repetitivePatterns.length === 0) {
        failures.push({
          id: 'workflow_failure_recovery_fallback_repetitive_patterns_missing',
          detail: 'fallback_loop_policy.repetitive_patterns',
        });
      }
      if (repetitivePatternDuplicates.length > 0) {
        failures.push({
          id: 'workflow_failure_recovery_fallback_repetitive_patterns_duplicate',
          detail: repetitivePatternDuplicates.join(','),
        });
      }
    }
    const recoveryPolicy =
      fixture.recovery_policy && typeof fixture.recovery_policy === 'object'
        ? fixture.recovery_policy
        : null;
    if (!recoveryPolicy) {
      failures.push({
        id: 'workflow_failure_recovery_recovery_policy_missing',
        detail: 'recovery_policy',
      });
    } else {
      const maxRetry = Number(recoveryPolicy.max_retry);
      const requireDegraded = recoveryPolicy.require_degraded_on_finalization_failure;
      const allowedReasons = canonicalStringList(recoveryPolicy.allowed_terminal_reasons, 140);
      const allowedReasonDuplicates = Array.from(new Set(duplicateValues(allowedReasons)));
      if (!Number.isInteger(maxRetry) || maxRetry < 0) {
        failures.push({
          id: 'workflow_failure_recovery_recovery_max_retry_invalid',
          detail: cleanText(String(recoveryPolicy.max_retry ?? 'missing'), 40),
        });
      }
      if (typeof requireDegraded !== 'boolean') {
        failures.push({
          id: 'workflow_failure_recovery_recovery_require_degraded_bool_invalid',
          detail: cleanText(String(requireDegraded ?? 'missing'), 40),
        });
      }
      if (allowedReasons.length === 0) {
        failures.push({
          id: 'workflow_failure_recovery_recovery_allowed_terminal_reasons_missing',
          detail: 'recovery_policy.allowed_terminal_reasons',
        });
      }
      if (allowedReasonDuplicates.length > 0) {
        failures.push({
          id: 'workflow_failure_recovery_recovery_allowed_terminal_reasons_duplicate',
          detail: allowedReasonDuplicates.join(','),
        });
      }
    }
    const caseIds = (fixture.cases || []).map((row) => cleanText(row?.id || '', 120));
    const duplicateCaseIds = Array.from(new Set(duplicateValues(caseIds.filter(Boolean))));
    if (duplicateCaseIds.length > 0) {
      failures.push({
        id: 'workflow_failure_recovery_case_ids_duplicate',
        detail: duplicateCaseIds.join(','),
      });
    }
    const noncanonicalCaseIds = caseIds.filter((id) => id.length === 0 || !CANONICAL_TOKEN_PATTERN.test(id));
    if (noncanonicalCaseIds.length > 0) {
      failures.push({
        id: 'workflow_failure_recovery_case_ids_noncanonical',
        detail: Array.from(new Set(noncanonicalCaseIds)).join(','),
      });
    }
  }

  const evaluations = (fixture?.cases || []).map((row) => evaluateCase(row, fixture as RecoveryFixture));
  for (const row of evaluations.filter((entry) => !entry.ok)) {
    failures.push({
      id: `case:${row.id}`,
      detail: cleanText(row.failures.join(';') || 'failed', 500),
    });
  }

  const loopDetectionCount = evaluations.filter((row) => row.loop_detected).length;
  const blockedLoopCount = evaluations.filter((row) => row.loop_detected && row.loop_blocked).length;
  const finalizationFailureCount = evaluations.filter((row) => row.finalization_failure).length;
  const boundedRetryViolations = evaluations.filter(
    (row) => row.finalization_failure && !row.retry_bounded,
  ).length;
  const missingForcedDegradedCount = evaluations.filter(
    (row) => row.finalization_failure && !row.degraded_synthesis_forced,
  ).length;
  const terminalReasons = Array.from(
    new Set(
      evaluations
        .filter((row) => row.terminal_state_reason.length > 0)
        .map((row) => row.terminal_state_reason),
    ),
  );

  const gateChecks = [
    {
      id: 'repetitive_fallback_loop_blocked',
      ok: evaluations.every((row) => !row.loop_detected || row.loop_blocked),
      detail: `loop_detected=${loopDetectionCount};blocked=${blockedLoopCount}`,
    },
    {
      id: 'finalization_failure_retry_bounded',
      ok: boundedRetryViolations === 0,
      detail: `value=${boundedRetryViolations};max=0`,
    },
    {
      id: 'finalization_failure_forced_degraded_synthesis',
      ok: missingForcedDegradedCount === 0,
      detail: `value=${missingForcedDegradedCount};max=0`,
    },
    {
      id: 'finalization_failure_terminal_reason_canonical',
      ok: evaluations.every((row) => !row.finalization_failure || row.terminal_reason_allowed),
      detail: `invalid=${evaluations.filter((row) => row.finalization_failure && !row.terminal_reason_allowed).length}`,
    },
  ];

  const allChecksPass = failures.length === 0 && gateChecks.every((row) => row.ok);
  const report = {
    type: 'workflow_failure_recovery',
    schema_version: 1,
    generated_at: new Date().toISOString(),
    strict_mode: args.strict,
    ok: allChecksPass,
    fixture_path: args.fixturePath,
    summary: {
      total_cases: evaluations.length,
      passed_cases: evaluations.filter((row) => row.ok).length,
      failed_cases: evaluations.filter((row) => !row.ok).length,
      loop_detection_count: loopDetectionCount,
      blocked_loop_count: blockedLoopCount,
      finalization_failure_count: finalizationFailureCount,
      bounded_retry_violations: boundedRetryViolations,
      missing_forced_degraded_synthesis_count: missingForcedDegradedCount,
      terminal_state_reasons: terminalReasons,
    },
    gate_checks: gateChecks,
    case_results: evaluations,
    failures,
  };

  const outAbs = path.resolve(root, args.outPath);
  const latestAbs = path.resolve(root, args.outLatestPath);
  const stateAbs = path.resolve(root, args.statePath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  writeJson(outAbs, report);
  writeJson(latestAbs, report);
  writeJson(stateAbs, report);
  writeMarkdown(markdownAbs, renderMarkdown(report));

  const exitCode = args.strict ? (allChecksPass ? 0 : 1) : 0;
  emitStructuredResult(
    {
      ok: allChecksPass,
      report_path: args.outPath,
      latest_path: args.outLatestPath,
      markdown_path: args.markdownPath,
      failures: failures.length,
    },
    { outPath: args.outPath },
  );
  return exitCode;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  process.exit(run());
}
