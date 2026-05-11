#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: workspace_ops/git-safety

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const root = process.cwd();
function git(args, envExtra = {}) {
  const run = spawnSync('git', args, {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 32 * 1024 * 1024,
    env: { ...process.env, ...envExtra },
  });
  return { status: run.status ?? 1, stdout: run.stdout || '', stderr: run.stderr || '' };
}
function argValues(name) {
  const prefix = `${name}=`;
  const out = [];
  for (let idx = 2; idx < process.argv.length; idx += 1) {
    const arg = process.argv[idx] || '';
    if (arg === name && process.argv[idx + 1]) out.push(process.argv[++idx]);
    else if (arg.startsWith(prefix)) out.push(arg.slice(prefix.length));
  }
  return out;
}
function normalizeSelectedPath(raw) {
  const value = String(raw || '').trim();
  if (!value) return null;
  const absolute = path.isAbsolute(value) ? value : path.join(root, value);
  const rel = path.relative(root, absolute).replaceAll(path.sep, '/');
  if (!rel || rel === '.' || rel.startsWith('../') || rel === '..') return null;
  if (rel === '.git' || rel.startsWith('.git/')) return null;
  return rel;
}
function selectedPaths() {
  const raw = [
    ...argValues('--path'),
    ...argValues('--paths').flatMap((value) => value.split(',').map((part) => part.trim())),
  ];
  return Array.from(new Set(raw.map(normalizeSelectedPath).filter(Boolean))).sort();
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
function pathMatches(file, selected) {
  const normalized = selected.replace(/\/+$/, '');
  return file === normalized || file.startsWith(`${normalized}/`);
}
function rowsForSelected(rows, selected) {
  if (!selected.length) return [];
  return rows.filter((row) => selected.some((item) => pathMatches(row.file, item)));
}
function temporaryIndexPlan(selected) {
  if (!selected.length) {
    return {
      attempted: false,
      ok: false,
      reason: 'no_explicit_paths_selected',
      dry_run_only: true,
      real_index_mutated: false,
      temporary_index_used: false,
    };
  }
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-safe-commit-'));
  const tempIndexPath = path.join(tmpDir, 'index');
  const env = { GIT_INDEX_FILE: tempIndexPath };
  const steps = [];
  function step(id, args) {
    const result = git(args, env);
    steps.push({
      id,
      status: result.status,
      stderr: result.stderr.trim().slice(0, 4000),
      stdout: result.stdout.trim().slice(0, 4000),
    });
    return result;
  }
  try {
    const readTree = step('temporary_index_read_tree_head', ['read-tree', 'HEAD']);
    if (readTree.status !== 0) {
      return {
        attempted: true,
        ok: false,
        reason: 'temporary_index_read_tree_failed',
        dry_run_only: true,
        real_index_mutated: false,
        temporary_index_used: true,
        selected_paths: selected,
        steps,
      };
    }
    const add = step('temporary_index_stage_selected_paths', ['add', '--', ...selected]);
    const diffCheck = step('temporary_index_diff_check', ['diff', '--cached', '--check']);
    const diffNameStatus = step('temporary_index_name_status', ['diff', '--cached', '--name-status']);
    const writeTree = step('temporary_index_write_tree', ['write-tree']);
    const ok = add.status === 0 && diffCheck.status === 0 && writeTree.status === 0;
    return {
      attempted: true,
      ok,
      reason: ok ? 'temporary_index_commit_candidate_valid' : 'temporary_index_commit_candidate_invalid',
      dry_run_only: true,
      real_index_mutated: false,
      temporary_index_used: true,
      selected_paths: selected,
      candidate_name_status: diffNameStatus.stdout
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter(Boolean),
      candidate_tree: writeTree.status === 0 ? writeTree.stdout.trim() : null,
      steps,
    };
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
}
function main() {
  const outFlag = process.argv.find((arg) => arg.startsWith('--out='));
  const status = git(['status', '--porcelain=v1', '-z']);
  const rows = parseStatusZ(status.stdout);
  const cls = classify(rows);
  const selected = selectedPaths();
  const selectedRows = rowsForSelected(rows, selected);
  const tempPlan = temporaryIndexPlan(selected);
  const workspaceUnsafeReasons = [];
  const selectedUnmerged = selectedRows.filter((row) => cls.unmerged.some((candidate) => candidate.file === row.file));
  const candidateBlockers = [];
  if (cls.unmerged.length) workspaceUnsafeReasons.push('unmerged_paths_present');
  if (cls.shadowedDeletes.length) workspaceUnsafeReasons.push('tracked_delete_shadowed_by_untracked_same_path');
  if (rows.length > 100) workspaceUnsafeReasons.push('large_dirty_surface_requires_surgical_commit');
  if (selected.length === 0) candidateBlockers.push('explicit_paths_required_for_surgical_commit_plan');
  if (selectedUnmerged.length > 0) candidateBlockers.push('selected_paths_include_unmerged_paths');
  if (tempPlan.attempted && !tempPlan.ok) candidateBlockers.push('temporary_index_candidate_failed');
  const ok = selected.length > 0 ? candidateBlockers.length === 0 && tempPlan.ok : workspaceUnsafeReasons.length === 0;
  const traceId = `workspace_ops:${new Date().toISOString()}:${process.pid}`;
  const payload = {
    trace_id: traceId,
    span_id: `span:${traceId}`,
    parent_span_id: null,
    source_domain: 'workspace_ops',
    ok,
    type: 'safe_commit_workspace_report',
    generated_at: new Date().toISOString(),
    git_status_exit_code: status.status,
    mode: selected.length ? 'surgical_commit_dry_run' : 'workspace_safety_report',
    dry_run_only: true,
    real_index_mutated: false,
    summary: {
      total_rows: rows.length,
      staged: cls.staged.length,
      unstaged: cls.unstaged.length,
      untracked: cls.untracked.length,
      unmerged: cls.unmerged.length,
      staged_deletes: cls.stagedDeletes.length,
      shadowed_deletes: cls.shadowedDeletes.length,
      unsafe_reasons: [...workspaceUnsafeReasons, ...candidateBlockers],
      workspace_unsafe_reasons: workspaceUnsafeReasons,
      candidate_blockers: candidateBlockers,
      normal_commit_safe: workspaceUnsafeReasons.length === 0,
      surgical_commit_recommended: workspaceUnsafeReasons.length > 0
    },
    surgical_commit_plan: {
      selected_paths: selected,
      selected_row_count: selectedRows.length,
      selected_unmerged_count: selectedUnmerged.length,
      unselected_dirty_count: Math.max(0, rows.length - selectedRows.length),
      selected_by_domain: countBy(selectedRows, (row) => summarizeDomain(row.file)),
      temporary_index: tempPlan,
      apply_policy:
        'This tool never mutates the real index. If the temporary-index plan passes, an operator may stage exactly selected_paths manually or through a separate approved commit command.'
    },
    by_domain: countBy(rows, (row) => summarizeDomain(row.file)),
    unmerged_paths: cls.unmerged.map((row) => row.file).slice(0, 100),
    shadowed_delete_paths: cls.shadowedDeletes.map((row) => row.file).slice(0, 100),
    sample_rows: rows.slice(0, 120),
    recommendation:
      selected.length && candidateBlockers.length === 0 && tempPlan.ok
        ? 'Temporary-index validation passed for selected_paths. Keep the real commit surgical and stage only those paths.'
        : workspaceUnsafeReasons.length || candidateBlockers.length
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
