#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/cross_layer_import_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/CROSS_LAYER_IMPORT_GUARD_CURRENT.md';
const SCAN_ROOTS = ['client', 'surface/orchestration', 'adapters'];
const EXTENSIONS = new Set(['.ts', '.tsx', '.js', '.mjs', '.cjs']);
const ROOT_SPEC_PREFIXES = ['client/', 'core/', 'surface/', 'adapters/', 'tests/'];
const IGNORED_DIR_NAMES = new Set([
  'node_modules',
  '.git',
  '.next',
  '.svelte-kit',
  'dist',
  'build',
  'coverage',
]);

type Violation = {
  rule_id: string;
  file: string;
  spec: string;
  resolved_target: string;
  detail: string;
};

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

function parseArgs(argv: string[]): Args {
  const parsed = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: parsed.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || parsed.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function walk(scanRoot: string): string[] {
  const rootPath = path.resolve(ROOT, scanRoot);
  if (!fs.existsSync(rootPath)) return [];
  const out: string[] = [];
  const stack = [rootPath];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      if (entry.isDirectory()) {
        if (IGNORED_DIR_NAMES.has(entry.name) || entry.name.startsWith('.')) continue;
        stack.push(path.join(current, entry.name));
        continue;
      }
      if (!entry.isFile()) continue;
      const abs = path.join(current, entry.name);
      if (EXTENSIONS.has(path.extname(abs).toLowerCase())) out.push(abs);
    }
  }
  return out.sort();
}

