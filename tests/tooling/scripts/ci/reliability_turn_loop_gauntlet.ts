#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

type GauntletCheck = {
  id: string;
  lane: 'continuity' | 'tool_completion' | 'liveness' | 'lifecycle' | 'e2e';
  name: string;
  test: string;
};

type GauntletResult = {
  id: string;
  lane: GauntletCheck['lane'];
  name: string;
  test: string;
  status: number;
  ok: boolean;
  duration_ms: number;
  stdout_tail: string;
  stderr_tail: string;
  failure_reason: string;
};

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const STATE_DIR = path.join(ROOT, 'local', 'state', 'ops', 'reliability_turn_loop_gauntlet');
const STATE_LATEST_PATH = path.join(STATE_DIR, 'latest.json');

const CHECK_TIMEOUT_MS = Math.max(
  30_000,
  Number.parseInt(process.env.INFRING_RELIABILITY_GAUNTLET_TEST_TIMEOUT_MS || '600000', 10) || 600_000,
);

const CHECKS: GauntletCheck[] = [
  {
    id: 'continuity_prefers_earliest_turn',
    lane: 'continuity',
    name: 'Memory recall prefers the earliest active-session turn when intent is first-chat recall',
    test: 'memory_recall_prefers_active_session_earliest_turn_for_first_chat_queries',
  },
  {
    id: 'continuity_active_session_scope',
    lane: 'continuity',
    name: 'Memory recall remains scoped to active session history',
    test: 'memory_recall_stays_scoped_to_active_session_history',
  },
  {
    id: 'tool_completion_no_ack_leak',
    lane: 'tool_completion',
    name: 'Final user response never leaks raw tool acknowledgement text',
    test: 'finalize_user_facing_response_never_leaks_tool_status_text',
  },
  {
    id: 'tool_completion_actionable_read_failure',
    lane: 'tool_completion',
    name: 'Transient request_read_failed web search failure returns actionable summary',
    test: 'web_search_request_read_failed_summary_is_actionable',
  },
  {
    id: 'tool_completion_actionable_steps_no_off_topic_dump',
    lane: 'tool_completion',
    name: 'Actionable-step prompts reject unrelated programming dump output',
    test: 'workflow_actionable_steps_request_rejects_unrelated_programming_dump',
  },
  {
    id: 'tool_completion_meta_control_blocks_web_tools',
    lane: 'tool_completion',
    name: 'Meta-control turns do not trigger web tool execution',
    test: 'meta_control_turn_does_not_trigger_web_tool_execution',
  },
  {
    id: 'tool_completion_web_failure_still_returns_final_response',
    lane: 'tool_completion',
    name: 'Web tool failures still return deterministic final user response',
    test: 'workflow_web_tool_failure_still_returns_final_user_response',
  },
  {
    id: 'liveness_active_agent_not_hidden',
    lane: 'liveness',
    name: 'Active collab agent remains visible despite stale terminated contract',
    test: 'active_collab_agent_is_not_hidden_by_stale_terminated_contract',
  },
  {
    id: 'lifecycle_descendant_management_scope',
    lane: 'lifecycle',
    name: 'Agent management remains scoped to parent-descendant lineage',
    test: 'actor_agent_management_is_scoped_to_descendants',
  },
  {
    id: 'lifecycle_terminal_policy_block_summary',
    lane: 'lifecycle',
    name: 'Policy-denied terminal command returns deterministic structured summary',
    test: 'agent_terminal_blocks_policy_denied_command_with_structured_summary',
  },
  {
    id: 'e2e_capability_gauntlet',
    lane: 'e2e',
    name: 'Capability gauntlet executes full 20-task reliability sweep',
    test: 'agent_capability_gauntlet_20_difficult_tasks',
  },
  {
    id: 'e2e_web_tooling_context_soak',
    lane: 'e2e',
    name: '32-turn mixed web/meta soak preserves terminal response contract',
    test: 'workflow_web_tooling_context_soak_32_turns_reports_zero_terminal_failures',
  },
  {
    id: 'e2e_web_transcript_replay_guard',
    lane: 'e2e',
    name: 'Transcript replay guard prevents speculative blocker copy resurrection',
    test: 'workflow_repair_does_not_resurrect_prior_speculative_web_blocker_copy',
  },
  {
    id: 'e2e_workflow_system_fallback_contract',
    lane: 'e2e',
    name: 'Workflow system fallback only triggers on final-stage synthesis failures',
    test: 'workflow_system_fallback_requires_final_stage_failure',
  },
];

function selectedChecks(): GauntletCheck[] {
  const raw = String(process.env.INFRING_RELIABILITY_GAUNTLET_ONLY || '').trim();
  if (!raw) return CHECKS;
  const allow = new Set(
    raw
      .split(',')
      .map((row) => row.trim().toLowerCase())
      .filter(Boolean),
  );
  const filtered = CHECKS.filter(
    (row) => allow.has(row.id.toLowerCase()) || allow.has(row.test.toLowerCase()),
  );
  return filtered.length > 0 ? filtered : CHECKS;
}

function nowIso(): string {
  return new Date().toISOString();
}

