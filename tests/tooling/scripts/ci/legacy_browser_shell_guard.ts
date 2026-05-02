#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_MANIFEST = 'shell/legacy/legacy_browser_shell_manifest.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/legacy_browser_shell_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/LEGACY_BROWSER_SHELL_GUARD_CURRENT.md';

type LegacyManifest = {
  type?: string;
  name?: string;
  status?: string;
  canonical?: boolean;
  may_receive_new_features?: boolean;
  may_bypass_gateway?: boolean;
  may_claim_socket_parity?: boolean;
  legacy_dashboard_host?: string;
  canonical_shell_socket_gateway_host?: string;
  socket_contract?: string;
  gateway_route_contract?: string;
  legacy_paths?: string[];
  allowed_work?: string[];
  forbidden_work?: string[];
  required_axioms?: Record<string, boolean>;
  retirement_condition?: string;
};

type Violation = { kind: string; path: string; detail: string };

const REQUIRED_ALLOWED_WORK = ['critical_fixes', 'parity_bridge', 'retirement_support'];
const REQUIRED_FORBIDDEN_WORK = [
  'new_features',
  'authority_migration_into_legacy',
  'full_state_cache_expansion',
  'direct_gateway_bypass',
  'canonical_shell_socket_proof',
];
const REQUIRED_AXIOMS = [
  'legacy_dashboard_is_shell_1_compatibility_only',
  'shell_socket_lives_under_shell_socket',
  'gateway_remains_only_external_authority_boundary',
  'shell_2_proof_targets_gateway_backend_5173_not_legacy_4173',
  'legacy_bugfixes_must_not_expand_authority',
  'legacy_work_must_not_block_socket_first_shell_2',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readJson<T>(relPath: string): T {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8')) as T;
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function requireFile(relPath: string | undefined, field: string, manifestPath: string, violations: Violation[]): void {
  const clean = cleanText(relPath || '', 260);
  if (!clean) {
    push(violations, 'missing_manifest_path', manifestPath, `${field} must be set.`);
    return;
  }
  if (!fs.existsSync(abs(clean))) push(violations, 'missing_manifest_target', clean, `${field} points to a missing file.`);
}

function applyControlledViolation(manifest: LegacyManifest): LegacyManifest {
  const copy = clone(manifest);
  copy.canonical = true;
  copy.may_bypass_gateway = true;
  copy.may_receive_new_features = true;
  copy.may_claim_socket_parity = true;
  copy.allowed_work = (copy.allowed_work || []).filter((item) => item !== 'critical_fixes');
  if (copy.required_axioms) copy.required_axioms.shell_2_proof_targets_gateway_backend_5173_not_legacy_4173 = false;
  return copy;
}

function validateManifest(manifest: LegacyManifest, manifestPath: string): Violation[] {
  const violations: Violation[] = [];
  if (manifest.type !== 'legacy_browser_shell_manifest') push(violations, 'wrong_manifest_type', manifestPath, 'Expected legacy_browser_shell_manifest.');
  if (manifest.name !== 'LegacyBrowserShellPlug') push(violations, 'wrong_legacy_shell_name', manifestPath, 'Legacy shell plug must be named LegacyBrowserShellPlug.');
  if (manifest.status !== 'legacy') push(violations, 'wrong_legacy_shell_status', manifestPath, 'Legacy shell plug status must be legacy.');
  if (manifest.canonical !== false) push(violations, 'legacy_shell_marked_canonical', manifestPath, 'Legacy shell plug must never be canonical.');
  if (manifest.may_receive_new_features !== false) push(violations, 'legacy_shell_allows_new_features', manifestPath, 'Legacy shell plug may receive critical fixes only, not new features.');
  if (manifest.may_bypass_gateway !== false) push(violations, 'legacy_shell_allows_gateway_bypass', manifestPath, 'Legacy shell plug must not bypass Gateway.');
  if (manifest.may_claim_socket_parity !== false) push(violations, 'legacy_shell_claims_socket_parity', manifestPath, 'Legacy shell plug must not satisfy Shell Socket parity.');
  if (manifest.legacy_dashboard_host !== 'http://127.0.0.1:4173') push(violations, 'legacy_host_mismatch', manifestPath, 'Legacy browser host must be documented as 4173.');
  if (manifest.canonical_shell_socket_gateway_host !== 'http://127.0.0.1:5173') {
    push(violations, 'socket_gateway_host_mismatch', manifestPath, 'Canonical Shell Socket live probe target must be 5173.');
  }
  requireFile(manifest.socket_contract, 'socket_contract', manifestPath, violations);
  requireFile(manifest.gateway_route_contract, 'gateway_route_contract', manifestPath, violations);

  const legacyPaths = manifest.legacy_paths || [];
  if (!legacyPaths.some((entry) => entry.startsWith('client/runtime/systems/ui/'))) {
    push(violations, 'legacy_paths_missing_dashboard_surface', manifestPath, 'Legacy manifest must name the legacy UI surface.');
  }
  for (const item of REQUIRED_ALLOWED_WORK) {
    if (!(manifest.allowed_work || []).includes(item)) push(violations, 'missing_allowed_legacy_work', manifestPath, `Missing allowed work ${item}.`);
  }
  for (const item of REQUIRED_FORBIDDEN_WORK) {
    if (!(manifest.forbidden_work || []).includes(item)) push(violations, 'missing_forbidden_legacy_work', manifestPath, `Missing forbidden work ${item}.`);
  }
  for (const axiom of REQUIRED_AXIOMS) {
    if (manifest.required_axioms?.[axiom] !== true) push(violations, 'missing_legacy_shell_axiom', manifestPath, `${axiom} must be true.`);
  }
  const retirement = cleanText(manifest.retirement_condition || '', 600);
  if (!retirement.includes('Retire LegacyBrowserShellPlug') || !retirement.includes('Gateway-only')) {
    push(violations, 'weak_retirement_condition', manifestPath, 'Retirement condition must require Gateway-only clean plug parity.');
  }
  return violations;
}

function markdown(manifestPath: string, violations: Violation[]): string {
  const lines = [
    '# Legacy Browser Shell Guard',
    '',
    `Manifest: \`${manifestPath}\``,
    `Pass: \`${violations.length === 0}\``,
    '',
    '## Result',
  ];
  if (violations.length === 0) {
    lines.push('- Legacy browser Shell 1.0 is quarantined as non-canonical compatibility.');
    lines.push('- `4173` is legacy dashboard host; `5173` is the Shell Socket Gateway/backend proof target.');
  } else {
    for (const violation of violations) lines.push(`- ${violation.kind}: ${violation.path} - ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

function main(): void {
  const argv = process.argv.slice(2);
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  const strict = common.strict;
  const manifestPath = cleanText(readFlag(argv, 'manifest') || DEFAULT_MANIFEST, 600);
  const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
  const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
  const controlled = parseBool(readFlag(argv, 'include-controlled-violation'), false);
  const manifest = controlled ? applyControlledViolation(readJson<LegacyManifest>(manifestPath)) : readJson<LegacyManifest>(manifestPath);
  const violations = validateManifest(manifest, manifestPath);
  const ok = violations.length === 0;
  const result = {
    ok,
    type: 'legacy_browser_shell_guard',
    revision: currentRevision(ROOT),
    manifest_path: manifestPath,
    controlled_violation: controlled,
    violations,
  };
  if (outMarkdown) writeTextArtifact(outMarkdown, markdown(manifestPath, violations));
  process.exitCode = emitStructuredResult(result, { outPath: outJson, strict, ok });
}

main();