function parseImportSpecs(source: string): string[] {
  const specs: string[] = [];
  const re = /(?:import\s+[^'"]*from\s+|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g;
  let match: RegExpExecArray | null = null;
  while ((match = re.exec(source)) != null) {
    specs.push(cleanText(match[1] || '', 600));
  }
  return specs.filter(Boolean);
}

function resolveRelativeImport(fromFile: string, spec: string): string | null {
  const base = path.resolve(path.dirname(fromFile), spec);
  const baseExt = path.extname(base).toLowerCase();
  const stem = baseExt ? base.slice(0, -baseExt.length) : base;
  const candidates = [
    base,
    `${base}.ts`,
    `${base}.tsx`,
    `${base}.js`,
    `${base}.mjs`,
    `${base}.cjs`,
    path.join(base, 'index.ts'),
    path.join(base, 'index.tsx'),
    path.join(base, 'index.js'),
    path.join(base, 'index.mjs'),
    path.join(base, 'index.cjs'),
    stem,
    `${stem}.ts`,
    `${stem}.tsx`,
    `${stem}.js`,
    `${stem}.mjs`,
    `${stem}.cjs`,
    path.join(stem, 'index.ts'),
    path.join(stem, 'index.tsx'),
    path.join(stem, 'index.js'),
    path.join(stem, 'index.mjs'),
    path.join(stem, 'index.cjs'),
  ];
  for (const candidate of candidates) {
    if (!fs.existsSync(candidate)) continue;
    if (!fs.statSync(candidate).isFile()) continue;
    return rel(candidate);
  }
  return null;
}

function resolveImportTarget(fromFile: string, spec: string): string | null {
  if (!spec) return null;
  if (spec.startsWith('.')) {
    if (spec === './$types' || spec.endsWith('/$types')) return null;
    return resolveRelativeImport(fromFile, spec);
  }
  const normalized = spec.replace(/\\/g, '/');
  for (const prefix of ROOT_SPEC_PREFIXES) {
    if (normalized.startsWith(prefix)) return normalized;
  }
  return null;
}

function isOrchestrationContractPath(target: string): boolean {
  return (
    target === 'surface/orchestration/src/contracts.rs' ||
    target.startsWith('surface/orchestration/contracts/') ||
    target.startsWith('surface/orchestration/scripts/')
  );
}

function isClientImportingOrchestrationInternals(source: string, target: string): boolean {
  if (!source.startsWith('client/')) return false;
  if (!target.startsWith('surface/orchestration/')) return false;
  if (isOrchestrationContractPath(target)) return false;
  return target.startsWith('surface/orchestration/src/') || target.startsWith('surface/orchestration/tests/');
}

function isKernelPolicyAuthorityPath(target: string): boolean {
  if (!target.startsWith('core/')) return false;
  const lower = target.toLowerCase();
  return (
    lower.includes('/policy') ||
    lower.includes('policy_') ||
    lower.includes('/admission') ||
    lower.includes('admission_') ||
    lower.includes('/scheduler') ||
    lower.includes('scheduler_')
  );
}

function isOrchestrationImportingKernelPolicyAuthority(source: string, target: string): boolean {
  if (!source.startsWith('surface/orchestration/')) return false;
  return isKernelPolicyAuthorityPath(target);
}

function isAdaptersImportingSchedulerAdmissionAuthority(source: string, target: string): boolean {
  if (!source.startsWith('adapters/')) return false;
  if (!(target.startsWith('core/') || target.startsWith('surface/orchestration/src/'))) return false;
  const lower = target.toLowerCase();
  return (
    lower.includes('/scheduler') ||
    lower.includes('scheduler_') ||
    lower.includes('/admission') ||
    lower.includes('admission_')
  );
}

function toMarkdown(payload: {
  generated_at: string;
  revision: string;
  summary: {
    scanned_files: number;
    scanned_imports: number;
    violation_count: number;
    pass: boolean;
  };
  violations: Violation[];
}): string {
  const lines: string[] = [];
  lines.push('# Cross-Layer Import Guard');
  lines.push('');
  lines.push(`- Generated: ${payload.generated_at}`);
  lines.push(`- Revision: ${payload.revision}`);
  lines.push(`- Pass: ${payload.summary.pass ? 'yes' : 'no'}`);
  lines.push(`- Scanned files: ${payload.summary.scanned_files}`);
  lines.push(`- Scanned imports: ${payload.summary.scanned_imports}`);
  lines.push(`- Violations: ${payload.summary.violation_count}`);
  lines.push('');
  if (payload.violations.length === 0) {
    lines.push('No violations detected.');
    lines.push('');
    return `${lines.join('\n')}\n`;
  }
  lines.push('## Violations');
  lines.push('');
  for (const violation of payload.violations) {
    lines.push(
      `- [${violation.rule_id}] ${violation.file} -> ${violation.spec} (${violation.resolved_target}) :: ${violation.detail}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const files = SCAN_ROOTS.flatMap((root) => walk(root));
  const revision = currentRevision(ROOT);
  const violations: Violation[] = [];
  let scannedImports = 0;

  for (const filePath of files) {
    const sourcePath = rel(filePath);
    const source = fs.readFileSync(filePath, 'utf8');
    const specs = parseImportSpecs(source);
    scannedImports += specs.length;
    for (const spec of specs) {
      const target = resolveImportTarget(filePath, spec);
      if (!target) continue;
      if (isClientImportingOrchestrationInternals(sourcePath, target)) {
        violations.push({
          rule_id: 'client_orchestration_internal_import_forbidden',
          file: sourcePath,
          spec,
          resolved_target: target,
          detail: 'client imports orchestration internals (only contracts/scripts are allowed)',
        });
      }
      if (isOrchestrationImportingKernelPolicyAuthority(sourcePath, target)) {
        violations.push({
          rule_id: 'orchestration_kernel_policy_import_forbidden',
          file: sourcePath,
          spec,
          resolved_target: target,
          detail: 'orchestration imports kernel policy/admission/scheduler authority path',
        });
      }
      if (isAdaptersImportingSchedulerAdmissionAuthority(sourcePath, target)) {
        violations.push({
          rule_id: 'adapter_scheduler_admission_import_forbidden',
          file: sourcePath,
          spec,
          resolved_target: target,
          detail: 'gateway adapter imports scheduler/admission authority path',
        });
      }
    }
  }

  const payload = {
    type: 'cross_layer_import_guard',
    generated_at: new Date().toISOString(),
    revision,
    summary: {
      scanned_files: files.length,
      scanned_imports: scannedImports,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    violations,
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: violations.length === 0,
  });
}

process.exit(main());
