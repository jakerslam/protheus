#!/usr/bin/env node
/* eslint-disable no-console */
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_amputation_regression_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_AMPUTATION_REGRESSION_GUARD_CURRENT.md';
const POLICY_DOC = 'docs/workspace/shell_independent_operation_policy.md';

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
  includeControlledViolation: boolean;
  keepFixture: boolean;
  skipSmoke: boolean;
};

type Violation = {
  kind: string;
  path?: string;
  detail: string;
};

type SmokeResult = {
  name: string;
  command: string[];
  cwd: string;
  status: number | null;
  signal: string | null;
  ok: boolean;
  stdout_tail: string;
  stderr_tail: string;
};

type CleanupResult = {
  attempted: boolean;
  ok: boolean | null;
  error?: string;
};

const TOP_LEVEL_FIXTURE_ITEMS = [
  'Cargo.toml',
  'Cargo.lock',
  'rust-toolchain.toml',
  'LICENSE',
  'package.json',
  'core',
  'surface',
  'adapters',
  'xtask',
  'client/cli',
  'client/pure-workspace',
];

const REMOVED_BROWSER_SHELL_PATHS = [
  'client/runtime/systems/ui/infring_static',
  'client/runtime/systems/ui/infring_static/js/svelte',
  'client/runtime/systems/ui/infring_static/css',
  'client/runtime/systems/ui/infring_static/assets',
  'client/runtime/systems/ui/infring_static/vendor',
];

const STATIC_SCAN_ROOTS = [
  'core',
  'surface',
  'adapters',
  'client/cli',
  'client/pure-workspace',
];

const ALLOWED_BROWSER_ASSET_OWNERS = new Set([
  'adapters/runtime/dashboard_asset_router.ts',
  'adapters/runtime/infring_dashboard.ts',
]);

const SOURCE_EXTENSIONS = new Set(['.rs', '.ts', '.tsx', '.sh']);

const BROWSER_ASSET_TOKEN = String.raw`(?:client/runtime/systems/ui/infring_static|infring_static|js/svelte|vendor/alpine|dashboard\.html|dashboard\.css)`;

const DEPENDENCY_PATTERNS: Array<{ kind: string; pattern: RegExp }> = [
  {
    kind: 'runtime_imports_browser_shell_asset',
    pattern: new RegExp(String.raw`\b(?:import|require)\s*(?:\(|[^;\n]*from\s*)['"][^'"\n]*${BROWSER_ASSET_TOKEN}`, 'i'),
  },
  {
    kind: 'runtime_reads_browser_shell_asset',
    pattern: new RegExp(String.raw`\b(?:readFileSync|readFile|createReadStream|readdirSync|existsSync|read_to_string)\s*\([^)\n]*${BROWSER_ASSET_TOKEN}`, 'i'),
  },
  {
    kind: 'runtime_embeds_browser_shell_asset',
    pattern: new RegExp(String.raw`\b(?:include_str!|include_bytes!)\s*\([^)\n]*${BROWSER_ASSET_TOKEN}`, 'i'),
  },
  {
    kind: 'runtime_execs_browser_shell_asset',
    pattern: new RegExp(String.raw`\b(?:spawn|spawnSync|execFile|exec|Command::new)\s*\([^)\n]*${BROWSER_ASSET_TOKEN}`, 'i'),
  },
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function rel(filePath: string, root = ROOT): string {
  return path.relative(root, filePath).replace(/\\/g, '/');
}

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600),
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
    keepFixture: parseBool(readFlag(argv, 'keep-fixture'), false),
    skipSmoke: parseBool(readFlag(argv, 'skip-smoke'), false),
  };
}

function shouldSkipCopy(absPath: string): boolean {
  const pathRel = rel(absPath);
  const parts = pathRel.split('/');
  if (parts.includes('.git') || parts.includes('target') || parts.includes('node_modules')) return true;
  if (pathRel === 'core/local' || pathRel.startsWith('core/local/')) return true;
  if (pathRel === 'local' || pathRel.startsWith('local/')) return true;
  for (const removedPath of REMOVED_BROWSER_SHELL_PATHS) {
    if (pathRel === removedPath || pathRel.startsWith(`${removedPath}/`)) return true;
  }
  return false;
}

