#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';

const ROOT = process.cwd();
const OUT = 'core/local/artifacts/assurance_physical_domain_placement_guard_current.json';

type Failure = { id: string; path: string; detail: string; category?: string };
type GateRow = { canonical_definition_paths?: unknown };
type Exemption = { path?: string; path_prefix?: string; owner?: string; reason?: string; expires?: string };

const CANONICAL_PREFIXES = [
  'validation/',
  'observability/',
];

const HARNESS_EXEMPT_PATHS = new Set([
  'tests/tooling/config/tooling_gate_registry.json',
]);

const CI_WORKFLOW_HARNESS_COMMANDS: Record<string, string[]> = {
  '.github/workflows/f100-a-plus-scorecard.yml': ['npm run -s ops:f100-a-plus:run'],
};

const INLINE_ASSURANCE_DEFINITION_PATTERN =
  /canonical_definition_paths|scorecard_derivation_rules|release_gate_thresholds|benchmark_regression_budgets|runtime_soak_scenarios|assurance_validation_registry/i;

const HARNESS_EXEMPT_PREFIXES = [
  'tests/tooling/scripts/',
  'tests/fixtures/',
  'tests/vitest/',
  'tests/client-memory-tools/',
];

const RETIRED_GOVERNED_PREFIXES: Array<{ category: string; prefix: string }> = [
  { category: 'release_gate_definition', prefix: 'releases/proof-packs/' },
];

const GOVERNED_PATTERNS: Array<{ category: string; pattern: RegExp }> = [
  { category: 'test_lifecycle_definition', pattern: /test[_-]?maturity|test[_-]?lifecycle|temporary[_-]?test|unregistered[_-]?test/i },
  { category: 'eval_definition', pattern: /(^|[/_.-])eval(s|_|-|\.|$)|gold_dataset|review_labels|judge_human/i },
  { category: 'scorecard_definition', pattern: /scorecard/i },
  { category: 'release_gate_definition', pattern: /release[_-]?gate|release[_-]?proof[_-]?pack|release[_-]?blocker|release[_-]?verdict/i },
  { category: 'benchmark_definition', pattern: /benchmark|boundedness|runtime[_-]?empirical|runtime[_-]?soak|soak[_-]?scenarios/i },
  { category: 'telemetry_contract', pattern: /telemetry|observability|evidence[_-]?envelope|freshness[_-]?policy|health[_-]?stream|runtime[_-]?finding/i },
  { category: 'sentinel_source_registry', pattern: /sentinel.*(source|registry|trace|observer|evidence)|assurance[_-]?observability/i },
];

function readFlag(name: string): string | undefined {
  const prefix = `--${name}=`;
  const value = process.argv.find((arg) => arg.startsWith(prefix));
  return value ? value.slice(prefix.length) : undefined;
}

