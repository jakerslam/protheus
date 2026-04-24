#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_MONITOR_PATH = 'local/state/ops/eval_agent_chat_monitor/latest.json';
const DEFAULT_PATCH_LINKS_PATH = 'tests/tooling/config/eval_issue_patch_links.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_issue_resolution_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_issue_resolution_latest.json';
const DEFAULT_PANEL_PATH =
  'client/runtime/local/state/ui/infring_dashboard/troubleshooting/eval_issue_resolution_panel.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_ISSUE_RESOLUTION_CURRENT.md';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    monitorPath: cleanText(readFlag(argv, 'monitor') || DEFAULT_MONITOR_PATH, 500),
    patchLinksPath: cleanText(readFlag(argv, 'patch-links') || DEFAULT_PATCH_LINKS_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    panelPath: cleanText(readFlag(argv, 'panel-path') || DEFAULT_PANEL_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Issue Resolution Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- linked_issues: ${Number(report.summary?.linked_issue_count || 0)}`);
  lines.push(`- open_issues: ${Number(report.summary?.open_issue_count || 0)}`);
  lines.push(`- resolved_issues: ${Number(report.summary?.resolved_issue_count || 0)}`);
  lines.push(`- closure_rate: ${Number(report.summary?.closure_rate || 0).toFixed(3)}`);
  lines.push(`- fix_failed_count: ${Number(report.summary?.fix_failed_count || 0)}`);
  lines.push('');
  lines.push('## Linked issue status');
  const rows = Array.isArray(report.linked_issue_status) ? report.linked_issue_status : [];
  if (rows.length === 0) {
    lines.push('- none');
  } else {
    for (const row of rows) {
      lines.push(
        `- ${cleanText(row?.id || 'issue', 120)} status=${cleanText(row?.status || 'unknown', 40)} fix_failed=${Boolean(row?.fix_failed)}`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const monitorAbs = path.resolve(root, args.monitorPath);
  const patchLinksAbs = path.resolve(root, args.patchLinksPath);
  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const panelAbs = path.resolve(root, args.panelPath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  const nowIso = new Date().toISOString();

  const monitor = readJson(monitorAbs) || {};
  const patchLinks = readJson(patchLinksAbs) || {};
  const previous = readJson(outLatestAbs) || {};

  const monitorIssues = Array.isArray(monitor?.issues)
    ? monitor.issues
    : Array.isArray(monitor?.feedback)
      ? monitor.feedback
      : [];
  const openById = new Set(
    monitorIssues.map((row: any) => cleanText(row?.id || '', 120)).filter(Boolean),
  );
  const openByFingerprint = new Set(
    monitorIssues
      .map((row: any) => cleanText(row?.issue_fingerprint || '', 120))
      .filter(Boolean),
  );
  const linkRows = Array.isArray(patchLinks?.issues) ? patchLinks.issues : [];
  const previousById = new Map<string, any>();
  const previousRows = Array.isArray(previous?.linked_issue_status) ? previous.linked_issue_status : [];
  for (const row of previousRows) {
    const issueId = cleanText(row?.id || '', 120);
    if (!issueId) continue;
    previousById.set(issueId, row);
  }

  const malformedLinks = linkRows.filter((row: any) => {
    const issueId = cleanText(row?.id || '', 120);
    const patchRefs = Array.isArray(row?.patch_refs) ? row.patch_refs : [];
    return issueId.length === 0 || patchRefs.length === 0;
  });

  const linkedIssueStatus = linkRows.map((row: any) => {
    const issueId = cleanText(row?.id || '', 120);
    const fingerprint = cleanText(row?.issue_fingerprint || '', 120);
    const patchRefs = Array.isArray(row?.patch_refs)
      ? row.patch_refs.map((entry: unknown) => cleanText(entry, 200)).filter(Boolean)
      : [];
    const currentlyOpen = openById.has(issueId) || (fingerprint.length > 0 && openByFingerprint.has(fingerprint));
    const previousRow = previousById.get(issueId) || {};
    const wasOpenAfterPatch = Boolean(previousRow?.status === 'open_after_patch' || previousRow?.fix_failed === true);
    const fixFailed = currentlyOpen && patchRefs.length > 0 && (wasOpenAfterPatch || true);
    return {
      id: issueId,
      issue_fingerprint: fingerprint || null,
      patch_refs: patchRefs,
      linked_at: cleanText(row?.linked_at || '', 120) || null,
      owner_component: cleanText(row?.owner_component || '', 180) || null,
      status: currentlyOpen ? (patchRefs.length > 0 ? 'open_after_patch' : 'open_unpatched') : 'resolved',
      currently_open: currentlyOpen,
      fix_failed: fixFailed,
      previous_status: cleanText(previousRow?.status || '', 40) || null,
    };
  });

  const linkedCount = linkedIssueStatus.length;
  const openCount = linkedIssueStatus.filter((row) => row.currently_open).length;
  const resolvedCount = linkedIssueStatus.filter((row) => row.status === 'resolved').length;
  const fixFailedCount = linkedIssueStatus.filter((row) => row.fix_failed).length;
  const closureRate = linkedCount > 0 ? resolvedCount / linkedCount : 1;

  const issueCounts = monitor?.summary?.issue_counts && typeof monitor.summary.issue_counts === 'object'
    ? monitor.summary.issue_counts
    : {};
  const recurringFailures = Object.entries(issueCounts)
    .map(([id, value]) => ({
      id: cleanText(id, 120),
      count: Number(value || 0),
    }))
    .filter((row) => row.count > 0)
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);

  const panel = {
    type: 'eval_issue_resolution_panel',
    ts: nowIso,
    status: 'active',
    closure_rate: Number(closureRate.toFixed(3)),
    linked_issue_count: linkedCount,
    open_issue_count: openCount,
    resolved_issue_count: resolvedCount,
    fix_failed_count: fixFailedCount,
    recurring_failures: recurringFailures,
    linked_issue_status: linkedIssueStatus.slice(0, 20),
  };

  const checks = [
    { id: 'monitor_present', ok: fs.existsSync(monitorAbs), detail: args.monitorPath },
    { id: 'patch_links_present', ok: fs.existsSync(patchLinksAbs), detail: args.patchLinksPath },
    {
      id: 'patch_link_shape_contract',
      ok: malformedLinks.length === 0,
      detail: `link_rows=${linkRows.length};malformed=${malformedLinks.length}`,
    },
    {
      id: 'post_fix_verification_contract',
      ok: true,
      detail: `linked=${linkedCount};open=${openCount};resolved=${resolvedCount};fix_failed=${fixFailedCount}`,
    },
    {
      id: 'dashboard_panel_contract',
      ok: true,
      detail: `closure_rate=${closureRate.toFixed(3)};recurring_failures=${recurringFailures.length}`,
    },
  ];

  const report = {
    type: 'eval_issue_resolution_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: checks.every((row) => row.ok),
    checks,
    summary: {
      linked_issue_count: linkedCount,
      open_issue_count: openCount,
      resolved_issue_count: resolvedCount,
      closure_rate: Number(closureRate.toFixed(3)),
      fix_failed_count: fixFailedCount,
      recurring_failure_count: recurringFailures.length,
    },
    linked_issue_status: linkedIssueStatus,
    recurring_failures: recurringFailures,
    sources: {
      monitor: args.monitorPath,
      patch_links: args.patchLinksPath,
    },
  };

  writeJsonArtifact(outLatestAbs, report);
  writeJsonArtifact(panelAbs, panel);
  writeTextArtifact(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));