function createFixture(violations: Violation[]): string {
  const shadowRoot = abs('local/workspace/shadow');
  fs.mkdirSync(shadowRoot, { recursive: true });
  const fixtureRoot = fs.mkdtempSync(path.join(shadowRoot, 'shell-amputation-regression-'));
  for (const item of TOP_LEVEL_FIXTURE_ITEMS) {
    const source = abs(item);
    const destination = path.join(fixtureRoot, item);
    if (!fs.existsSync(source)) {
      violations.push({ kind: 'shell_amputation_fixture_source_missing', path: item, detail: 'Required source path for fixture copy is missing.' });
      continue;
    }
    fs.mkdirSync(path.dirname(destination), { recursive: true });
    fs.cpSync(source, destination, {
      recursive: true,
      filter: (src) => !shouldSkipCopy(src),
    });
  }
  const manifest = {
    kind: 'shell_amputation_fixture_manifest',
    root: fixtureRoot,
    removed_browser_shell_paths: REMOVED_BROWSER_SHELL_PATHS,
    copied_roots: TOP_LEVEL_FIXTURE_ITEMS,
  };
  fs.writeFileSync(path.join(fixtureRoot, 'shell-amputation-fixture.json'), `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
  return fixtureRoot;
}

function assertShellAssetsAbsent(fixtureRoot: string, violations: Violation[]): void {
  for (const removedPath of REMOVED_BROWSER_SHELL_PATHS) {
    if (fs.existsSync(path.join(fixtureRoot, removedPath))) {
      violations.push({
        kind: 'shell_amputation_fixture_still_contains_browser_assets',
        path: removedPath,
        detail: 'Disposable no-Shell fixture still contains a browser Shell asset path.',
      });
    }
  }
}

function walkFiles(rootRel: string, baseRoot: string, files: string[]): void {
  const start = path.join(baseRoot, rootRel);
  if (!fs.existsSync(start)) return;
  const stack = [start];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    const stat = fs.statSync(current);
    const fileRel = rel(current, baseRoot);
    const parts = fileRel.split('/');
    if (stat.isDirectory()) {
      if (parts.includes('.git') || parts.includes('target') || parts.includes('node_modules') || parts.includes('local')) continue;
      for (const entry of fs.readdirSync(current)) stack.push(path.join(current, entry));
      continue;
    }
    if (SOURCE_EXTENSIONS.has(path.extname(current))) files.push(current);
  }
}

function scanRuntimeDependencies(baseRoot: string, sourceLabel: string, violations: Violation[]): number {
  const files: string[] = [];
  for (const scanRoot of STATIC_SCAN_ROOTS) walkFiles(scanRoot, baseRoot, files);
  let checked = 0;
  for (const file of files) {
    const fileRel = rel(file, baseRoot);
    if (ALLOWED_BROWSER_ASSET_OWNERS.has(fileRel)) continue;
    checked += 1;
    const source = fs.readFileSync(file, 'utf8');
    const lines = source.split(/\r?\n/);
    for (let index = 0; index < lines.length; index += 1) {
      const line = lines[index];
      for (const rule of DEPENDENCY_PATTERNS) {
        if (rule.pattern.test(line)) {
          violations.push({
            kind: rule.kind,
            path: `${sourceLabel}:${fileRel}:${index + 1}`,
            detail: 'Non-UI runtime path depends on browser Shell assets.',
          });
        }
      }
    }
  }
  return checked;
}

function installControlledViolation(fixtureRoot: string): void {
  const target = path.join(fixtureRoot, 'core/layer0/ops/src/__controlled_shell_asset_import.rs');
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(
    target,
    'const CONTROLLED: &str = include_str!("client/runtime/systems/ui/infring_static/js/app.ts");\n',
    'utf8',
  );
}

function truncateOutput(value: unknown): string {
  const text = String(value || '').trim();
  if (text.length <= 1600) return text;
  return text.slice(-1600);
}

function runSmokeCommand(fixtureRoot: string, name: string, command: string[]): SmokeResult {
  const result = spawnSync(command[0], command.slice(1), {
    cwd: fixtureRoot,
    encoding: 'utf8',
    timeout: 180_000,
    env: {
      ...process.env,
      CARGO_TARGET_DIR: path.join(ROOT, 'target/shell-amputation-regression'),
      INFRING_WORKSPACE_ROOT: fixtureRoot,
      INFRING_RUNTIME_ROOT: fixtureRoot,
      INFRING_SHELL_AMPUTATION_FIXTURE: '1',
      INFRING_OPS_USE_PREBUILT: '0',
      INFRING_OPS_ALLOW_STALE: '1',
      NO_COLOR: '1',
    },
  });
  return {
    name,
    command,
    cwd: fixtureRoot,
    status: result.status,
    signal: result.signal,
    ok: result.status === 0,
    stdout_tail: truncateOutput(result.stdout),
    stderr_tail: truncateOutput(result.stderr),
  };
}

function runSmoke(fixtureRoot: string): SmokeResult[] {
  return [
    runSmokeCommand(fixtureRoot, 'orchestration_surface_cargo_check', [
      'cargo', 'check', '--manifest-path', 'surface/orchestration/Cargo.toml', '--quiet',
    ]),
    runSmokeCommand(fixtureRoot, 'core_ops_cli_cargo_check', [
      'cargo', 'check', '--manifest-path', 'core/layer0/ops/Cargo.toml', '--bin', 'infring-ops', '--quiet',
    ]),
    runSmokeCommand(fixtureRoot, 'rust_cli_command_registry_smoke', [
      'cargo', 'run', '--quiet', '--manifest-path', 'core/layer0/ops/Cargo.toml', '--bin', 'infring-ops', '--',
      'command-list-kernel', '--mode=help',
    ]),
    runSmokeCommand(fixtureRoot, 'gateway_status_contract_smoke', [
      'cargo', 'run', '--quiet', '--manifest-path', 'core/layer0/ops/Cargo.toml', '--bin', 'infring-ops', '--',
      'daemon-control', 'status', '--json', '--gateway-persist=0',
    ]),
  ];
}

function validatePolicyDoc(violations: Violation[]): void {
  if (!fs.existsSync(abs(POLICY_DOC))) {
    violations.push({ kind: 'shell_independence_policy_missing', path: POLICY_DOC, detail: 'Canonical Shell-independent operation policy is missing.' });
    return;
  }
  const doc = fs.readFileSync(abs(POLICY_DOC), 'utf8');
  const tokens = [
    'The system must operate without the browser Shell.',
    'Shell independence must be tested by deletion',
    'Shell amputation regression guard',
    'ops:shell:amputation:guard',
  ];
  for (const token of tokens) {
    if (!doc.includes(token)) {
      violations.push({ kind: 'shell_independence_policy_missing_guard_token', path: POLICY_DOC, detail: `Missing token: ${token}` });
    }
  }
}

function markdownReport(report: Record<string, any>): string {
  const lines = [
    '# Shell Amputation Regression Guard',
    '',
    `- ok: ${report.ok}`,
    `- revision: ${report.revision}`,
    `- fixture: ${report.fixture_root}`,
    `- fixture retained: ${report.fixture_kept}`,
    `- scanned source files: ${report.static_scan.source_files_checked}`,
    `- scanned fixture files: ${report.static_scan.fixture_files_checked}`,
    `- smoke skipped: ${report.smoke_skipped}`,
    `- violation count: ${report.violations.length}`,
    '',
    '## Smoke',
  ];
  for (const row of report.smoke_results as SmokeResult[]) {
    lines.push(`- ${row.name}: ${row.ok ? 'pass' : 'fail'} (${row.status ?? row.signal ?? 'no-status'})`);
  }
  lines.push('', '## Violations');
  if (report.violations.length === 0) {
    lines.push('- none');
  } else {
    for (const violation of report.violations as Violation[]) {
      lines.push(`- ${violation.kind}: ${violation.path || '<repo>'} — ${violation.detail}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function cleanupFixture(fixtureRoot: string, keepFixture: boolean): CleanupResult {
  if (keepFixture) return { attempted: false, ok: null };
  try {
    fs.rmSync(fixtureRoot, { recursive: true, force: true });
    return { attempted: true, ok: true };
  } catch (err) {
    return { attempted: true, ok: false, error: String((err as Error).message || err) };
  }
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const violations: Violation[] = [];
  validatePolicyDoc(violations);
  const fixtureRoot = createFixture(violations);
  if (args.includeControlledViolation) installControlledViolation(fixtureRoot);
  assertShellAssetsAbsent(fixtureRoot, violations);
  const sourceFilesChecked = scanRuntimeDependencies(ROOT, 'source', violations);
  const fixtureFilesChecked = scanRuntimeDependencies(fixtureRoot, 'fixture', violations);
  const smokeResults = args.skipSmoke ? [] : runSmoke(fixtureRoot);
  for (const smoke of smokeResults) {
    if (!smoke.ok) {
      violations.push({
        kind: 'shell_amputation_smoke_failed',
        path: smoke.name,
        detail: `Command failed: ${smoke.command.join(' ')}`,
      });
    }
  }
  const cleanup = cleanupFixture(fixtureRoot, args.keepFixture);
  const report = {
    ok: violations.length === 0,
    type: 'shell_amputation_regression_guard',
    revision: currentRevision(),
    fixture_root: fixtureRoot,
    fixture_kept: args.keepFixture,
    fixture_retention: args.keepFixture ? 'operator_requested_keep' : 'removed_after_run',
    fixture_cleanup: cleanup,
    platform: `${os.platform()}-${os.arch()}`,
    removed_browser_shell_paths: REMOVED_BROWSER_SHELL_PATHS,
    static_scan: { roots: STATIC_SCAN_ROOTS, source_files_checked: sourceFilesChecked, fixture_files_checked: fixtureFilesChecked },
    smoke_skipped: args.skipSmoke,
    smoke_results: smokeResults,
    violations,
  };
  writeTextArtifact(args.outMarkdown, markdownReport(report));
  return emitStructuredResult(report, { outPath: args.outJson, strict: args.strict, ok: report.ok });
}

process.exit(main());
