#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

type Args = {
  strict: boolean;
  out: string;
};

type Violation = {
  file: string;
  reason: string;
  detail: string;
};

const ROOT = process.cwd();

const FORBIDDEN_STATE_PATTERNS: Array<{ re: RegExp; detail: string }> = [
  { re: /\bHashMap\s*</, detail: 'HashMap' },
  { re: /\bBTreeMap\s*</, detail: 'BTreeMap' },
  { re: /\bDashMap\s*</, detail: 'DashMap' },
  { re: /\bVecDeque\s*</, detail: 'VecDeque' },
  { re: /\blazy_static!\s*\{/, detail: 'lazy_static' },
  { re: /\bOnceLock\s*</, detail: 'OnceLock' },
];

const FORBIDDEN_DURABLE_IO_PATTERNS: Array<{ re: RegExp; detail: string }> = [
  { re: /\bOpenOptions::new\s*\(/, detail: 'OpenOptions::new' },
  { re: /\bFile::create\s*\(/, detail: 'File::create' },
  { re: /\bstd::fs::write\s*\(/, detail: 'std::fs::write' },
  { re: /\bfs::write\s*\(/, detail: 'fs::write' },
  { re: /\bcreate_dir_all\s*\(/, detail: 'create_dir_all' },
];

const FORBIDDEN_SELF_MAINTENANCE_PATTERNS: Array<{ re: RegExp; detail: string }> = [
  { re: /\bstd::fs::/, detail: 'std::fs' },
  { re: /\btokio::fs::/, detail: 'tokio::fs' },
  { re: /\bstd::process::Command\b/, detail: 'std::process::Command' },
];

const PRESENTATION_FILES = new Set([
  'surface/orchestration/src/clarification.rs',
  'surface/orchestration/src/progress.rs',
  'surface/orchestration/src/result_packaging.rs',
]);

const REQUIRED_TRANSIENT_CONTEXT_PATTERNS: Array<{ re: RegExp; detail: string }> = [
  { re: /\bstruct\s+TransientContextStore\s*\{[\s\S]*entries:\s*BTreeMap<[\s\S]*execution_observations:\s*BTreeMap</m, detail: 'transient_store_struct_fields' },
  { re: /\bfn\s+upsert_execution_observation\s*\(/, detail: 'upsert_execution_observation' },
  { re: /\bfn\s+execution_observation\s*\(/, detail: 'execution_observation_accessor' },
  { re: /\bfn\s+clear_execution_observation\s*\(/, detail: 'clear_execution_observation' },
  { re: /\bfn\s+prune_inactive_execution_observations\s*\(/, detail: 'prune_inactive_execution_observations' },
  { re: /\bwrite_ephemeral\b/, detail: 'write_ephemeral_usage' },
  { re: /\bcleanup_with_cas\b/, detail: 'cleanup_with_cas_usage' },
];

function parseArgs(argv: string[]): Args {
  const out: Args = {
    strict: false,
    out: 'core/local/artifacts/orchestration_hidden_state_guard_current.json',
  };
  for (const arg of argv) {
    if (arg === '--strict' || arg === '--strict=1') out.strict = true;
    else if (arg.startsWith('--strict=')) {
      const value = arg.slice('--strict='.length).trim().toLowerCase();
      out.strict = value === '1' || value === 'true' || value === 'yes' || value === 'on';
    } else if (arg.startsWith('--out=')) {
      out.out = arg.slice('--out='.length).trim() || out.out;
    }
  }
  return out;
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function walkRustFiles(base: string): string[] {
  if (!fs.existsSync(base)) return [];
  const out: string[] = [];
  const stack = [base];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const abs = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(abs);
        continue;
      }
      if (entry.isFile() && abs.endsWith('.rs')) out.push(abs);
    }
  }
  return out.sort();
}

function isEphemeralContainerAllowed(file: string, source: string): boolean {
  const normalized = rel(file);
  if (normalized !== 'surface/orchestration/src/transient_context.rs') return false;
  return (
    source.includes('EphemeralMemoryHeap') &&
    source.includes('write_ephemeral') &&
    source.includes('cleanup_with_cas')
  );
}

function run(args: Args): number {
  const files = walkRustFiles(path.resolve(ROOT, 'surface/orchestration/src'));
  const violations: Violation[] = [];
  for (const file of files) {
    const source = fs.readFileSync(file, 'utf8');
    const normalized = rel(file);
    for (const pattern of FORBIDDEN_STATE_PATTERNS) {
      if (!pattern.re.test(source)) continue;
      if (isEphemeralContainerAllowed(file, source) && pattern.detail === 'BTreeMap') {
        continue;
      }
      violations.push({
        file: normalized,
        reason: 'hidden_state_container_forbidden',
        detail: pattern.detail,
      });
    }
    for (const pattern of FORBIDDEN_DURABLE_IO_PATTERNS) {
      if (!pattern.re.test(source)) continue;
      violations.push({
        file: normalized,
        reason: 'durable_io_in_orchestration_forbidden',
        detail: pattern.detail,
      });
    }

    if (normalized.startsWith('surface/orchestration/src/planner/')) {
      if (
        /\btransient_context\b|\bTransientContextStore\b|\bself_maintenance\b/.test(source)
      ) {
        violations.push({
          file: normalized,
          reason: 'planner_domain_violation',
          detail: 'planner must not depend on transient context or self_maintenance domains',
        });
      }
    }

    if (PRESENTATION_FILES.has(normalized)) {
      if (
        /\bcrate::planner\b|\bcrate::transient_context\b|\bcrate::self_maintenance\b/.test(source)
      ) {
        violations.push({
          file: normalized,
          reason: 'presentation_domain_violation',
          detail: 'presentation files may depend only on orchestration contracts and projections',
        });
      }
    }

    if (normalized === 'surface/orchestration/src/transient_context.rs') {
      if (/\bcrate::planner\b|\bcrate::self_maintenance\b/.test(source)) {
        violations.push({
          file: normalized,
          reason: 'transient_domain_violation',
          detail: 'transient context must remain isolated from planner and self_maintenance domains',
        });
      }
      for (const pattern of REQUIRED_TRANSIENT_CONTEXT_PATTERNS) {
        if (pattern.re.test(source)) continue;
        violations.push({
          file: normalized,
          reason: 'transient_context_contract_missing',
          detail: pattern.detail,
        });
      }
    }

    if (normalized.startsWith('surface/orchestration/src/self_maintenance/')) {
      for (const pattern of FORBIDDEN_SELF_MAINTENANCE_PATTERNS) {
        if (!pattern.re.test(source)) continue;
        violations.push({
          file: normalized,
          reason: 'self_maintenance_direct_io_forbidden',
          detail: pattern.detail,
        });
      }
    }
  }
  const payload = {
    type: 'orchestration_hidden_state_guard',
    generated_at: new Date().toISOString(),
    summary: {
      file_count: files.length,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    violations,
  };
  const outPath = path.resolve(ROOT, args.out);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(payload, null, 2));
  if (args.strict && violations.length > 0) return 1;
  return 0;
}

process.exit(run(parseArgs(process.argv.slice(2))));
