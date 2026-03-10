#!/usr/bin/env node
/**
 * Coreization Wave 1 static audit:
 * - verifies target TS surfaces are thin wrappers
 * - flags potential authority logic left in client target modules
 */
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';

const ROOT = process.cwd();
const TARGETS = [
  { module: 'security', dir: 'client/runtime/systems/security' },
  { module: 'spine', dir: 'client/runtime/systems/spine' },
  { module: 'memory', dir: 'client/runtime/systems/memory' },
  { module: 'autonomy', dir: 'client/runtime/systems/autonomy' },
  { module: 'workflow', dir: 'client/runtime/systems/workflow' },
  { module: 'ops-daemon', dir: 'client/runtime/systems/ops', include: ['protheusd.ts'] },
];

const WRAPPER_PATTERNS = [
  /createLegacyRetiredModule/,
  /createOpsLaneBridge/,
  /createManifestLaneBridge/,
  /legacy-retired-lane/,
  /ts_bootstrap/,
  /ts_entrypoint/,
];

const LAYER_OWNERSHIP_RE = /Layer ownership:\s*core\//;
const MAX_THIN_WRAPPER_LINES = 80;

function walk(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...walk(p));
      continue;
    }
    if (/\.(ts|js)$/.test(entry.name)) {
      out.push(p);
    }
  }
  return out;
}

function classify(source) {
  const isWrapper = WRAPPER_PATTERNS.some((re) => re.test(source));
  const hasOwnership = LAYER_OWNERSHIP_RE.test(source);
  return { isWrapper, hasOwnership };
}

function rel(p) {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function fileSetForTarget(target) {
  const absDir = path.join(ROOT, target.dir);
  if (!fs.existsSync(absDir)) return [];
  const all = walk(absDir);
  if (!target.include || target.include.length === 0) {
    return all;
  }
  const includes = new Set(target.include.map((v) => path.posix.normalize(v)));
  return all.filter((abs) => includes.has(path.basename(abs)));
}

function main() {
  const outArgIndex = process.argv.findIndex((v) => v === '--out');
  const outPath =
    outArgIndex >= 0 && process.argv[outArgIndex + 1]
      ? path.resolve(process.argv[outArgIndex + 1])
      : null;

  let revision = 'unknown';
  try {
    revision = execSync('git rev-parse HEAD', { cwd: ROOT, encoding: 'utf8' }).trim();
  } catch {}

  const modules = [];
  const violations = [];
  const warnings = [];

  for (const target of TARGETS) {
    const files = fileSetForTarget(target);
    let tsFiles = 0;
    let jsFiles = 0;
    let wrappers = 0;
    let nonWrappers = 0;

    for (const abs of files) {
      const source = fs.readFileSync(abs, 'utf8');
      const { isWrapper, hasOwnership } = classify(source);
      const lines = source.split('\n').length;
      const ext = path.extname(abs).toLowerCase();
      if (ext === '.ts') tsFiles += 1;
      if (ext === '.js') jsFiles += 1;

      if (isWrapper) wrappers += 1;
      else nonWrappers += 1;

      const item = {
        module: target.module,
        path: rel(abs),
        ext,
        lines,
      };

      if (ext === '.ts' && !isWrapper) {
        violations.push({
          ...item,
          reason: 'ts_non_wrapper_in_target_module',
        });
      }

      if (ext === '.ts' && isWrapper && !hasOwnership) {
        warnings.push({
          ...item,
          reason: 'ts_wrapper_missing_layer_ownership_comment',
        });
      }

      if (isWrapper && lines > MAX_THIN_WRAPPER_LINES) {
        warnings.push({
          ...item,
          reason: 'wrapper_exceeds_thin_line_budget',
          max_lines: MAX_THIN_WRAPPER_LINES,
        });
      }
    }

    modules.push({
      module: target.module,
      directory: target.dir,
      files: files.length,
      ts_files: tsFiles,
      js_files: jsFiles,
      wrappers,
      non_wrappers: nonWrappers,
    });
  }

  const payload = {
    generated_at: new Date().toISOString(),
    revision,
    policy: {
      target: 'Hard Coreization Wave 1 static surface check',
      max_thin_wrapper_lines: MAX_THIN_WRAPPER_LINES,
    },
    modules,
    summary: {
      module_count: modules.length,
      violation_count: violations.length,
      warning_count: warnings.length,
      pass: violations.length === 0,
    },
    violations,
    warnings,
  };

  if (outPath) {
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
  }

  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  process.exit(violations.length === 0 ? 0 : 1);
}

main();
