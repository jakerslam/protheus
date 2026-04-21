#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type ImportBoundaryRule = {
  id?: string;
  scan_roots?: string[];
  extensions?: string[];
  forbidden_import_patterns?: string[];
  allow_import_patterns?: string[];
};

type SymbolBoundaryRule = {
  id?: string;
  scan_roots?: string[];
  extensions?: string[];
  forbidden_symbol_patterns?: string[];
  allow_file_patterns?: string[];
};

type PathBoundaryRule = {
  id?: string;
  scan_roots?: string[];
  extensions?: string[];
  forbidden_path_patterns?: string[];
  allow_path_patterns?: string[];
};

type Policy = {
  version?: string;
  import_boundaries?: ImportBoundaryRule[];
  symbol_boundaries?: SymbolBoundaryRule[];
  path_boundaries?: PathBoundaryRule[];
};

type DriftViolation = {
  check_id: 'import' | 'symbol' | 'path';
  boundary_id: string;
  file: string;
  detail: string;
};

type Args = {
  strict: boolean;
  policyPath: string;
  outJsonPath: string;
  outMarkdownPath: string;
};

const ROOT = process.cwd();
const DEFAULT_POLICY_PATH = 'client/runtime/config/ownership_drift_policy.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/ownership_drift_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/OWNERSHIP_DRIFT_GUARD_CURRENT.md';

function rel(p: string): string {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function parseArgs(argv: string[]): Args {
  const strictOut = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: strictOut.strict,
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 400),
    outJsonPath: cleanText(readFlag(argv, 'out-json') || strictOut.out || DEFAULT_OUT_JSON, 400),
    outMarkdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function listFiles(roots: string[], extensions: string[]): string[] {
  const extSet = new Set(extensions.map((value) => cleanText(value, 32).toLowerCase()).filter(Boolean));
  const files: string[] = [];
  const stack = roots
    .map((row) => path.resolve(ROOT, cleanText(row, 400)))
    .filter((absRoot) => fs.existsSync(absRoot));
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const absPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(absPath);
        continue;
      }
      if (!entry.isFile()) continue;
      const ext = path.extname(entry.name).toLowerCase();
      if (extSet.has(ext)) files.push(absPath);
    }
  }
  return files.sort((a, b) => a.localeCompare(b));
}

