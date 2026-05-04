#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/orchestration_canonical_name_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/ORCHESTRATION_CANONICAL_NAME_GUARD_CURRENT.md';
const REJECTED_TERM = /\bTower\b|\btower(?:[-_:][a-z0-9]+)?\b/;
const ALLOWED_CONTEXT = /(not canonical|not a canonical|non-canonical|historical|metaphor|rejected|not as an owning|not as a subsystem|informal)/i;

type Violation = { kind: string; path: string; detail: string; line?: number };

const STRICT_FILES = [
  'package.json',
  'tests/tooling/config/tooling_gate_registry.json',
  'client/runtime/config/system_map_registry.json',
  'docs/client/architecture/SYSTEM_MAP.md',
  'validation/conformance/contracts/domain_virtual_repo_manifest.json',
  'validation/conformance/contracts/orchestration_path_transition_register.json',
  'validation/conformance/contracts/shell_cognition_burn_down_register.json',
  'validation/domain_manifest.json',
  'observability/domain_manifest.json',
];

const POLICY_FILES = [
  'README.md',
  'ARCHITECTURE.md',
  'docs/workspace/orchestration_ownership_policy.md',
  'docs/workspace/codex_enforcer.md',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readIfExists(relPath: string): string | null {
  const target = abs(relPath);
  if (!fs.existsSync(target)) return null;
  return fs.readFileSync(target, 'utf8');
}

function checkStrictFile(relPath: string, violations: Violation[]): void {
  const source = readIfExists(relPath);
  if (source == null) return;
  const lines = source.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    if (REJECTED_TERM.test(lines[index])) {
      violations.push({
        kind: 'rejected_coordination_name_in_active_surface',
        path: relPath,
        line: index + 1,
        detail: 'Active maps, manifests, package scripts, and gate registries must use Orchestration Control Plane, not the rejected Tower metaphor.',
      });
    }
  }
}

function checkPolicyFile(relPath: string, violations: Violation[]): void {
  const source = readIfExists(relPath);
  if (source == null) return;
  const lines = source.split(/\r?\n/);
  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    if (REJECTED_TERM.test(line) && !ALLOWED_CONTEXT.test(line)) {
      violations.push({
        kind: 'rejected_coordination_name_without_historical_context',
        path: relPath,
        line: index + 1,
        detail: 'Policy docs may mention Tower only when explicitly marking it as rejected, historical, informal, or metaphor-only.',
      });
    }
  }
}

function renderMarkdown(report: any): string {
  const lines = [
    '# Orchestration Canonical Name Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    `violations: ${report.violations.length}`,
    '',
    '## Violations',
  ];
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations) {
    lines.push(`- ${violation.kind} at \`${violation.path}${violation.line ? `:${violation.line}` : ''}\`: ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const args = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
const outJson = cleanText(readFlag(argv, 'out-json') || args.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const violations: Violation[] = [];

for (const relPath of STRICT_FILES) checkStrictFile(relPath, violations);
for (const relPath of POLICY_FILES) checkPolicyFile(relPath, violations);
if (includeControlledViolation) {
  violations.push({
    kind: 'controlled_rejected_coordination_name_violation',
    path: '__controlled_violation__',
    detail: 'Controlled violation injected to prove the guard fails closed.',
  });
}

const report = {
  ok: violations.length === 0,
  type: 'orchestration_canonical_name_guard',
  revision: currentRevision(ROOT),
  rejected_term: 'Tower',
  canonical_name: 'Orchestration Control Plane',
  strict_files: STRICT_FILES,
  policy_files: POLICY_FILES,
  violations,
};

writeTextArtifact(outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict: args.strict, ok: report.ok });
