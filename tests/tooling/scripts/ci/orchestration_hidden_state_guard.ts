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
    for (const pattern of FORBIDDEN_STATE_PATTERNS) {
      if (!pattern.re.test(source)) continue;
      if (isEphemeralContainerAllowed(file, source) && pattern.detail === 'BTreeMap') {
        continue;
      }
      violations.push({
        file: rel(file),
        reason: 'hidden_state_container_forbidden',
        detail: pattern.detail,
      });
    }
    for (const pattern of FORBIDDEN_DURABLE_IO_PATTERNS) {
      if (!pattern.re.test(source)) continue;
      violations.push({
        file: rel(file),
        reason: 'durable_io_in_orchestration_forbidden',
        detail: pattern.detail,
      });
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
