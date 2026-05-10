#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_SOCKET_CONTRACT = 'shell/socket/contract/shell_socket_contract.json';
const DEFAULT_ROUTE_CONTRACT = 'validation/conformance/contracts/shell_socket_gateway_contract.json';
const DEFAULT_GATEWAY_CONTRACT = 'client/runtime/config/gateway_ingress_egress_contract.json';
const DEFAULT_SHELL_SOCKET_IMPL = 'core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/shell_socket.rs';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_socket_gateway_route_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_SOCKET_GATEWAY_ROUTE_GUARD_CURRENT.md';

type Capability = {
  id?: string;
  route_class?: string;
  default_projection?: string;
  conduit_scrambler_posture?: string;
  owner_of_truth?: string;
};

type SocketContract = {
  type?: string;
  capabilities?: Capability[];
  projection_types?: Array<{ name?: string }>;
};

type RouteMapping = {
  capability_id?: string;
  gateway_route_id?: string;
  canonical_route_pattern?: string;
  route_class?: string;
  source_domain?: string;
  target_domain?: string;
  owner_of_truth?: string;
  backend_owner_path?: string;
  payload_budget_ref?: string;
  default_response_projection?: string;
  capability_or_lease_required?: boolean;
  audit_receipt_required?: boolean;
  nexus_checkpoint_required?: boolean;
  conduit_scrambler_posture?: string;
  detail_ref_behavior?: string;
  implementation_status?: string;
};

type RouteContract = {
  type?: string;
  owner?: string;
  status?: string;
  source_contract?: string;
  gateway_only_invariant?: Record<string, boolean>;
  allowed_route_classes?: string[];
  prohibited_route_shapes?: string[];
  required_gateway_enforcement?: string[];
  route_mappings?: RouteMapping[];
  missing_route_policy?: Record<string, boolean>;
};

type GatewayContract = {
  required_route_classes?: string[];
  forbidden_route_shapes?: string[];
};

type Violation = { kind: string; path: string; detail: string };

const REQUIRED_INVARIANTS = [
  'shell_plug_to_gateway_only',
  'no_shell_plug_to_kernel_direct',
  'no_shell_plug_to_orchestration_direct',
  'no_shell_plug_to_assurance_direct',
  'no_stateful_shell_socket_runtime',
  'gateway_remains_external_ambiguity_firewall',
];

const REQUIRED_GATEWAY_ENFORCEMENT = [
  'authentication',
  'authorization',
  'payload_limits',
  'projection_shaping',
  'rate_limits',
  'capability_checks',
  'mutation_approval_checks',
  'audit_receipts',
  'route_policy',
  'shell_isolation',
  'lazy_detail_access',
  'issue_eval_submission_controls',
];

const REQUIRED_MISSING_ROUTE_POLICY = [
  'missing_route_is_not_shell_workaround_permission',
  'missing_route_must_create_gateway_backed_task',
  'shell_plug_must_not_call_backend_owner_directly',
  'legacy_dashboard_compatibility_does_not_satisfy_socket_parity',
];

const HTTP_PATTERN = /^(GET|POST|PUT|PATCH|DELETE) \/api\/shell-socket(\/|\?|$)/;
const SHELL_SOCKET_SEARCH_FN_PATTERN = /fn\s+shell_socket_search\b[\s\S]*?\n}\n\nfn\s+shell_socket_ingress_ack\b/;

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function exists(relPath: string): boolean {
  return fs.existsSync(abs(relPath));
}