function tsSlug(iso: string): string {
  return iso.replaceAll(':', '-').replaceAll('.', '-');
}

function cleanText(raw: unknown, maxLen = 1600): string {
  return String(raw ?? '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function writeJson(pathname: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function runCargoTestWithTimeoutKill(testName: string) {
  const out = spawnSync(
    'cargo',
    ['test', '-p', 'protheus-ops-core', '--lib', testName, '--quiet', '--', '--nocapture'],
    {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: CHECK_TIMEOUT_MS,
      killSignal: 'SIGKILL',
    },
  );
  const timeoutMessage = String(out.error?.message || '').toLowerCase();
  const timedOut =
    !!out.error && (timeoutMessage.includes('timed out') || timeoutMessage.includes('etimedout'));
  if (timedOut) {
    const pattern = `cargo test -p protheus-ops-core --lib ${testName}`;
    spawnSync('pkill', ['-TERM', '-f', pattern], { cwd: ROOT, stdio: 'ignore' });
    spawnSync('pkill', ['-KILL', '-f', pattern], { cwd: ROOT, stdio: 'ignore' });
  }
  return {
    out,
    timedOut,
  };
}

function runCheck(check: GauntletCheck): GauntletResult {
  const started = Date.now();
  const { out, timedOut } = runCargoTestWithTimeoutKill(check.test);
  const status = timedOut ? 124 : Number.isFinite(out.status) ? Number(out.status) : 1;
  const durationMs = Date.now() - started;
  const failureReason = timedOut
    ? `timeout_after_ms_${CHECK_TIMEOUT_MS}`
    : cleanText(out.error?.message || '', 220);
  return {
    id: check.id,
    lane: check.lane,
    name: check.name,
    test: check.test,
    status,
    ok: status === 0,
    duration_ms: durationMs,
    stdout_tail: cleanText(out.stdout, 1400),
    stderr_tail: cleanText(`${out.stderr || ''} ${failureReason}`.trim(), 1400),
    failure_reason: failureReason,
  };
}

function laneSummary(results: GauntletResult[]): Record<string, { total: number; passed: number; ok: boolean }> {
  const lanes = ['continuity', 'tool_completion', 'liveness', 'lifecycle', 'e2e'];
  const out: Record<string, { total: number; passed: number; ok: boolean }> = {};
  for (const lane of lanes) {
    const laneRows = results.filter((row) => row.lane === lane);
    const passed = laneRows.filter((row) => row.ok).length;
    out[lane] = {
      total: laneRows.length,
      passed,
      ok: laneRows.length === 0 ? true : passed === laneRows.length,
    };
  }
  return out;
}

function recoveryHints(results: GauntletResult[]): string[] {
  const failedLanes = new Set(results.filter((row) => !row.ok).map((row) => row.lane));
  const hints: string[] = [];
  if (failedLanes.has('continuity')) {
    hints.push(
      'Continuity lane failed: inspect active-session compaction + recall context in core/layer0/ops/src/dashboard_compat_api_parts.',
    );
  }
  if (failedLanes.has('tool_completion')) {
    hints.push(
      'Tool completion lane failed: verify finalize_user_facing_response + tool_output_match_filter suppression contracts.',
    );
  }
  if (failedLanes.has('liveness')) {
    hints.push(
      'Liveness lane failed: validate roster visibility filters and terminated-contract reconciliation in dashboard agent state.',
    );
  }
  if (failedLanes.has('lifecycle')) {
    hints.push(
      'Lifecycle lane failed: re-check descendant permission gates and deterministic terminal policy summaries.',
    );
  }
  if (failedLanes.has('e2e')) {
    hints.push('E2E gauntlet failed: run the failing test locally with --nocapture and inspect scenario receipts.');
  }
  if (hints.length === 0) {
    hints.push('Gauntlet healthy: all reliability lanes passed.');
  }
  return hints;
}

const startedAt = nowIso();
const checksToRun = selectedChecks();
const results = checksToRun.map(runCheck);
const lanes = laneSummary(results);
const failures = results
  .filter((row) => !row.ok)
  .map((row) => ({
    id: row.id,
    lane: row.lane,
    test: row.test,
    status: row.status,
  }));
const ok = failures.length === 0 && Object.values(lanes).every((row) => row.ok);

const report = {
  type: 'reliability_turn_loop_gauntlet_report',
  schema_version: 1,
  started_at: startedAt,
  finished_at: nowIso(),
  config: {
    check_timeout_ms: CHECK_TIMEOUT_MS,
    selected_checks: checksToRun.map((row) => row.id),
  },
  ok,
  lanes,
  checks: results,
  failures,
  recovery_hints: recoveryHints(results),
  command: 'cargo test -p protheus-ops-core --lib <test-name> -- --nocapture',
};

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
const stamp = tsSlug(report.finished_at);
const stampedPath = path.join(ARTIFACT_DIR, `reliability_turn_loop_gauntlet_report_${stamp}.json`);
const latestPath = path.join(ARTIFACT_DIR, 'reliability_turn_loop_gauntlet_report_latest.json');
writeJson(stampedPath, report);
writeJson(latestPath, report);
writeJson(STATE_LATEST_PATH, report);

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(ok ? 0 : 1);
