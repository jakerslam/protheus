#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'validation/conformance/contracts/gateway_external_surface_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/gateway_external_surface_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/GATEWAY_EXTERNAL_SURFACE_GUARD_CURRENT.md';

type EvidenceGate = {
  id: string;
  required_package_script?: boolean;
  required_tooling_registry?: boolean;
  proves?: string[];
};

type ExternalSurface = {
  id: string;
  domain: string;
  owner: string;
  status: string;
  activation_rule?: string;
  allowed_route_classes?: string[];
  forbidden_bypass?: string[];
  required_evidence_gates?: string[];
};

type Contract = {
  type?: string;
  schema_version?: number;
  owner?: string;
  status?: string;
  policy_refs?: string[];
  policy_doc_required_tokens?: string[];
  required_route_classes?: string[];
  global_rules?: Record<string, boolean>;
  required_forbidden_bypass_patterns?: string[];
  evidence_gates?: EvidenceGate[];
  external_surfaces?: ExternalSurface[];
};

type Violation = { kind: string; path: string; detail: string };

const REQUIRED_SURFACES = [
  'shell',
  'cli',
  'sdk',
  'issue_submission',
  'eval_submission',
  'plugin_external_agent',
  'future_app_mobile_tauri',
];

const REQUIRED_ROUTE_CLASSES = [
  'request_ingress',
  'event_output_egress',
  'health_status',
  'detail_fetch',
  'bounded_search_query',
];

const REQUIRED_GLOBAL_TRUE = [
  'gateway_only_external_ingress',
  'no_first_party_shell_exception',
  'issue_eval_submission_gateway_only',
  'bounded_default_payloads',
  'nexus_checkpoint_required',
  'no_direct_kernel_or_orchestration_external_surface',
  'detail_payloads_lazy_by_ref',
];

const REQUIRED_BYPASS_BLOCKS = [
  'direct_kernel_or_core_authority_call',
  'direct_orchestration_internal_import',
  'first_party_shell_exception',
  'raw_runtime_state_upload',
  'full_state_mirror',
];

