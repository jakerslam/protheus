#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'shell/socket/contract/shell_socket_contract.json';
const DEFAULT_GATEWAY_CONTRACT = 'client/runtime/config/gateway_ingress_egress_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_socket_contract_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_SOCKET_CONTRACT_GUARD_CURRENT.md';

type Projection = {
  name?: string;
  allowed_top_level_fields?: string[];
  max_response_bytes?: number;
  max_array_items?: number;
  requires_cursor?: boolean;
  requires_detail_refs?: boolean;
};

type Capability = {
  id?: string;
  route_class?: string;
  request_shape?: string;
  response_shape?: string;
  default_projection?: string;
  capability_or_lease_required?: boolean;
  audit_receipt_required?: boolean;
  nexus_checkpoint_required?: boolean;
  conduit_scrambler_posture?: string;
  owner_of_truth?: string;
  shell_may_hold?: string[];
  shell_must_not_hold?: string[];
};

type SocketContract = {
  type?: string;
  owner?: string;
  status?: string;
  policy_refs?: string[];
  axioms?: Record<string, boolean>;
  forbidden_default_payload_fields?: string[];
  default_payload_budget?: Record<string, number>;
  projection_types?: Projection[];
  capabilities?: Capability[];
};

type GatewayContract = {
  required_route_classes?: string[];
  forbidden_default_payload_fields?: string[];
};

type Violation = { kind: string; path: string; detail: string };

const REQUIRED_CAPABILITIES = [
  'get_runtime_status',
  'list_agents',
  'list_sessions',
  'get_message_window',
  'get_message_detail',
  'submit_input',
  'subscribe_events',
  'search',
  'submit_issue',
  'submit_approval_decision',
  'set_model',
  'set_git_tree',
  'submit_terminal_command',
];

const REQUIRED_PROJECTIONS = [
  'RuntimeStatusProjection',
  'AgentRosterProjection',
  'SessionListProjection',
  'MessageWindowProjection',
  'MessageDetailProjection',
  'ShellEventProjection',
  'BoundedSearchResults',
  'IngressAck',
];

const REQUIRED_AXIOMS = [
  'socket_is_contract_not_runtime',
  'socket_owns_no_authority',
  'socket_owns_no_canonical_state',
  'shell_plugs_call_gateway_only',
  'default_payloads_are_projections',
  'heavy_payloads_are_lazy_detail_refs',
];

const REQUIRED_FORBIDDEN_FIELDS = [
  'raw',
  'root',
  'full_state',
  'all_messages',
  'conversation_tree',
  'raw_tool_result',
  'trace_body',
  'plan_graph',
  'workflow_graph',
  'execution_observation',
  'policy_decision',
  'authorization_state',
];

const STRONG_POSTURE_CAPABILITIES = new Set([
  'get_message_detail',
  'submit_input',
  'submit_issue',
  'submit_approval_decision',
  'set_model',
  'set_git_tree',
  'submit_terminal_command',
]);

