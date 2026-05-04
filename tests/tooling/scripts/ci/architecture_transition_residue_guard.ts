#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/architecture_transition_residue_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/ARCHITECTURE_TRANSITION_RESIDUE_GUARD_CURRENT.md';

type Violation = { kind: string; path: string; detail: string };

function abs(rel: string): string {
  return path.resolve(ROOT, rel);
}

function readJson(rel: string): any {
  return JSON.parse(fs.readFileSync(abs(rel), 'utf8'));
}

function exists(rel: string): boolean {
  return fs.existsSync(abs(rel));
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function validateDomainManifest(violations: Violation[]): void {
  const manifestPath = 'validation/conformance/contracts/domain_virtual_repo_manifest.json';
  const manifest = readJson(manifestPath);
  const domains = new Map((manifest.domains || []).map((domain: any) => [domain.id, domain]));
  for (const id of manifest.required_domains || []) {
    const domain = domains.get(id) as any;
    if (!domain) {
      push(violations, 'missing_virtual_repo_domain', manifestPath, `Missing required domain: ${id}`);
      continue;
    }
    for (const field of ['canonical_owner', 'paths', 'public_contracts', 'allowed_dependencies', 'forbidden_dependencies']) {
      if (domain[field] == null || (Array.isArray(domain[field]) && domain[field].length === 0)) {
        push(violations, 'incomplete_virtual_repo_domain', manifestPath, `${id} missing ${field}`);
      }
    }
    if (!(domain.paths || []).some((domainPath: string) => exists(domainPath.replace(/\/$/, '')))) {
      push(violations, 'virtual_repo_path_missing', manifestPath, `${id} has no existing implementation path.`);
    }
  }
}

function validateOrchestrationTransition(violations: Violation[]): void {
  const registerPath = 'validation/conformance/contracts/orchestration_path_transition_register.json';
  const register = readJson(registerPath);
  if (register.canonical_path !== 'orchestration/') {
    push(violations, 'wrong_orchestration_canonical_path', registerPath, 'Canonical Orchestration path must be orchestration/.');
  }
  const legacy = (register.legacy_paths || []).find((row: any) => row.path === 'surface/orchestration/');
  if (!legacy) {
    push(violations, 'missing_surface_orchestration_transition_row', registerPath, 'surface/orchestration/ must be explicitly governed while references are burned down.');
  } else if (!legacy.expires || legacy.expires < '2026-05-01') {
    push(violations, 'invalid_surface_orchestration_expiry', registerPath, 'surface/orchestration transition row needs a future expiry.');
  }
  if (exists('surface/orchestration')) {
    push(violations, 'retired_surface_orchestration_path_reintroduced', 'surface/orchestration/', 'Retired Orchestration compatibility path exists again.');
  }
  const activeFiles = ['package.json', 'tests/tooling/scripts/ci/shell_amputation_regression_guard.ts', 'tests/tooling/scripts/ci/shell_authority_config_guard.ts'];
  for (const relPath of activeFiles) {
    if (exists(relPath) && fs.readFileSync(abs(relPath), 'utf8').includes('surface/orchestration')) {
      push(violations, 'active_surface_orchestration_reference', relPath, 'Active command/guard still points at surface/orchestration.');
    }
  }
}

function validateShellCognitionBurnDown(violations: Violation[]): void {
  const registerPath = 'validation/conformance/contracts/shell_cognition_burn_down_register.json';
  const register = readJson(registerPath);
  const allowed = new Set(register.required_dispositions || []);
  const systems = register.systems || [];
  if (systems.length < 10) {
    push(violations, 'shell_cognition_burn_down_incomplete', registerPath, 'Expected all ten generated Shell Cognition subsystems to be classified.');
  }
  for (const row of systems) {
    if (!allowed.has(row.disposition)) push(violations, 'invalid_shell_cognition_disposition', registerPath, `${row.id} has invalid disposition.`);
    for (const field of ['target_domain', 'owner', 'deadline', 'reason']) {
      if (!row[field]) push(violations, 'incomplete_shell_cognition_row', registerPath, `${row.id} missing ${field}.`);
    }
    if (row.disposition !== 'presentation_local_only' && row.target_domain === 'shell') {
      push(violations, 'shell_cognition_still_shell_owned', registerPath, `${row.id} must not remain Shell-owned unless presentation-local.`);
    }
  }
}

function validateCommandWiring(violations: Violation[]): void {
  const pkg = readJson('package.json');
  const gates = readJson('tests/tooling/config/tooling_gate_registry.json').gates || {};
  const requiredScripts = {
    'ops:shell:amputation:guard': 'shell_amputation_regression_guard.ts',
    'ops:shell:long-chat-ram:guard': 'shell_long_chat_ram_regression_guard.ts',
    'ops:representation-collapse:report': 'representation_collapse_report.ts',
    'ops:orchestration:naming:guard': 'orchestration_canonical_name_guard.ts',
    'ops:gateway:external-surface:guard': 'gateway_external_surface_guard.ts',
    'ops:shell:runtime-payload-budget:guard': 'shell_runtime_payload_budget_guard.ts',
    'ops:shell:tiered-long-chat-heap:guard': 'shell_tiered_long_chat_heap_guard.ts',
    'ops:tooling:harness-only:guard': 'tooling_harness_only_guard.ts',
    'ops:architecture:transition-residue:guard': 'architecture_transition_residue_guard.ts'
  } as Record<string, string>;
  for (const [script, token] of Object.entries(requiredScripts)) {
    const command = pkg.scripts?.[script] || '';
    if (!command.includes(token)) push(violations, 'missing_or_wrong_package_script', 'package.json', `${script} must execute ${token}.`);
    if (!gates[script]) push(violations, 'missing_tooling_gate_registry_row', 'tests/tooling/config/tooling_gate_registry.json', `${script} missing from tooling gate registry.`);
  }
}

function renderMarkdown(report: any): string {
  const lines = [
    '# Architecture Transition Residue Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    `violations: ${report.violations.length}`,
    '',
    '## Violations'
  ];
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations) lines.push(`- ${violation.kind} at \`${violation.path}\`: ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

const args = parseStrictOutArgs(process.argv.slice(2), { strict: true, out: DEFAULT_OUT_JSON });
const outJson = cleanText(readFlag(process.argv.slice(2), 'out-json') || args.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(process.argv.slice(2), 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const violations: Violation[] = [];
validateDomainManifest(violations);
validateOrchestrationTransition(violations);
validateShellCognitionBurnDown(violations);
validateCommandWiring(violations);
const report = {
  ok: violations.length === 0,
  type: 'architecture_transition_residue_guard',
  revision: currentRevision(ROOT),
  checked_contracts: [
    'validation/conformance/contracts/domain_virtual_repo_manifest.json',
    'validation/conformance/contracts/orchestration_path_transition_register.json',
    'validation/conformance/contracts/shell_cognition_burn_down_register.json'
  ],
  violations
};
writeTextArtifact(outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict: args.strict, ok: report.ok });