const REQUIRED_EVIDENCE_GATES = [
  'ops:gateway:interface:guard',
  'ops:interface:payload-budget:guard',
  'ops:nexus:route-inventory:guard',
  'ops:conduit:scrambler-posture:guard',
  'ops:shell:projection:guard',
  'ops:shell:ui-message-contract:guard',
  'ops:shell:truth-leak:guard',
  'ops:eval:issue-authority:guard',
  'ops:gateway-boundary:guard',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function exists(relPath: string): boolean {
  return fs.existsSync(abs(relPath));
}

function readJson<T>(relPath: string): T {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8')) as T;
}

function cloneContract(contract: Contract): Contract {
  return JSON.parse(JSON.stringify(contract)) as Contract;
}

function applyControlledViolation(contract: Contract): Contract {
  const copy = cloneContract(contract);
  if (copy.global_rules) copy.global_rules.no_first_party_shell_exception = false;
  copy.external_surfaces = (copy.external_surfaces || []).filter((surface) => surface.id !== 'shell');
  const evalGate = (copy.evidence_gates || []).find((gate) => gate.id === 'ops:eval:issue-authority:guard');
  if (evalGate) evalGate.required_package_script = false;
  return copy;
}

function registryEntry(registry: any, id: string): any {
  return registry?.gates?.[id] || registry?.[id];
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function validatePolicyRefs(contract: Contract, contractPath: string, violations: Violation[]): void {
  for (const ref of contract.policy_refs || []) {
    if (!exists(ref)) push(violations, 'missing_policy_ref', ref, 'Gateway external surface contract references a missing policy.');
  }
  const gatewayPolicy = contract.policy_refs?.find((ref) => ref.endsWith('gateway_ingress_egress_policy.md'));
  if (!gatewayPolicy || !exists(gatewayPolicy)) {
    push(violations, 'missing_gateway_policy_ref', contractPath, 'Contract must reference the canonical Gateway ingress/egress policy.');
    return;
  }
  const doc = fs.readFileSync(abs(gatewayPolicy), 'utf8');
  for (const token of contract.policy_doc_required_tokens || []) {
    if (!doc.includes(token)) push(violations, 'gateway_policy_missing_required_token', gatewayPolicy, `Missing token: ${token}`);
  }
}

function validateGlobalRules(contract: Contract, contractPath: string, violations: Violation[]): void {
  for (const key of REQUIRED_GLOBAL_TRUE) {
    if (contract.global_rules?.[key] !== true) push(violations, 'gateway_external_global_rule_missing', contractPath, `${key} must be true.`);
  }
  for (const routeClass of REQUIRED_ROUTE_CLASSES) {
    if (!(contract.required_route_classes || []).includes(routeClass)) {
      push(violations, 'gateway_external_route_class_missing', contractPath, `Required route class ${routeClass} is missing.`);
    }
  }
  for (const bypass of REQUIRED_BYPASS_BLOCKS) {
    if (!(contract.required_forbidden_bypass_patterns || []).includes(bypass)) {
      push(violations, 'gateway_external_bypass_block_missing', contractPath, `Required bypass block ${bypass} is missing.`);
    }
  }
}

function validateEvidenceGates(contract: Contract, contractPath: string, violations: Violation[]): void {
  const pkg = readJson<any>('package.json');
  const registry = readJson<any>('tests/tooling/config/tooling_gate_registry.json');
  const gateRows = new Map((contract.evidence_gates || []).map((gate) => [gate.id, gate]));
  for (const id of REQUIRED_EVIDENCE_GATES) {
    const gate = gateRows.get(id);
    if (!gate) {
      push(violations, 'gateway_external_evidence_gate_missing', contractPath, `Missing evidence gate ${id}.`);
      continue;
    }
    if (gate.required_package_script !== true) {
      push(violations, 'gateway_external_evidence_gate_not_required', contractPath, `${id} must require a package script.`);
    }
    if (!pkg.scripts?.[id]) push(violations, 'missing_package_script_for_evidence_gate', 'package.json', `${id} is required by Gateway external surface coverage.`);
    if (gate.required_tooling_registry && !registryEntry(registry, id)) {
      push(violations, 'missing_tooling_registry_for_evidence_gate', 'tests/tooling/config/tooling_gate_registry.json', `${id} is required in tooling registry.`);
    }
    if (!(gate.proves || []).length) push(violations, 'evidence_gate_missing_proof_claims', contractPath, `${id} must declare proof claims.`);
  }
  if (!pkg.scripts?.['ops:gateway:external-surface:guard']?.includes('gateway_external_surface_guard.ts')) {
    push(violations, 'missing_self_package_script', 'package.json', 'ops:gateway:external-surface:guard must execute gateway_external_surface_guard.ts.');
  }
  if (!registryEntry(registry, 'ops:gateway:external-surface:guard')) {
    push(violations, 'missing_self_tooling_registry_row', 'tests/tooling/config/tooling_gate_registry.json', 'ops:gateway:external-surface:guard must be registered.');
  }
}

function validateSurfaces(contract: Contract, contractPath: string, violations: Violation[]): void {
  const evidenceIds = new Set((contract.evidence_gates || []).map((gate) => gate.id));
  const routeClasses = new Set(contract.required_route_classes || []);
  const surfaces = new Map((contract.external_surfaces || []).map((surface) => [surface.id, surface]));
  for (const id of REQUIRED_SURFACES) {
    const surface = surfaces.get(id);
    if (!surface) {
      push(violations, 'gateway_external_surface_missing', contractPath, `Missing external surface ${id}.`);
      continue;
    }
    for (const field of ['domain', 'owner', 'status'] as const) {
      if (!cleanText(String(surface[field] || ''), 300)) push(violations, 'gateway_external_surface_field_missing', contractPath, `${id} missing ${field}.`);
    }
    if (surface.status === 'reserved' && !surface.activation_rule) {
      push(violations, 'reserved_surface_missing_activation_rule', contractPath, `${id} must declare activation_rule.`);
    }
    if (!(surface.allowed_route_classes || []).length) {
      push(violations, 'gateway_external_surface_route_classes_missing', contractPath, `${id} must declare allowed_route_classes.`);
    }
    for (const routeClass of surface.allowed_route_classes || []) {
      if (!routeClasses.has(routeClass)) push(violations, 'gateway_external_surface_unknown_route_class', contractPath, `${id} uses unknown route class ${routeClass}.`);
    }
    for (const bypass of REQUIRED_BYPASS_BLOCKS) {
      if (!(surface.forbidden_bypass || []).includes(bypass)) push(violations, 'gateway_external_surface_bypass_gap', contractPath, `${id} does not forbid ${bypass}.`);
    }
    for (const gate of surface.required_evidence_gates || []) {
      if (!evidenceIds.has(gate)) push(violations, 'gateway_external_surface_unknown_evidence_gate', contractPath, `${id} references unknown evidence gate ${gate}.`);
    }
    if (!(surface.required_evidence_gates || []).includes('ops:gateway:interface:guard')) {
      push(violations, 'gateway_external_surface_missing_gateway_interface_gate', contractPath, `${id} must require ops:gateway:interface:guard.`);
    }
    if (!(surface.required_evidence_gates || []).includes('ops:interface:payload-budget:guard')) {
      push(violations, 'gateway_external_surface_missing_payload_budget_gate', contractPath, `${id} must require ops:interface:payload-budget:guard.`);
    }
  }
}

function renderMarkdown(report: any): string {
  const lines = [
    '# Gateway External Surface Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    `surfaces: ${report.surface_count}`,
    `evidence_gates: ${report.evidence_gate_count}`,
    `violations: ${report.violations.length}`,
    '',
    '## Violations',
  ];
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations) lines.push(`- ${violation.kind} at \`${violation.path}\`: ${violation.detail}`);
  lines.push('', '## Covered surfaces');
  for (const surface of report.covered_surfaces) {
    lines.push(`- ${surface.id}: ${surface.status}; owner=${surface.owner}; routes=${surface.allowed_route_classes.join(', ')}`);
  }
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
const strict = common.strict;
const contractPath = cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT, 600);
const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);