const FORBIDDEN_DEPENDENCY_TOKENS = [
  'Alpine',
  'Svelte',
  'localStorage',
  'sessionStorage',
  'browser_globals',
  'DOM APIs',
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

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function applyControlledViolation(contract: SocketContract): SocketContract {
  const copy = clone(contract);
  if (copy.axioms) copy.axioms.shell_plugs_call_gateway_only = false;
  copy.capabilities = (copy.capabilities || []).filter((capability) => capability.id !== 'submit_input');
  const projection = (copy.projection_types || []).find((row) => row.name === 'MessageWindowProjection');
  if (projection) projection.allowed_top_level_fields = [...(projection.allowed_top_level_fields || []), 'all_messages'];
  const detail = (copy.capabilities || []).find((capability) => capability.id === 'get_message_detail');
  if (detail) detail.conduit_scrambler_posture = 'standard_scrambler';
  return copy;
}

function validateMetadata(contract: SocketContract, contractPath: string, violations: Violation[]): void {
  if (contract.type !== 'shell_socket_contract') push(violations, 'wrong_contract_type', contractPath, 'Expected shell_socket_contract.');
  if (contract.owner !== 'shell.presentation_contract') push(violations, 'wrong_contract_owner', contractPath, 'Owner must be shell.presentation_contract.');
  if (!['contracted', 'enforced'].includes(cleanText(contract.status || '', 80))) {
    push(violations, 'wrong_contract_status', contractPath, 'Status must be contracted or enforced.');
  }
  for (const ref of contract.policy_refs || []) {
    if (!exists(ref)) push(violations, 'missing_policy_ref', ref, 'Shell Socket contract references a missing policy.');
  }
  for (const key of REQUIRED_AXIOMS) {
    if (contract.axioms?.[key] !== true) push(violations, 'missing_socket_axiom', contractPath, `${key} must be true.`);
  }
  const serialized = JSON.stringify(contract);
  for (const token of FORBIDDEN_DEPENDENCY_TOKENS) {
    if (serialized.includes(token)) push(violations, 'browser_framework_dependency_token', contractPath, `Contract must not depend on ${token}.`);
  }
}

function validateForbiddenFields(contract: SocketContract, gateway: GatewayContract, contractPath: string, violations: Violation[]): Set<string> {
  const forbidden = new Set([...(contract.forbidden_default_payload_fields || []), ...(gateway.forbidden_default_payload_fields || [])]);
  for (const field of REQUIRED_FORBIDDEN_FIELDS) {
    if (!forbidden.has(field)) push(violations, 'missing_forbidden_default_field', contractPath, `Missing forbidden default field ${field}.`);
  }
  return forbidden;
}

function validateBudget(contract: SocketContract, contractPath: string, violations: Violation[]): void {
  const budget = contract.default_payload_budget || {};
  const ceilings: Record<string, number> = {
    max_response_bytes: 65536,
    max_array_items: 100,
    max_object_depth: 4,
    max_string_chars: 12000,
    max_nested_collection_items: 20,
    max_top_level_fields: 32,
  };
  for (const [key, ceiling] of Object.entries(ceilings)) {
    const value = Number(budget[key]);
    if (!Number.isFinite(value) || value < 1 || value > ceiling) {
      push(violations, 'socket_payload_budget_invalid', contractPath, `${key} must be between 1 and ${ceiling}.`);
    }
  }
}

function validateProjections(contract: SocketContract, forbidden: Set<string>, contractPath: string, violations: Violation[]): void {
  const projections = new Map((contract.projection_types || []).map((projection) => [projection.name, projection]));
  for (const name of REQUIRED_PROJECTIONS) {
    const projection = projections.get(name);
    if (!projection) {
      push(violations, 'missing_projection_type', contractPath, `Missing projection ${name}.`);
      continue;
    }
    if (!(projection.allowed_top_level_fields || []).length) push(violations, 'projection_fields_missing', contractPath, `${name} must declare allowed_top_level_fields.`);
    for (const field of projection.allowed_top_level_fields || []) {
      if (forbidden.has(field)) push(violations, 'projection_allows_forbidden_field', contractPath, `${name} allows forbidden field ${field}.`);
    }
    if (!Number.isFinite(Number(projection.max_response_bytes)) || Number(projection.max_response_bytes) > 65536) {
      push(violations, 'projection_budget_invalid', contractPath, `${name} must keep max_response_bytes <= 65536.`);
    }
  }
}

function validateCapabilities(contract: SocketContract, gateway: GatewayContract, contractPath: string, violations: Violation[]): void {
  const allowedRouteClasses = new Set(gateway.required_route_classes || []);
  const projections = new Set((contract.projection_types || []).map((projection) => projection.name));
  const capabilities = new Map((contract.capabilities || []).map((capability) => [capability.id, capability]));
  for (const id of REQUIRED_CAPABILITIES) {
    const capability = capabilities.get(id);
    if (!capability) {
      push(violations, 'missing_socket_capability', contractPath, `Missing capability ${id}.`);
      continue;
    }
    for (const field of ['route_class', 'request_shape', 'response_shape', 'default_projection', 'owner_of_truth'] as const) {
      if (!cleanText(capability[field] || '', 200)) push(violations, 'socket_capability_field_missing', contractPath, `${id} missing ${field}.`);
    }
    if (!allowedRouteClasses.has(cleanText(capability.route_class || '', 100))) {
      push(violations, 'socket_capability_unknown_route_class', contractPath, `${id} uses unknown route class ${capability.route_class}.`);
    }
    if (!projections.has(capability.default_projection)) {
      push(violations, 'socket_capability_unknown_projection', contractPath, `${id} references unknown projection ${capability.default_projection}.`);
    }
    for (const [key, expected] of [
      ['capability_or_lease_required', true],
      ['audit_receipt_required', true],
      ['nexus_checkpoint_required', true],
    ] as const) {
      if (capability[key] !== expected) push(violations, 'socket_capability_missing_boundary_requirement', contractPath, `${id} must set ${key}=true.`);
    }
    if (STRONG_POSTURE_CAPABILITIES.has(id) && capability.conduit_scrambler_posture !== 'strong_scrambler') {
      push(violations, 'socket_sensitive_capability_not_strong_scrambler', contractPath, `${id} must use strong_scrambler.`);
    }
    if (!(capability.shell_may_hold || []).length || !(capability.shell_must_not_hold || []).length) {
      push(violations, 'socket_capability_hold_contract_missing', contractPath, `${id} must declare shell_may_hold and shell_must_not_hold.`);
    }
  }
}

function renderMarkdown(report: any): string {
  const lines = [
    '# Shell Socket Contract Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    `capabilities: ${report.capability_count}`,
    `projections: ${report.projection_count}`,
    `violations: ${report.violations.length}`,
    '',
    '## Violations',
  ];
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations) lines.push(`- ${violation.kind} at \`${violation.path}\`: ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
const strict = common.strict;
const contractPath = cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT, 600);
const gatewayPath = cleanText(readFlag(argv, 'gateway-contract') || DEFAULT_GATEWAY_CONTRACT, 600);
const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);

const baseContract = readJson<SocketContract>(contractPath);
const contract = includeControlledViolation ? applyControlledViolation(baseContract) : baseContract;
const gateway = readJson<GatewayContract>(gatewayPath);
const violations: Violation[] = [];
validateMetadata(contract, contractPath, violations);
const forbidden = validateForbiddenFields(contract, gateway, contractPath, violations);
validateBudget(contract, contractPath, violations);
validateProjections(contract, forbidden, contractPath, violations);
validateCapabilities(contract, gateway, contractPath, violations);

const report = {
  ok: violations.length === 0,
  type: 'shell_socket_contract_guard',
  revision: currentRevision(ROOT),
  contract_path: contractPath,
  gateway_contract_path: gatewayPath,
  capability_count: contract.capabilities?.length || 0,
  projection_count: contract.projection_types?.length || 0,
  violations,
};

writeTextArtifact(outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
