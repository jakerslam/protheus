#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_MONITOR_PATH = 'local/state/ops/eval_agent_chat_monitor/latest.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_monitor_slo_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_monitor_slo_latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_MONITOR_SLO_CURRENT.md';
const DEFAULT_MAX_AGE_SECONDS = 6 * 60 * 60;
const DEFAULT_MAX_INTERVAL_SECONDS = 24 * 60 * 60;
const CANONICAL_TOKEN = /^[a-z0-9][a-z0-9._:-]*$/;

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  const maxAgeSeconds = Number.parseInt(cleanText(readFlag(argv, 'max-age-seconds') || '', 20), 10);
  const maxIntervalSeconds = Number.parseInt(cleanText(readFlag(argv, 'max-interval-seconds') || '', 20), 10);
  return {
    strict: common.strict,
    monitorPath: cleanText(readFlag(argv, 'monitor') || DEFAULT_MONITOR_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
    maxAgeSeconds: Number.isFinite(maxAgeSeconds) && maxAgeSeconds > 0 ? maxAgeSeconds : DEFAULT_MAX_AGE_SECONDS,
    maxIntervalSeconds:
      Number.isFinite(maxIntervalSeconds) && maxIntervalSeconds > 0
        ? maxIntervalSeconds
        : DEFAULT_MAX_INTERVAL_SECONDS,
  };
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function parseIso(raw: string): number {
  const parsed = Date.parse(cleanText(raw, 120));
  return Number.isFinite(parsed) ? parsed : 0;
}

function isCanonicalRelativePath(value: string, requiredPrefix = ''): boolean {
  const normalized = cleanText(value || '', 500);
  if (!normalized) return false;
  if (path.isAbsolute(normalized)) return false;
  if (normalized.includes('\\')) return false;
  if (normalized.includes('..')) return false;
  if (normalized.includes('//')) return false;
  if (normalized.endsWith('/')) return false;
  if (requiredPrefix && !normalized.startsWith(requiredPrefix)) return false;
  return true;
}

function duplicateValues(values: string[]): string[] {
  return values.filter((value, index, rows) => rows.indexOf(value) !== index);
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Monitor SLO Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- report_age_seconds: ${Number(report.metrics?.report_age_seconds || 0)}`);
  lines.push(`- run_interval_seconds: ${Number(report.metrics?.run_interval_seconds || 0)}`);
  lines.push(`- status: ${cleanText(report.metrics?.status || 'unknown', 80)}`);
  lines.push(`- alert_count: ${Number(report.summary?.alert_count || 0)}`);
  lines.push('');
  const alerts = Array.isArray(report.alerts) ? report.alerts : [];
  lines.push('## Alerts');
  if (alerts.length === 0) {
    lines.push('- none');
  } else {
    alerts.forEach((row) => {
      lines.push(`- [${cleanText(row?.severity || 'info', 20)}] ${cleanText(row?.id || 'alert', 120)}: ${cleanText(row?.detail || '', 260)}`);
    });
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const monitorAbs = path.resolve(root, args.monitorPath);
  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  const nowMs = Date.now();
  const nowIso = new Date(nowMs).toISOString();

  const monitor = readJson(monitorAbs) || {};
  const generatedAt = cleanText(monitor?.generated_at || monitor?.ts || '', 120);
  const generatedAtMs = parseIso(generatedAt);
  const reportAgeSeconds = generatedAtMs > 0 ? Math.max(0, Math.floor((nowMs - generatedAtMs) / 1000)) : Number.POSITIVE_INFINITY;
  const previousTs = cleanText(
    monitor?.summary?.troubleshooting_latest_eval?.previous_ts
      || monitor?.previous_ts
      || '',
    120,
  );
  const previousMs = parseIso(previousTs);
  const runIntervalSeconds =
    generatedAtMs > 0 && previousMs > 0 ? Math.max(0, Math.floor((generatedAtMs - previousMs) / 1000)) : Number.POSITIVE_INFINITY;
  const status = cleanText(monitor?.status || 'active', 60);
  const alertRows: Array<Record<string, unknown>> = [];
  if (!fs.existsSync(monitorAbs)) {
    alertRows.push({
      id: 'monitor_missing',
      severity: 'high',
      detail: args.monitorPath,
    });
  }
  if (!Number.isFinite(reportAgeSeconds) || reportAgeSeconds > args.maxAgeSeconds) {
    alertRows.push({
      id: 'monitor_stale',
      severity: 'high',
      detail: `age_seconds=${Number.isFinite(reportAgeSeconds) ? reportAgeSeconds : -1};threshold=${args.maxAgeSeconds}`,
    });
  }
  if (!Number.isFinite(runIntervalSeconds) || runIntervalSeconds > args.maxIntervalSeconds) {
    alertRows.push({
      id: 'monitor_run_interval_breach',
      severity: 'medium',
      detail: `interval_seconds=${Number.isFinite(runIntervalSeconds) ? runIntervalSeconds : -1};threshold=${args.maxIntervalSeconds}`,
    });
  }
  if (status !== 'active') {
    alertRows.push({
      id: 'monitor_not_active',
      severity: 'high',
      detail: `status=${status}`,
    });
  }
  const monitorType = cleanText(monitor?.type || '', 120);
  const monitorSchemaVersion = Number(monitor?.schema_version || 0);
  const monitorOkIsBoolean = typeof monitor?.ok === 'boolean';
  const monitorChecks = Array.isArray(monitor?.checks) ? monitor.checks : [];
  const monitorCheckIds = monitorChecks
    .map((row: any) => cleanText(row?.id || '', 160))
    .filter(Boolean);
  const monitorCheckIdDuplicates = Array.from(new Set(duplicateValues(monitorCheckIds)));
  const monitorCheckIdNoncanonical = monitorCheckIds.filter((id) => !CANONICAL_TOKEN.test(id));
  const summary = monitor?.summary && typeof monitor.summary === 'object' ? monitor.summary : null;
  const issueCounts = summary?.issue_counts && typeof summary.issue_counts === 'object'
    ? (summary.issue_counts as Record<string, unknown>)
    : null;
  const troubleshootingLatestEval =
    summary?.troubleshooting_latest_eval && typeof summary.troubleshooting_latest_eval === 'object'
      ? summary.troubleshooting_latest_eval
      : null;
  const troubleshootingTs = cleanText(troubleshootingLatestEval?.ts || '', 120);
  const troubleshootingPreviousTs = cleanText(troubleshootingLatestEval?.previous_ts || '', 120);
  const troubleshootingTsValid = parseIso(troubleshootingTs) > 0;
  const troubleshootingPreviousTsValid =
    troubleshootingPreviousTs.length === 0 || parseIso(troubleshootingPreviousTs) > 0;
  const requiredIssueCountKeys = [
    'wrong_tool_selection_count',
    'no_response_count',
    'file_tool_route_misdirection_count',
    'repeated_response_loop_count',
  ];
  const issueCountsMissingKeys = requiredIssueCountKeys.filter(
    (key) => !issueCounts || !Object.prototype.hasOwnProperty.call(issueCounts, key),
  );
  const sources = monitor?.sources && typeof monitor.sources === 'object'
    ? (monitor.sources as Record<string, unknown>)
    : null;
  const sourceQueuePath = cleanText((sources?.queue as string) || '', 260);
  const sourceTroubleshootingPath = cleanText((sources?.troubleshooting_latest as string) || '', 260);
  const sourceHistoryPath = cleanText((sources?.history as string) || '', 260);
  const outputPathsDistinct =
    new Set([args.outPath, args.outLatestPath, args.markdownPath, args.monitorPath]).size === 4;

  const checks = [
    {
      id: 'eval_monitor_slo_monitor_path_canonical_contract',
      ok: isCanonicalRelativePath(args.monitorPath, 'local/state/ops/eval_agent_chat_monitor/'),
      detail: args.monitorPath,
    },
    {
      id: 'eval_monitor_slo_out_path_canonical_contract',
      ok: isCanonicalRelativePath(args.outPath, 'core/local/artifacts/'),
      detail: args.outPath,
    },
    {
      id: 'eval_monitor_slo_out_path_current_suffix_contract',
      ok: cleanText(args.outPath, 500).endsWith('_current.json'),
      detail: args.outPath,
    },
    {
      id: 'eval_monitor_slo_out_latest_path_canonical_contract',
      ok: isCanonicalRelativePath(args.outLatestPath, 'artifacts/'),
      detail: args.outLatestPath,
    },
    {
      id: 'eval_monitor_slo_out_latest_path_latest_suffix_contract',
      ok: cleanText(args.outLatestPath, 500).endsWith('_latest.json'),
      detail: args.outLatestPath,
    },
    {
      id: 'eval_monitor_slo_markdown_path_canonical_contract',
      ok: isCanonicalRelativePath(args.markdownPath, 'local/workspace/reports/'),
      detail: args.markdownPath,
    },
    {
      id: 'eval_monitor_slo_markdown_path_contract',
      ok: cleanText(args.markdownPath, 500) === DEFAULT_MARKDOWN_PATH,
      detail: args.markdownPath,
    },
    {
      id: 'eval_monitor_slo_output_paths_distinct_contract',
      ok: outputPathsDistinct,
      detail: `${args.outPath}|${args.outLatestPath}|${args.markdownPath}|${args.monitorPath}`,
    },
    {
      id: 'eval_monitor_slo_monitor_type_contract',
      ok: monitorType === 'eval_agent_chat_monitor_guard',
      detail: monitorType || 'missing',
    },
    {
      id: 'eval_monitor_slo_monitor_schema_version_contract',
      ok: monitorSchemaVersion === 1,
      detail: String(monitorSchemaVersion),
    },
    {
      id: 'eval_monitor_slo_monitor_generated_at_iso_contract',
      ok: generatedAtMs > 0,
      detail: generatedAt || 'missing',
    },
    {
      id: 'eval_monitor_slo_monitor_ok_boolean_contract',
      ok: monitorOkIsBoolean,
      detail: String(monitor?.ok ?? 'missing'),
    },
    {
      id: 'eval_monitor_slo_monitor_checks_array_nonempty_contract',
      ok: Array.isArray(monitor?.checks) && monitorChecks.length > 0,
      detail: `checks=${monitorChecks.length}`,
    },
    {
      id: 'eval_monitor_slo_monitor_check_ids_unique_contract',
      ok: monitorCheckIdDuplicates.length === 0,
      detail: monitorCheckIdDuplicates.join(',') || 'none',
    },
    {
      id: 'eval_monitor_slo_monitor_check_ids_canonical_contract',
      ok: monitorCheckIdNoncanonical.length === 0,
      detail: monitorCheckIdNoncanonical.join(',') || 'none',
    },
    {
      id: 'eval_monitor_slo_summary_object_contract',
      ok: !!summary,
      detail: summary ? 'present' : 'missing',
    },
    {
      id: 'eval_monitor_slo_summary_troubleshooting_ts_contract',
      ok: troubleshootingTsValid,
      detail: troubleshootingTs || 'missing',
    },
    {
      id: 'eval_monitor_slo_summary_troubleshooting_previous_ts_contract',
      ok: troubleshootingPreviousTsValid,
      detail: troubleshootingPreviousTs || 'none',
    },
    {
      id: 'eval_monitor_slo_summary_issue_counts_keys_contract',
      ok: issueCountsMissingKeys.length === 0,
      detail: issueCountsMissingKeys.join(',') || 'none',
    },
    {
      id: 'eval_monitor_slo_sources_paths_contract',
      ok:
        isCanonicalRelativePath(sourceQueuePath, 'local/state/')
        && isCanonicalRelativePath(sourceTroubleshootingPath, 'client/runtime/local/state/')
        && isCanonicalRelativePath(sourceHistoryPath, 'local/state/ops/eval_agent_chat_monitor/'),
      detail:
        `${sourceQueuePath || 'missing'}|${sourceTroubleshootingPath || 'missing'}|${sourceHistoryPath || 'missing'}`,
    },
    { id: 'monitor_present', ok: fs.existsSync(monitorAbs), detail: args.monitorPath },
    {
      id: 'monitor_freshness_slo_contract',
      ok: Number.isFinite(reportAgeSeconds) && reportAgeSeconds <= args.maxAgeSeconds,
      detail: `age_seconds=${Number.isFinite(reportAgeSeconds) ? reportAgeSeconds : -1};threshold=${args.maxAgeSeconds}`,
    },
    {
      id: 'monitor_interval_slo_contract',
      ok: Number.isFinite(runIntervalSeconds) && runIntervalSeconds <= args.maxIntervalSeconds,
      detail: `interval_seconds=${Number.isFinite(runIntervalSeconds) ? runIntervalSeconds : -1};threshold=${args.maxIntervalSeconds}`,
    },
    {
      id: 'monitor_status_contract',
      ok: status === 'active',
      detail: `status=${status}`,
    },
  ];

  const report = {
    type: 'eval_monitor_slo_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    metrics: {
      status,
      report_age_seconds: Number.isFinite(reportAgeSeconds) ? reportAgeSeconds : -1,
      run_interval_seconds: Number.isFinite(runIntervalSeconds) ? runIntervalSeconds : -1,
      thresholds: {
        max_age_seconds: args.maxAgeSeconds,
        max_interval_seconds: args.maxIntervalSeconds,
      },
    },
    summary: {
      alert_count: alertRows.length,
    },
    alerts: alertRows,
    sources: {
      monitor: args.monitorPath,
    },
  };

  writeJsonArtifact(outLatestAbs, report);
  writeTextArtifact(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));