const baseContract = readJson<Contract>(contractPath);
const contract = includeControlledViolation ? applyControlledViolation(baseContract) : baseContract;
const violations: Violation[] = [];
if (contract.type !== 'gateway_external_surface_contract') push(violations, 'wrong_contract_type', contractPath, 'Expected gateway_external_surface_contract.');
if (contract.owner !== 'assurance.validation') push(violations, 'wrong_contract_owner', contractPath, 'Owner must be assurance.validation.');
if (contract.status !== 'enforced') push(violations, 'wrong_contract_status', contractPath, 'Status must be enforced.');
validatePolicyRefs(contract, contractPath, violations);
validateGlobalRules(contract, contractPath, violations);
validateEvidenceGates(contract, contractPath, violations);
validateSurfaces(contract, contractPath, violations);

const report = {
  ok: violations.length === 0,
  type: 'gateway_external_surface_guard',
  revision: currentRevision(ROOT),
  contract_path: contractPath,
  surface_count: contract.external_surfaces?.length || 0,
  evidence_gate_count: contract.evidence_gates?.length || 0,
  covered_surfaces: (contract.external_surfaces || []).map((surface) => ({
    id: surface.id,
    status: surface.status,
    owner: surface.owner,
    allowed_route_classes: surface.allowed_route_classes || [],
  })),
  violations,
};

writeTextArtifact(outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
