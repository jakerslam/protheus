#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY = 'client/runtime/config/orchestration_ts_boundary_policy.json';
const DEFAULT_OUT = 'core/local/artifacts/orchestration_ts_boundary_audit_current.json';

type Policy = {
  surface_root?: string;
  script_root?: string;
  min_rust_pct?: number;
  max_nonempty_lines_per_script?: number;
  required_markers?: string[];
  forbidden_markers?: string[];
  script_constraint_exempt_prefixes?: string[];
  rust_ratio_exempt_prefixes?: string[];
};

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function walk(base: string, ext: string, out: string[] = []): string[] {
  if (!fs.existsSync(base)) return out;
  for (const entry of fs.readdirSync(base, { withFileTypes: true })) {
    const abs = path.join(base, entry.name);
    if (entry.isDirectory()) {
      walk(abs, ext, out);
      continue;
    }
    if (entry.isFile() && abs.endsWith(ext)) out.push(abs);
  }
  return out;
}

function countLines(filePath: string): number {
  const source = fs.readFileSync(filePath, 'utf8');
  if (!source) return 0;
  return source.split(/\r?\n/).length;
}

function countNonEmptyLines(source: string): number {
  return source
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean).length;
}

function normalizePolicyPrefix(value: string): string {
  const normalized = cleanText(value, 400).replace(/\\/g, '/');
  if (!normalized) return '';
  return normalized.endsWith('/') ? normalized : `${normalized}/`;
}

function isPathUnderAnyPrefix(relPath: string, prefixes: string[]): boolean {
  const normalizedPath = cleanText(relPath, 600).replace(/\\/g, '/');
  if (!normalizedPath) return false;
  return prefixes.some((prefix) =>
    normalizedPath === prefix.slice(0, -1) || normalizedPath.startsWith(prefix),
  );
}

function main() {
  const argv = process.argv.slice(2);
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT });
  const args = {
    out: cleanText(common.out || DEFAULT_OUT, 400),
    strict: common.strict,
    policy: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
  };
  const policyPath = path.resolve(ROOT, args.policy);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Policy;
  const surfaceRoot = path.resolve(ROOT, cleanText(policy.surface_root || 'surface/orchestration', 400));
  const scriptRoot = path.resolve(ROOT, cleanText(policy.script_root || 'surface/orchestration/scripts', 400));
  const requiredMarkers = Array.isArray(policy.required_markers)
    ? policy.required_markers.map((value) => String(value))
    : [];
  const forbiddenMarkers = Array.isArray(policy.forbidden_markers)
    ? policy.forbidden_markers.map((value) => String(value))
    : [];
  const minRustPct = Number(policy.min_rust_pct || 95);
  const maxNonEmptyLinesPerScript = Number(policy.max_nonempty_lines_per_script || 4);
  const scriptConstraintExemptPrefixes = Array.isArray(policy.script_constraint_exempt_prefixes)
    ? policy.script_constraint_exempt_prefixes
      .map((value) => normalizePolicyPrefix(String(value)))
      .filter(Boolean)
    : [];
  const rustRatioExemptPrefixes = Array.isArray(policy.rust_ratio_exempt_prefixes)
    ? policy.rust_ratio_exempt_prefixes
      .map((value) => normalizePolicyPrefix(String(value)))
      .filter(Boolean)
    : [];

  const rustFiles = walk(surfaceRoot, '.rs');
  const tsFiles = walk(surfaceRoot, '.ts');
  const scriptFiles = walk(scriptRoot, '.ts');
  const rustFilesForRatio = rustFiles.filter(
    (filePath) => !isPathUnderAnyPrefix(rel(filePath), rustRatioExemptPrefixes),
  );
  const tsFilesForRatio = tsFiles.filter(
    (filePath) => !isPathUnderAnyPrefix(rel(filePath), rustRatioExemptPrefixes),
  );
  const rustLines = rustFilesForRatio.reduce((sum, filePath) => sum + countLines(filePath), 0);
  const tsLines = tsFilesForRatio.reduce((sum, filePath) => sum + countLines(filePath), 0);
  const rustPct = rustLines + tsLines === 0 ? 100 : (rustLines * 100) / (rustLines + tsLines);
  const scriptRootRel = rel(scriptRoot);
  const violations: Array<Record<string, unknown>> = [];

  for (const tsFile of tsFiles) {
    const rp = rel(tsFile);
    if (!rp.startsWith(scriptRootRel)) {
      violations.push({
        file: rp,
        reason: 'ts_outside_orchestration_script_root',
      });
    }
  }

  for (const scriptFile of scriptFiles) {
    const rp = rel(scriptFile);
    if (isPathUnderAnyPrefix(rp, scriptConstraintExemptPrefixes)) {
      continue;
    }
    const source = fs.readFileSync(scriptFile, 'utf8');
    const nonEmptyLines = countNonEmptyLines(source);
    if (nonEmptyLines > maxNonEmptyLinesPerScript) {
      violations.push({
        file: rp,
        reason: 'orchestration_script_line_budget_exceeded',
        detail: `${nonEmptyLines} > ${maxNonEmptyLinesPerScript}`,
      });
    }
    for (const marker of requiredMarkers) {
      if (!source.includes(marker)) {
        violations.push({
          file: rp,
          reason: 'missing_required_marker',
          detail: marker,
        });
      }
    }
    for (const marker of forbiddenMarkers) {
      if (source.includes(marker)) {
        violations.push({
          file: rp,
          reason: 'forbidden_marker_present',
          detail: marker,
        });
      }
    }
  }

  if (rustPct < minRustPct) {
    violations.push({
      file: rel(surfaceRoot),
      reason: 'orchestration_rust_ratio_below_policy',
      detail: `${rustPct.toFixed(2)} < ${minRustPct.toFixed(2)}`,
    });
  }

  const payload = {
    ok: violations.length === 0,
    type: 'orchestration_ts_boundary_audit',
    generated_at: new Date().toISOString(),
    owner: 'ops',
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      policy: rel(policyPath),
      out: args.out,
    },
    summary: {
      rust_file_count: rustFilesForRatio.length,
      ts_file_count: tsFilesForRatio.length,
      script_file_count: scriptFiles.length,
      rust_lines: rustLines,
      ts_lines: tsLines,
      rust_pct: Number(rustPct.toFixed(2)),
      min_rust_pct: minRustPct,
      max_nonempty_lines_per_script: maxNonEmptyLinesPerScript,
      script_constraint_exempt_prefixes: scriptConstraintExemptPrefixes,
      rust_ratio_exempt_prefixes: rustRatioExemptPrefixes,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    artifact_paths: [args.out],
    violations,
  };

  process.exit(
    emitStructuredResult(payload, {
      outPath: args.out,
      strict: args.strict,
      ok: payload.ok,
    }),
  );
}

main();