function parseImportSpecs(source: string): string[] {
  const specs: string[] = [];
  const importRegex =
    /(?:import\s+[^'"]*from\s+|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g;
  let match: RegExpExecArray | null = null;
  while ((match = importRegex.exec(source)) !== null) {
    specs.push(cleanText(String(match[1] || ''), 500));
  }
  return specs;
}

function matchesPattern(input: string, pattern: string): boolean {
  const normalized = cleanText(pattern, 500);
  if (!normalized) return false;
  if (normalized.startsWith('re:')) {
    try {
      return new RegExp(normalized.slice(3), 'm').test(input);
    } catch {
      return false;
    }
  }
  return input.includes(normalized);
}

function matchesAny(input: string, patterns: string[]): boolean {
  return patterns.some((pattern) => matchesPattern(input, pattern));
}

function runImportBoundaries(policy: Policy): DriftViolation[] {
  const violations: DriftViolation[] = [];
  for (const rule of policy.import_boundaries || []) {
    const boundaryId = cleanText(rule.id || 'import_boundary', 120);
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0
        ? rule.extensions
        : ['.ts', '.tsx', '.rs'];
    const forbidden = Array.isArray(rule.forbidden_import_patterns)
      ? rule.forbidden_import_patterns
      : [];
    const allow = Array.isArray(rule.allow_import_patterns) ? rule.allow_import_patterns : [];
    const files = listFiles(roots, extensions);
    for (const filePath of files) {
      const source = fs.readFileSync(filePath, 'utf8');
      for (const specRaw of parseImportSpecs(source)) {
        const spec = specRaw.replace(/\\/g, '/');
        if (!matchesAny(spec, forbidden)) continue;
        if (allow.length > 0 && matchesAny(spec, allow)) continue;
        violations.push({
          check_id: 'import',
          boundary_id: boundaryId,
          file: rel(filePath),
          detail: spec,
        });
      }
    }
  }
  return violations;
}

function runSymbolBoundaries(policy: Policy): DriftViolation[] {
  const violations: DriftViolation[] = [];
  for (const rule of policy.symbol_boundaries || []) {
    const boundaryId = cleanText(rule.id || 'symbol_boundary', 120);
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0
        ? rule.extensions
        : ['.ts', '.tsx', '.rs'];
    const forbidden = Array.isArray(rule.forbidden_symbol_patterns)
      ? rule.forbidden_symbol_patterns
      : [];
    const allowFiles = Array.isArray(rule.allow_file_patterns) ? rule.allow_file_patterns : [];
    const files = listFiles(roots, extensions);
    for (const filePath of files) {
      const fileRel = rel(filePath);
      if (allowFiles.length > 0 && matchesAny(fileRel, allowFiles)) continue;
      const source = fs.readFileSync(filePath, 'utf8');
      for (const pattern of forbidden) {
        if (!matchesPattern(source, pattern)) continue;
        violations.push({
          check_id: 'symbol',
          boundary_id: boundaryId,
          file: fileRel,
          detail: cleanText(pattern, 260),
        });
      }
    }
  }
  return violations;
}

function runPathBoundaries(policy: Policy): DriftViolation[] {
  const violations: DriftViolation[] = [];
  for (const rule of policy.path_boundaries || []) {
    const boundaryId = cleanText(rule.id || 'path_boundary', 120);
    const roots = Array.isArray(rule.scan_roots) ? rule.scan_roots : [];
    const extensions =
      Array.isArray(rule.extensions) && rule.extensions.length > 0
        ? rule.extensions
        : ['.ts', '.tsx', '.rs'];
    const forbidden = Array.isArray(rule.forbidden_path_patterns)
      ? rule.forbidden_path_patterns
      : [];
    const allow = Array.isArray(rule.allow_path_patterns) ? rule.allow_path_patterns : [];
    const files = listFiles(roots, extensions);
    for (const filePath of files) {
      const fileRel = rel(filePath);
      if (!matchesAny(fileRel, forbidden)) continue;
      if (allow.length > 0 && matchesAny(fileRel, allow)) continue;
      violations.push({
        check_id: 'path',
        boundary_id: boundaryId,
        file: fileRel,
        detail: 'forbidden_path_pattern_match',
      });
    }
  }
  return violations;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Ownership Drift Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Policy: ${payload.inputs.policy_path}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push('');
  lines.push(`- Path drift violations: ${payload.summary.path_violation_count}`);
  lines.push(`- Import drift violations: ${payload.summary.import_violation_count}`);
  lines.push(`- Symbol drift violations: ${payload.summary.symbol_violation_count}`);
  lines.push(`- Total violations: ${payload.summary.total_violation_count}`);
  lines.push('');
  lines.push('## Violations');
  lines.push('');
  lines.push('| Check | Boundary | File | Detail |');
  lines.push('| --- | --- | --- | --- |');
  const rows = Array.isArray(payload.violations) ? payload.violations : [];
  if (rows.length === 0) {
    lines.push('| (none) | - | - | - |');
  } else {
    for (const row of rows.slice(0, 180)) {
      lines.push(
        `| ${String(row.check_id)} | ${String(row.boundary_id)} | ${String(
          row.file,
        )} | ${String(row.detail).slice(0, 180)} |`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const policyPath = path.resolve(ROOT, args.policyPath);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Policy;

  const pathViolations = runPathBoundaries(policy);
  const importViolations = runImportBoundaries(policy);
  const symbolViolations = runSymbolBoundaries(policy);
  const violations = [...pathViolations, ...importViolations, ...symbolViolations];

  const payload = {
    ok: violations.length === 0,
    type: 'ownership_drift_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      policy_path: rel(policyPath),
      out_json: args.outJsonPath,
      out_markdown: args.outMarkdownPath,
    },
    summary: {
      pass: violations.length === 0,
      path_violation_count: pathViolations.length,
      import_violation_count: importViolations.length,
      symbol_violation_count: symbolViolations.length,
      total_violation_count: violations.length,
    },
    violations,
    failures: violations.map((row) => ({
      id: `ownership_drift_${row.check_id}_violation`,
      detail: `${row.boundary_id}:${row.file}:${row.detail}`,
    })),
  };

  writeTextArtifact(path.resolve(ROOT, args.outMarkdownPath), toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: path.resolve(ROOT, args.outJsonPath),
    strict: args.strict,
    ok: payload.ok,
  });
}

const exitCode = main();
if (exitCode !== 0) process.exit(exitCode);

