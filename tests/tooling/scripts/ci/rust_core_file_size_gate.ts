#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision, trackedFiles } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const DEFAULTS = {
  strict: false,
  policyPath: 'docs/workspace/rust_core_file_size_policy.json',
  outJson: 'core/local/artifacts/rust_core_file_size_gate_current.json',
  outMarkdown: 'local/workspace/reports/RUST_CORE_FILE_SIZE_GATE_CURRENT.md',
};

// SRS: V12-SYS-HL-029

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, DEFAULTS);
  return {
    strict: common.strict,
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULTS.policyPath, 260),
    outJson: cleanText(readFlag(argv, 'out-json') || DEFAULTS.outJson, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULTS.outMarkdown, 400),
  };
}

function readJson(filePath: string) {
  return JSON.parse(readFileSync(resolve(filePath), 'utf8'));
}

function listRustCoreFiles() {
  return trackedFiles()
    .filter((file) => file.startsWith('core/'))
    .filter((file) => file.endsWith('.rs'))
    .filter((file) => !isTestPath(file))
    .sort((a, b) => a.localeCompare(b));
}

function isTestPath(filePath: string) {
  return (
    /(^|\/)tests\//.test(filePath) ||
    /\.test\.(t|j)sx?$/.test(filePath) ||
    /(^|\/)__tests__(\/|$)/.test(filePath)
  );
}

function lineCount(filePath: string) {
  const content = readFileSync(resolve(filePath), 'utf8');
  return content.split(/\r?\n/).length;
}

function isExpired(dateIso, now) {
  const ts = Date.parse(String(dateIso || '').trim());
  if (!Number.isFinite(ts)) return true;
  return ts < now.getTime();
}