function readJson<T>(relPath: string): T {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8')) as T;
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function applyControlledViolation(contract: RouteContract): RouteContract {
  const copy = clone(contract);
  copy.route_mappings = (copy.route_mappings || []).filter((route) => route.capability_id !== 'submit_input');
  const detail = (copy.route_mappings || []).find((route) => route.capability_id === 'get_message_detail');
  if (detail) {
    detail.target_domain = 'orchestration.direct';
    detail.canonical_route_pattern = 'GET /api/internal/orchestration/details/{detail_ref}';
    detail.conduit_scrambler_posture = 'standard_scrambler';
  }
  if (copy.gateway_only_invariant) copy.gateway_only_invariant.shell_plug_to_gateway_only = false;
  return copy;
}

function validateMetadata(routeContract: RouteContract, routePath: string, socketPath: string, violations: Violation[]): void {
  if (routeContract.type !== 'shell_socket_gateway_contract') push(violations, 'wrong_contract_type', routePath, 'Expected shell_socket_gateway_contract.');
  if (routeContract.owner !== 'assurance.validation') push(violations, 'wrong_contract_owner', routePath, 'Owner must be assurance.validation.');
  if (!['contracted', 'enforced'].includes(cleanText(routeContract.status || '', 80))) {
    push(violations, 'wrong_contract_status', routePath, 'Status must be contracted or enforced.');
  }
  if (routeContract.source_contract !== socketPath) {
    push(violations, 'source_contract_mismatch', routePath, `Expected source_contract=${socketPath}.`);
  }
  for (const key of REQUIRED_INVARIANTS) {
    if (routeContract.gateway_only_invariant?.[key] !== true) push(violations, 'missing_gateway_only_invariant', routePath, `${key} must be true.`);
  }
  for (const key of REQUIRED_MISSING_ROUTE_POLICY) {
    if (routeContract.missing_route_policy?.[key] !== true) push(violations, 'missing_route_policy_gap', routePath, `${key} must be true.`);
  }
}

function validateGatewayAlignment(routeContract: RouteContract, gateway: GatewayContract, routePath: string, violations: Violation[]): Set<string> {
  const allowed = new Set(routeContract.allowed_route_classes || []);
  for (const routeClass of gateway.required_route_classes || []) {
    if (!allowed.has(routeClass)) push(violations, 'missing_allowed_gateway_route_class', routePath, `Missing route class ${routeClass}.`);
  }
  for (const shape of gateway.forbidden_route_shapes || []) {
    if (!(routeContract.prohibited_route_shapes || []).includes(shape)) push(violations, 'missing_prohibited_route_shape', routePath, `Missing prohibited route shape ${shape}.`);
  }
  for (const enforcement of REQUIRED_GATEWAY_ENFORCEMENT) {
    if (!(routeContract.required_gateway_enforcement || []).includes(enforcement)) {
      push(violations, 'missing_gateway_enforcement_requirement', routePath, `Missing enforcement requirement ${enforcement}.`);
    }
  }
  return allowed;
}

function validateRouteCoverage(socket: SocketContract, routeContract: RouteContract, routePath: string, violations: Violation[]): void {
  const capabilities = new Map((socket.capabilities || []).map((capability) => [capability.id, capability]));
  const mappings = new Map((routeContract.route_mappings || []).map((mapping) => [mapping.capability_id, mapping]));
  for (const id of capabilities.keys()) {
    if (!mappings.has(id)) push(violations, 'missing_socket_gateway_route_mapping', routePath, `Missing route mapping for ${id}.`);
  }
  for (const id of mappings.keys()) {
    if (!capabilities.has(id)) push(violations, 'orphan_socket_gateway_route_mapping', routePath, `Route mapping ${id} has no socket capability.`);
  }
}

function validateRouteMapping(mapping: RouteMapping, capability: Capability, allowedRouteClasses: Set<string>, routePath: string, violations: Violation[]): void {
  const id = cleanText(mapping.capability_id || '', 160);
  if (!cleanText(mapping.gateway_route_id || '', 200).startsWith('shell_socket.')) {
    push(violations, 'invalid_gateway_route_id', routePath, `${id} gateway_route_id must start with shell_socket.`);
  }
  if (!HTTP_PATTERN.test(cleanText(mapping.canonical_route_pattern || '', 400))) {
    push(violations, 'invalid_shell_socket_route_pattern', routePath, `${id} must use a /api/shell-socket route pattern.`);
  }
  if (!allowedRouteClasses.has(cleanText(mapping.route_class || '', 100))) {
    push(violations, 'unknown_route_class', routePath, `${id} uses unknown route class ${mapping.route_class}.`);
  }
  if (mapping.route_class !== capability.route_class) {
    push(violations, 'route_class_mismatch', routePath, `${id} route_class must match socket capability route_class.`);
  }
  if (mapping.source_domain !== 'shell_plug') push(violations, 'invalid_source_domain', routePath, `${id} source_domain must be shell_plug.`);
  if (!cleanText(mapping.target_domain || '', 160).startsWith('gateway.')) {
    push(violations, 'invalid_target_domain', routePath, `${id} target_domain must start with gateway.`);
  }
  if (mapping.owner_of_truth !== capability.owner_of_truth) {
    push(violations, 'owner_of_truth_mismatch', routePath, `${id} owner_of_truth must match socket capability.`);
  }
  const backendPath = cleanText(mapping.backend_owner_path || '', 500);
  if (!backendPath || backendPath.includes('client/runtime/systems/ui') || backendPath.includes('infring_static')) {
    push(violations, 'invalid_backend_owner_path', routePath, `${id} backend owner must not be browser shell assets.`);
  }
  if (!cleanText(mapping.payload_budget_ref || '', 300).includes(cleanText(capability.default_projection || '', 160))) {
    push(violations, 'payload_budget_projection_mismatch', routePath, `${id} payload budget ref must name ${capability.default_projection}.`);
  }
  if (mapping.default_response_projection !== capability.default_projection) {
    push(violations, 'default_projection_mismatch', routePath, `${id} default response projection must match socket capability.`);
  }
  for (const [key, expected] of [
    ['capability_or_lease_required', true],
    ['audit_receipt_required', true],
    ['nexus_checkpoint_required', true],
  ] as const) {
    if (mapping[key] !== expected) push(violations, 'route_mapping_missing_boundary_requirement', routePath, `${id} must set ${key}=true.`);
  }
  if (mapping.conduit_scrambler_posture !== capability.conduit_scrambler_posture) {
    push(violations, 'scrambler_posture_mismatch', routePath, `${id} Conduit posture must match socket capability.`);
  }
  if (!cleanText(mapping.detail_ref_behavior || '', 300)) push(violations, 'detail_ref_behavior_missing', routePath, `${id} must declare detail_ref_behavior.`);
}

function validateRoutes(socket: SocketContract, routeContract: RouteContract, allowedRouteClasses: Set<string>, routePath: string, violations: Violation[]): void {
  const capabilities = new Map((socket.capabilities || []).map((capability) => [capability.id, capability]));
  for (const mapping of routeContract.route_mappings || []) {
    const capability = capabilities.get(mapping.capability_id);
    if (!capability) continue;
    validateRouteMapping(mapping, capability, allowedRouteClasses, routePath, violations);
  }
}

function validateShellSocketImplementation(implPath: string, violations: Violation[]): void {
  if (!exists(implPath)) {
    push(violations, 'missing_shell_socket_impl', implPath, 'Missing Shell Socket Gateway implementation file.');
    return;
  }
  const source = readText(implPath);
  const searchFn = source.match(SHELL_SOCKET_SEARCH_FN_PATTERN)?.[0] || '';
  if (!searchFn) {
    push(violations, 'missing_shell_socket_search_impl', implPath, 'Missing shell_socket_search implementation.');
    return;
  }
  if (searchFn.includes('dashboard_internal_search::search_conversations') || searchFn.includes('search_conversations(')) {
    push(
      violations,
      'shell_socket_search_uses_legacy_full_conversation_search',
      implPath,
      'Shell Socket search must use bounded projection indexes and must not call the legacy full conversation search path.',
    );
  }
  if (!searchFn.includes('build_sidebar_agent_roster_fast') || !searchFn.includes('compact_sidebar_roster_rows')) {
    push(
      violations,
      'shell_socket_search_not_projection_bounded',
      implPath,
      'Shell Socket search must stay bound to the compact roster projection path until a dedicated bounded index owner exists.',
    );
  }
}

function renderMarkdown(report: any): string {
  const lines = [
    '# Shell Socket Gateway Route Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    `capabilities: ${report.capability_count}`,
    `route_mappings: ${report.route_mapping_count}`,
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
const socketPath = cleanText(readFlag(argv, 'socket-contract') || DEFAULT_SOCKET_CONTRACT, 600);
const routePath = cleanText(readFlag(argv, 'route-contract') || DEFAULT_ROUTE_CONTRACT, 600);
const gatewayPath = cleanText(readFlag(argv, 'gateway-contract') || DEFAULT_GATEWAY_CONTRACT, 600);
const implPath = cleanText(readFlag(argv, 'shell-socket-impl') || DEFAULT_SHELL_SOCKET_IMPL, 600);
const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);

if (!exists(socketPath)) throw new Error(`Missing socket contract: ${socketPath}`);
if (!exists(routePath)) throw new Error(`Missing route contract: ${routePath}`);
if (!exists(gatewayPath)) throw new Error(`Missing Gateway contract: ${gatewayPath}`);

const socket = readJson<SocketContract>(socketPath);
const baseRouteContract = readJson<RouteContract>(routePath);
const routeContract = includeControlledViolation ? applyControlledViolation(baseRouteContract) : baseRouteContract;
const gateway = readJson<GatewayContract>(gatewayPath);
const violations: Violation[] = [];
if (socket.type !== 'shell_socket_contract') push(violations, 'wrong_socket_contract_type', socketPath, 'Expected shell_socket_contract.');
validateMetadata(routeContract, routePath, socketPath, violations);
const allowedRouteClasses = validateGatewayAlignment(routeContract, gateway, routePath, violations);
validateRouteCoverage(socket, routeContract, routePath, violations);
validateRoutes(socket, routeContract, allowedRouteClasses, routePath, violations);
validateShellSocketImplementation(implPath, violations);

const report = {
  ok: violations.length === 0,
  type: 'shell_socket_gateway_route_guard',
  revision: currentRevision(ROOT),
  socket_contract_path: socketPath,
  route_contract_path: routePath,
  gateway_contract_path: gatewayPath,
  shell_socket_impl_path: implPath,
  capability_count: socket.capabilities?.length || 0,
  route_mapping_count: routeContract.route_mappings?.length || 0,
  violations,
};

writeTextArtifact(outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
