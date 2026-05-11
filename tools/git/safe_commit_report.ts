#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: workspace_ops/git-safety

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const root = process.cwd();
function git(args) {
  const run = spawnSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  return { status: run.status ?? 1, stdout: run.stdout || '', stderr: run.stderr || '' };
}
function parseStatusZ(raw) {
  const parts = raw.split('\0').filter(Boolean);
  const rows = [];
  for (let i = 0; i < parts.length; i += 1) {
    const entry = parts[i];
    const xy = entry.slice(0, 2);
    const file = entry.slice(3);
    let source = '';
    if (xy.startsWith('R') || xy.startsWith('C')) source = parts[++i] || '';
    rows.push({ xy, index: xy[0], worktree: xy[1], file, source });
  }
  return rows;
}
function classify(rows) {
  const byFile = new Map(rows.map((row) => [row.file, row]));
  const unmerged = rows.filter((row) => /[U]/.test(row.xy) || ['AA', 'DD', 'AU', 'UA', 'DU', 'UD'].includes(row.xy));
  const staged = rows.filter((row) => row.index !== ' ' && row.index !== '?' && !unmerged.includes(row));
  const unstaged = rows.filter((row) => row.worktree !== ' ' && row.index !== '?' && !unmerged.includes(row));
  const untracked = rows.filter((row) => row.index === '?' || row.xy === '??');
  const stagedDeletes = rows.filter((row) => row.index === 'D');
  const shadowedDeletes = stagedDeletes.filter((row) => byFile.has(row.file) && untracked.some((candidate) => candidate.file === row.file));
  const deletedOrShadowedCommittedPaths = shadowedDeletes.map((row) => row.file);
  return { unmerged, staged, unstaged, untracked, stagedDeletes, shadowedDeletes, deletedOrShadowedCommittedPaths };
}
function summarizeDomain(file) {
  if (file.startsWith('client/') || file.startsWith('shell/')) return 'shell';
  if (file.startsWith('orchestration/') || file.startsWith('surface/orchestration/')) return 'orchestration';
  if (file.startsWith('core/')) return 'kernel_or_core';
  if (file.startsWith('observability/')) return 'observability';
  if (file.startsWith('validation/')) return 'validation';
  if (file.startsWith('tests/tooling/')) return 'validation_harness';
  if (file.startsWith('install')) return 'installer';
  if (file.startsWith('.github/')) return 'ci';
  if (file.startsWith('tools/')) return 'workspace_ops';
  return 'other';
}
function countBy(rows, fn) {
  return rows.reduce((acc, row) => {
    const key = fn(row);
    acc[key] = (acc[key] || 0) + 1;
    return acc;
  }, {});
}
function main() {
  const outFlag = process.argv.find((arg) => arg.startsWith('--out='));
  const status = git(['status', '--porcelain=v1', '-z']);
  const rows = parseStatusZ(status.stdout);
  const cls = classify(rows);
  const unsafeReasons = [];
  if (cls.unmerged.length) unsafeReasons.push('unmerged_paths_present');
  if (cls.shadowedDeletes.length) unsafeReasons.push('tracked_delete_shadowed_by_untracked_same_path');
  if (rows.length > 100) unsafeReasons.push('large_dirty_surface_requires_surgical_commit');
  const traceId = `workspace_ops:${new Date().toISOString()}:${process.pid}`;
  const payload = {
    trace_id: traceId,
    span_id: `span:${traceId}`,
    parent_span_id: null,
    source_domain: 'workspace_ops',
    ok: unsafeReasons.length === 0,
    type: 'safe_commit_workspace_report',
    generated_at: new Date().toISOString(),
    git_status_exit_code: status.status,
    summary: {
      total_rows: rows.length,
      staged: cls.staged.length,
      unstaged: cls.unstaged.length,
      untracked: cls.untracked.length,
      unmerged: cls.unmerged.length,
      staged_deletes: cls.stagedDeletes.length,
      shadowed_deletes: cls.shadowedDeletes.length,
      unsafe_reasons: unsafeReasons,
      normal_commit_safe: unsafeReasons.length === 0,
      surgical_commit_recommended: unsafeReasons.length > 0
    },
    by_domain: countBy(rows, (row) => summarizeDomain(row.file)),
    unmerged_paths: cls.unmerged.map((row) => row.file).slice(0, 100),
    shadowed_delete_paths: cls.shadowedDeletes.map((row) => row.file).slice(0, 100),
    sample_rows: rows.slice(0, 120),
    recommendation: unsafeReasons.length
      ? 'Do not use ordinary staging/commit flow. Use an isolated temporary index or clean/stash unrelated work first.'
      : 'Normal commit flow is acceptable.'
  };
  const outPath = outFlag ? outFlag.slice('--out='.length) : 'validation/reports/safe_commit_workspace_report_2026-05-10.json';
  fs.mkdirSync(path.dirname(path.join(root, outPath)), { recursive: true });
  fs.writeFileSync(path.join(root, outPath), `${JSON.stringify(payload, null, 2)}\n`);
  console.log(JSON.stringify(payload, null, 2));
  process.exit(0);
}
main();