function toMarkdown(payload) {
  const lines = [];
  lines.push('# Rust Core File Size Gate (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Policy: ${payload.policy_path}`);
  lines.push(`Pass: ${payload.summary.pass ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- total_files: ${payload.summary.total_files}`);
  lines.push(`- max_lines: ${payload.summary.max_lines}`);
  lines.push(`- oversize_files: ${payload.summary.oversize_files}`);
  lines.push(`- exempt_files: ${payload.summary.exempt_files}`);
  lines.push(`- split_required_exempt_files: ${payload.summary.split_required_exempt_files}`);
  lines.push(`- keep_contiguous_exempt_files: ${payload.summary.keep_contiguous_exempt_files}`);
  lines.push(`- newly_oversized_non_exempt_files: ${payload.summary.newly_oversized_non_exempt_files}`);
  lines.push(`- stale_exemptions: ${payload.summary.stale_exemptions}`);
  lines.push(`- metadata_failures: ${payload.summary.metadata_failures}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push(`- strict: ${payload.summary.strict}`);
  lines.push('');
  lines.push('## Checks');
  lines.push('| Check | Pass | Detail |');
  lines.push('| --- | --- | --- |');
  for (const row of payload.checks) {
    lines.push(`| ${row.id} | ${row.ok ? 'true' : 'false'} | ${row.detail} |`);
  }
  lines.push('');
  if (payload.violations.length) {
    lines.push('## Violations');
    lines.push('| Path | Lines | Code | Detail |');
    lines.push('| --- | ---: | --- | --- |');
    for (const row of payload.violations) {
      lines.push(`| ${row.path} | ${row.lines} | ${row.code} | ${row.detail} |`);
    }
    lines.push('');
  }
  lines.push('## Oversize Inventory');
  lines.push('| Path | Lines | Status | Disposition | Expires |');
  lines.push('| --- | ---: | --- | --- | --- |');
  for (const row of payload.oversize_inventory) {
    lines.push(
      `| ${row.path} | ${row.lines} | ${row.status} | ${row.disposition || ''} | ${row.expires || ''} |`,
    );
  }
  if (payload.stale_exemptions.length) {
    lines.push('');
    lines.push('## Stale Exemptions');
    lines.push('| Path | Code | Detail |');
    lines.push('| --- | --- | --- |');
    for (const row of payload.stale_exemptions) {
      lines.push(`| ${row.path} | ${row.code} | ${row.detail} |`);
    }
  }
  return `${lines.join('\n')}\n`;
}

function requiredMetadata(policy) {
  const configured = Array.isArray(policy?.exception_metadata_required)
    ? policy.exception_metadata_required
    : [];
  const rows = configured
    .map((row) => String(row || '').trim())
    .filter(Boolean);
  return rows.length
    ? rows
    : ['owner', 'reason', 'expires', 'task_id', 'planned_batch_date', 'disposition', 'rationale', 'split_strategy'];
}

function metadataFailure(exception, required) {
  const missing = required.filter((field) => !String(exception?.[field] || '').trim());
  if (missing.length > 0) {
    return `missing required exemption metadata: ${missing.join(',')}`;
  }
  const disposition = String(exception?.disposition || '').trim();
  if (!['split_required', 'keep_contiguous'].includes(disposition)) {
    return 'disposition must be split_required or keep_contiguous';
  }
  const expires = String(exception?.expires || '').trim();
  const planned = String(exception?.planned_batch_date || '').trim();
  if (planned && expires && Date.parse(planned) > Date.parse(expires)) {
    return `planned_batch_date=${planned} is after expires=${expires}`;
  }
  if (disposition === 'split_required' && !String(exception?.split_strategy || '').trim()) {
    return 'split_required exemptions must include split_strategy';
  }
  if (disposition === 'keep_contiguous' && !String(exception?.rationale || '').includes('contiguous')) {
    return 'keep_contiguous exemptions must include a contiguous correctness rationale';
  }
  return null;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const now = new Date();
  const policy = readJson(args.policyPath);
  const maxLines = Number(policy?.max_lines || 500);
  const exceptionRows = Array.isArray(policy?.exceptions) ? policy.exceptions : [];
  const metadataFields = requiredMetadata(policy);
  const exceptionMap = new Map();
  for (const row of exceptionRows) {
    const path = String(row?.path || '').trim();
    if (!path) continue;
    exceptionMap.set(path, row);
  }

  const files = listRustCoreFiles();
  const oversizeInventory = [];
  const violations = [];
  const observedOversizePaths = new Set();
  let metadataFailures = 0;
  let exemptCount = 0;

  for (const path of files) {
    const lines = lineCount(path);
    if (lines <= maxLines) continue;
    observedOversizePaths.add(path);
    const exception = exceptionMap.get(path) || null;
    if (!exception) {
      oversizeInventory.push({ path, lines, status: 'violation_unlisted', expires: null });
      violations.push({
        path,
        lines,
        code: 'oversize_unlisted',
        detail: `file exceeds ${maxLines} lines without an exception entry`,
      });
      continue;
    }

    const expires = String(exception.expires || '').trim();
    const disposition = String(exception.disposition || '').trim();
    const metadataError = metadataFailure(exception, metadataFields);
    if (metadataError) {
      metadataFailures += 1;
      oversizeInventory.push({
        path,
        lines,
        status: 'violation_metadata',
        disposition,
        expires: expires || null,
      });
      violations.push({
        path,
        lines,
        code: 'exception_metadata_missing',
        detail: metadataError,
      });
      continue;
    }

    if (isExpired(expires, now)) {
      oversizeInventory.push({ path, lines, status: 'violation_expired', disposition, expires });
      violations.push({
        path,
        lines,
        code: 'exception_expired',
        detail: `exception expired on ${expires}`,
      });
      continue;
    }

    oversizeInventory.push({ path, lines, status: 'exempt', disposition, expires });
    exemptCount += 1;
  }

  const staleExemptions = [];
  for (const row of exceptionRows) {
    const exceptionPath = String(row?.path || '').trim();
    if (!exceptionPath || observedOversizePaths.has(exceptionPath)) continue;
    const stale = {
      path: exceptionPath,
      lines: 0,
      code: 'exception_stale_or_under_cap',
      detail: 'exception no longer maps to an oversized tracked core Rust file',
    };
    staleExemptions.push(stale);
    violations.push(stale);
  }

  oversizeInventory.sort((a, b) => b.lines - a.lines || a.path.localeCompare(b.path));
  const newlyOversizedNonExempt = violations.filter((row) => row.code === 'oversize_unlisted');
  const splitRequiredExemptions = oversizeInventory.filter(
    (row) => row.status === 'exempt' && row.disposition === 'split_required',
  );
  const keepContiguousExemptions = oversizeInventory.filter(
    (row) => row.status === 'exempt' && row.disposition === 'keep_contiguous',
  );
  const largestOversized = oversizeInventory[0] || null;
  const checks = [
    {
      id: 'kernel_core_size_no_newly_oversized_non_exempt_files',
      ok: newlyOversizedNonExempt.length === 0,
      detail: `newly_oversized_non_exempt_files=${newlyOversizedNonExempt.length}`,
    },
    {
      id: 'kernel_core_size_all_oversized_files_are_tracked',
      ok: oversizeInventory.length === exemptCount,
      detail: `oversize_files=${oversizeInventory.length};tracked_exempt_files=${exemptCount}`,
    },
    {
      id: 'kernel_core_size_no_stale_or_under_cap_exemptions',
      ok: staleExemptions.length === 0,
      detail: `stale_exemptions=${staleExemptions.length}`,
    },
    {
      id: 'kernel_core_size_split_required_debt_is_explicit',
      ok: splitRequiredExemptions.length + keepContiguousExemptions.length === exemptCount,
      detail: `split_required=${splitRequiredExemptions.length};keep_contiguous=${keepContiguousExemptions.length}`,
    },
  ];
  const ok = violations.length === 0 && checks.every((check) => check.ok);

  const payload = {
    ok,
    type: 'rust_core_file_size_gate',
    generated_at: now.toISOString(),
    revision: currentRevision(),
    policy_path: args.policyPath,
    summary: {
      strict: args.strict,
      pass: ok,
      total_files: files.length,
      max_lines: maxLines,
      oversize_files: oversizeInventory.length,
      exempt_files: exemptCount,
      split_required_exempt_files: splitRequiredExemptions.length,
      keep_contiguous_exempt_files: keepContiguousExemptions.length,
      newly_oversized_non_exempt_files: newlyOversizedNonExempt.length,
      largest_oversize_path: largestOversized?.path || null,
      largest_oversize_lines: largestOversized?.lines || 0,
      stale_exemptions: staleExemptions.length,
      metadata_failures: metadataFailures,
      violations: violations.length,
    },
    artifact_paths: [args.outJson, args.outMarkdown],
    checks,
    violations,
    stale_exemptions: staleExemptions,
    oversize_inventory: oversizeInventory,
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  process.exitCode = emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

main();