function writeJson(rel: string, payload: unknown) {
  const abs = path.resolve(ROOT, rel);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`);
}

function readJson<T>(rel: string): T {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, rel), 'utf8')) as T;
}

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/').replace(/^\.\//, '');
}

function isDefinitionExtension(file: string): boolean {
  return /\.(json|jsonl|ya?ml)$/i.test(file);
}

function governedCategory(file: string): string | null {
  if (!isDefinitionExtension(file)) return null;
  const lower = file.toLowerCase();
  for (const { category, prefix } of RETIRED_GOVERNED_PREFIXES) {
    if (lower.startsWith(prefix)) return category;
  }
  for (const { category, pattern } of GOVERNED_PATTERNS) {
    if (pattern.test(lower)) return category;
  }
  return null;
}

function trackedFiles(): string[] {
  try {
    return execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' })
      .split(/\r?\n/)
      .map(normalizePath)
      .filter(Boolean)
      .filter((file) => fs.existsSync(path.resolve(ROOT, file)));
  } catch {
    return [];
  }
}

function collectMirrorPaths(): Set<string> {
  const allowed = new Set<string>();
  for (const root of ['validation', 'observability']) {
    const stack = [path.resolve(ROOT, root)];
    while (stack.length) {
      const dir = stack.pop()!;
      if (!fs.existsSync(dir)) continue;
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const abs = path.join(dir, entry.name);
        if (entry.isDirectory()) {
          stack.push(abs);
          continue;
        }
        if (entry.name !== 'compatibility_mirrors.json') continue;
        const rel = normalizePath(path.relative(ROOT, abs));
        const payload: any = readJson(rel);
        for (const mirror of payload.mirrors || []) {
          if (typeof mirror.legacy_path === 'string') allowed.add(normalizePath(mirror.legacy_path));
        }
      }
    }
  }
  return allowed;
}

function isCiWorkflowHarnessOnly(file: string): boolean {
  const requiredCommands = CI_WORKFLOW_HARNESS_COMMANDS[file];
  if (!requiredCommands) return false;
  if (!/^\.github\/workflows\/[^/]+\.ya?ml$/i.test(file)) return false;
  const text = fs.readFileSync(path.resolve(ROOT, file), 'utf8');
  if (INLINE_ASSURANCE_DEFINITION_PATTERN.test(text)) return false;
  return requiredCommands.every((command) => text.includes(`run: ${command}`));
}

function isAllowedDefinitionPath(file: string, mirrorPaths: Set<string>): boolean {
  if (CANONICAL_PREFIXES.some((prefix) => file.startsWith(prefix))) return true;
  if (mirrorPaths.has(file)) return true;
  if (HARNESS_EXEMPT_PATHS.has(file)) return true;
  if (isCiWorkflowHarnessOnly(file)) return true;
  return HARNESS_EXEMPT_PREFIXES.some((prefix) => file.startsWith(prefix));
}

function scanPlacement(files: string[], mirrorPaths: Set<string>): Failure[] {
  const failures: Failure[] = [];
  const exemptionPath = readFlag('exemptions') || 'validation/conformance/contracts/assurance_physical_domain_placement_exemptions.json';
  const exemptionPayload: any = readJson(exemptionPath);
  const exemptions = Array.isArray(exemptionPayload.exemptions) ? exemptionPayload.exemptions as Exemption[] : [];
  const today = new Date().toISOString().slice(0, 10);
  for (const exemption of exemptions) {
    if (!exemption.owner || !exemption.reason || !exemption.expires) {
      failures.push({
        id: 'invalid_physical_domain_exemption',
        path: exemption.path || exemption.path_prefix || exemptionPath,
        detail: 'exemption requires owner, reason, and expires',
      });
    }
    if (exemption.expires && exemption.expires < today) {
      failures.push({
        id: 'expired_physical_domain_exemption',
        path: exemption.path || exemption.path_prefix || exemptionPath,
        detail: `exemption expired ${exemption.expires}`,
      });
    }
  }
  for (const file of files) {
    const category = governedCategory(file);
    if (!category) continue;
    if (isAllowedDefinitionPath(file, mirrorPaths)) continue;
    if (exemptions.some((row) => (row.path && file === normalizePath(row.path)) || (row.path_prefix && file.startsWith(normalizePath(row.path_prefix))))) continue;
    failures.push({
      id: 'definition_outside_physical_domain',
      path: file,
      category,
      detail: 'definition-shaped Assurance artifact must live under validation/** or observability/**, or be listed as a compatibility mirror / harness-only exemption',
    });
  }
  return failures;
}

function scanCommandRegistry(): Failure[] {
  const failures: Failure[] = [];
  const pkg: any = readJson('package.json');
  const scripts = pkg.scripts || {};
  const requiredScriptFragments: Record<string, string[]> = {
    'ops:assurance:placement:guard': [
      'validation/conformance/contracts/assurance_validation_registry.json',
      'validation/conformance/contracts/assurance_consumer_boundary_contract.json',
      'validation/scorecards/contracts/assurance_scorecard_derivation_contract.json',
    ],
    'ops:assurance:envelope:guard': [
      'observability/evidence_normalization/assurance_evidence_envelope.schema.json',
    ],
    'ops:assurance:scorecard-derivation:guard': [
      'validation/scorecards/contracts/assurance_scorecard_derivation_contract.json',
    ],
    'ops:assurance:shell-truth-leak:guard': [
      'validation/conformance/contracts/assurance_consumer_boundary_contract.json',
      'observability/source_coverage/assurance_observability_registry.json',
    ],
  };
  for (const [script, fragments] of Object.entries(requiredScriptFragments)) {
    const command = String(scripts[script] || '');
    for (const fragment of fragments) {
      if (!command.includes(fragment)) {
        failures.push({
          id: 'package_script_missing_canonical_definition_path',
          path: 'package.json',
          detail: `${script} missing ${fragment}`,
        });
      }
    }
  }

  const registry: any = readJson('tests/tooling/config/tooling_gate_registry.json');
  for (const script of Object.keys(requiredScriptFragments)) {
    const row = (registry.gates || registry)[script] as GateRow | undefined;
    const canonical = Array.isArray(row?.canonical_definition_paths)
      ? row!.canonical_definition_paths.map(String)
      : [];
    if (canonical.length === 0) {
      failures.push({
        id: 'tooling_gate_missing_canonical_definition_paths',
        path: 'tests/tooling/config/tooling_gate_registry.json',
        detail: `${script} missing canonical_definition_paths`,
      });
      continue;
    }
    for (const defPath of canonical) {
      const normalized = normalizePath(defPath);
      if (!CANONICAL_PREFIXES.some((prefix) => normalized.startsWith(prefix))) {
        failures.push({
          id: 'tooling_gate_definition_path_outside_physical_domain',
          path: 'tests/tooling/config/tooling_gate_registry.json',
          detail: `${script} references ${normalized}`,
        });
      }
    }
  }
  return failures;
}

function run() {
  const strict = process.argv.includes('--strict') || process.argv.includes('--strict=1');
  const injectViolation = readFlag('inject-violation');
  const exemptionPath = readFlag('exemptions') || 'validation/conformance/contracts/assurance_physical_domain_placement_exemptions.json';
  const mirrorPaths = collectMirrorPaths();
  const files = trackedFiles();
  if (injectViolation) files.push(normalizePath(injectViolation));
  const placementFailures = scanPlacement(files, mirrorPaths);
  const registryFailures = scanCommandRegistry();
  const failures = [...placementFailures, ...registryFailures];
  const payload = {
    ok: failures.length === 0,
    type: 'assurance_physical_domain_placement_guard',
    generated_at: new Date().toISOString(),
    strict,
    summary: {
      tracked_files_scanned: files.length,
      compatibility_mirror_paths: mirrorPaths.size,
      exemption_path: exemptionPath,
      placement_failures: placementFailures.length,
      registry_failures: registryFailures.length,
      injected_violation: injectViolation || null,
      failures: failures.length,
    },
    failures,
  };
  writeJson(OUT, payload);
  console.log(JSON.stringify(payload, null, 2));
  if (strict && failures.length) process.exit(1);
}

run();
