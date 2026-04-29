#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'client/runtime/config/interface_payload_budget_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/interface_payload_budget_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/INTERFACE_PAYLOAD_BUDGET_GUARD_CURRENT.md';

type GlobalBudget = {
  max_response_bytes: number;
  max_array_items: number;
  max_object_depth: number;
  max_string_chars: number;
  max_nested_collection_items: number;
  max_top_level_fields: number;
  max_endpoint_patterns: number;
};

type EndpointBudget = {
  name: string;
  surface: string;
  route_class: string;
  payload_kind: string;
  default_payload: boolean;
  endpoint_patterns?: string[];
  max_response_bytes: number;
  max_array_items: number;
  max_object_depth: number;
  max_string_chars: number;
  max_nested_collection_items: number;
  max_top_level_fields: number;
  allowed_top_level_fields?: string[];
  requires_cursor: boolean;
  cursor_fields?: string[];
  requires_detail_refs: boolean;
  required_ref_fields?: string[];
  overflow_strategy: string;
  capability_or_lease_required: boolean;
  audit_receipt: boolean;
  nexus_checkpoint: boolean;
};

type Contract = {
  version?: string;
  policy_doc_path: string;
  gateway_contract_path: string;
  shell_message_contract_path: string;
  related_policy_paths?: string[];
  policy_doc_required_tokens?: string[];
  global_default_payload_budget?: GlobalBudget;
  required_endpoint_budgets?: string[];
  forbidden_default_payload_fields?: string[];
  default_endpoint_budgets?: EndpointBudget[];
};

type GatewayContract = {
  required_route_classes?: string[];
  route_classes?: Array<{ name: string }>;
  forbidden_default_payload_fields?: string[];
};

type ShellMessageContract = {
  prohibited_default_fields?: string[];
};

type Args = {
  strict: boolean;
  contractPath: string;
  outJson: string;
  outMarkdown: string;
  includeControlledViolation: boolean;
};

type Violation = {
  kind: string;
  path?: string;
  detail: string;
};

const REQUIRED_ENDPOINT_BUDGETS = [
  'health_status_projection',
  'agent_sidebar_projection',
  'session_list_projection',
  'chat_message_window_projection',
  'runtime_event_projection',
];

const RAW_FIELD_PATTERN = /(^|_)(raw|root|full|all|trace|decision_trace|plan_graph|workflow_graph|payload|observation|authorization|policy_decision|tool_input|tool_result|artifact_body|file_output|folder_tree|eval_payload)(_|$)/;
const CURSOR_ROUTE_CLASSES = new Set(['event_output_egress', 'bounded_search_query']);
const ALLOWED_OVERFLOW = new Set(['omit', 'cursor_or_detail_ref']);

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function exists(relPath: string): boolean {
  return fs.existsSync(abs(relPath));
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function readJson<T>(relPath: string): T {
  return JSON.parse(readText(relPath)) as T;
}

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    contractPath: cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT, 600),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600),
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
  };
}

function cloneContract(contract: Contract): Contract {
  return JSON.parse(JSON.stringify(contract)) as Contract;
}

function duplicateValues(rows: string[]): string[] {
  const counts = new Map<string, number>();
  for (const row of rows) counts.set(row, (counts.get(row) || 0) + 1);
  return Array.from(counts.entries()).filter(([, count]) => count > 1).map(([row]) => row);
}

function numberInRange(value: number, min: number, max: number): boolean {
  return Number.isFinite(value) && value >= min && value <= max;
}

function mergedForbiddenFields(contract: Contract, gateway: GatewayContract, shell: ShellMessageContract): Set<string> {
  return new Set([
    ...(contract.forbidden_default_payload_fields || []),
    ...(gateway.forbidden_default_payload_fields || []),
    ...(shell.prohibited_default_fields || []),
  ]);
}

function applyControlledViolation(contract: Contract): Contract {
  const copy = cloneContract(contract);
  copy.required_endpoint_budgets = (copy.required_endpoint_budgets || []).filter((name) => name !== 'chat_message_window_projection');
  const chat = (copy.default_endpoint_budgets || []).find((row) => row.name === 'chat_message_window_projection');
  if (chat) {
    chat.max_response_bytes = 10_000_000;
    chat.max_array_items = 5000;
    chat.max_object_depth = 20;
    chat.allowed_top_level_fields = [...(chat.allowed_top_level_fields || []), 'all_messages', 'raw_tool_result', 'decision_trace'];
    chat.requires_cursor = false;
    chat.audit_receipt = false;
    chat.overflow_strategy = 'return_all';
  }
  return copy;
}

function validateDocs(contract: Contract, contractPath: string, violations: Violation[]): void {
  const docs = [
    contract.policy_doc_path,
    contract.gateway_contract_path,
    contract.shell_message_contract_path,
    ...(contract.related_policy_paths || []),
  ];
  for (const docPath of docs) {
    if (!docPath || !exists(docPath)) {
      violations.push({ kind: 'missing_interface_payload_budget_reference', path: docPath || contractPath, detail: 'Required policy or contract reference is missing.' });
    }
  }
  if (contract.policy_doc_path && exists(contract.policy_doc_path)) {
    const doc = readText(contract.policy_doc_path);
    for (const token of contract.policy_doc_required_tokens || []) {
      if (!doc.includes(token)) {
        violations.push({ kind: 'interface_payload_budget_doc_missing_token', path: contract.policy_doc_path, detail: `Missing required policy token: ${token}` });
      }
    }
  }
}

function validateGlobalBudget(contract: Contract, violations: Violation[]): void {
  const budget = contract.global_default_payload_budget;
  if (!budget) {
    violations.push({ kind: 'interface_payload_global_budget_missing', detail: 'Global default payload budget is missing.' });
    return;
  }
  const checks: Array<[keyof GlobalBudget, number, number]> = [
    ['max_response_bytes', 1, 65536],
    ['max_array_items', 1, 100],
    ['max_object_depth', 1, 4],
    ['max_string_chars', 1, 12000],
    ['max_nested_collection_items', 1, 20],
    ['max_top_level_fields', 1, 32],
    ['max_endpoint_patterns', 1, 8],
  ];
  for (const [key, min, max] of checks) {
    if (!numberInRange(Number(budget[key]), min, max)) {
      violations.push({ kind: 'interface_payload_global_budget_invalid', detail: `${key} must be between ${min} and ${max}.` });
    }
  }
}

function validateEndpointList(contract: Contract, gateway: GatewayContract, violations: Violation[]): void {
  const required = contract.required_endpoint_budgets || [];
  const endpoints = contract.default_endpoint_budgets || [];
  const names = endpoints.map((row) => row.name);
  for (const requiredName of REQUIRED_ENDPOINT_BUDGETS) {
    if (!required.includes(requiredName)) violations.push({ kind: 'interface_payload_required_endpoint_not_declared', detail: `Required endpoint budget list is missing ${requiredName}.` });
    if (!names.includes(requiredName)) violations.push({ kind: 'interface_payload_required_endpoint_missing', detail: `Endpoint budget definition is missing ${requiredName}.` });
  }
  for (const duplicate of duplicateValues(names)) {
    violations.push({ kind: 'interface_payload_duplicate_endpoint_budget', detail: `Duplicate endpoint budget ${duplicate}.` });
  }
  const gatewayRouteNames = new Set([...(gateway.required_route_classes || []), ...(gateway.route_classes || []).map((route) => route.name)]);
  for (const row of endpoints) {
    if (!gatewayRouteNames.has(row.route_class)) {
      violations.push({ kind: 'interface_payload_unknown_gateway_route_class', detail: `${row.name} references unknown Gateway route_class ${row.route_class}.` });
    }
  }
}

function validateEndpoint(row: EndpointBudget, contract: Contract, forbidden: Set<string>, violations: Violation[]): void {
  const global = contract.global_default_payload_budget;
  if (!global) return;
  if (row.default_payload !== true) {
    violations.push({ kind: 'interface_payload_endpoint_not_default', detail: `${row.name} must declare default_payload=true.` });
  }
  if (!cleanText(row.surface || '', 120)) {
    violations.push({ kind: 'interface_payload_endpoint_surface_missing', detail: `${row.name} must declare a surface.` });
  }
  if (!cleanText(row.payload_kind || '', 160).startsWith('bounded_')) {
    violations.push({ kind: 'interface_payload_kind_not_bounded', detail: `${row.name} payload_kind must start with bounded_.` });
  }
  const patterns = row.endpoint_patterns || [];
  if (!patterns.length || patterns.length > global.max_endpoint_patterns) {
    violations.push({ kind: 'interface_payload_endpoint_pattern_budget_invalid', detail: `${row.name} must declare 1-${global.max_endpoint_patterns} endpoint patterns.` });
  }
  for (const pattern of patterns) {
    if (!pattern.startsWith('/')) {
      violations.push({ kind: 'interface_payload_endpoint_pattern_invalid', detail: `${row.name} pattern must start with /: ${pattern}` });
    }
  }
  const numericChecks: Array<[keyof EndpointBudget, number]> = [
    ['max_response_bytes', global.max_response_bytes],
    ['max_array_items', global.max_array_items],
    ['max_object_depth', global.max_object_depth],
    ['max_string_chars', global.max_string_chars],
    ['max_nested_collection_items', global.max_nested_collection_items],
    ['max_top_level_fields', global.max_top_level_fields],
  ];
  for (const [key, max] of numericChecks) {
    const value = Number(row[key]);
    if (!numberInRange(value, 1, max)) {
      violations.push({ kind: 'interface_payload_endpoint_budget_invalid', detail: `${row.name} ${String(key)} must be between 1 and ${max}.` });
    }
  }
  const fields = row.allowed_top_level_fields || [];
  if (!fields.length || fields.length > row.max_top_level_fields) {
    violations.push({ kind: 'interface_payload_allowed_fields_invalid', detail: `${row.name} must declare 1-${row.max_top_level_fields} allowed top-level fields.` });
  }
  for (const duplicate of duplicateValues(fields)) {
    violations.push({ kind: 'interface_payload_duplicate_allowed_field', detail: `${row.name} repeats allowed field ${duplicate}.` });
  }
  for (const field of fields) {
    if (forbidden.has(field) || RAW_FIELD_PATTERN.test(field)) {
      violations.push({ kind: 'interface_payload_forbidden_default_field', detail: `${row.name} exposes forbidden/default-heavy field ${field}.` });
    }
  }
  if (CURSOR_ROUTE_CLASSES.has(row.route_class) && row.requires_cursor !== true) {
    violations.push({ kind: 'interface_payload_cursor_required', detail: `${row.name} must require cursor/window fields for ${row.route_class}.` });
  }
  if (row.requires_cursor && !(row.cursor_fields || []).length) {
    violations.push({ kind: 'interface_payload_cursor_fields_missing', detail: `${row.name} requires cursor fields but declares none.` });
  }
  if (row.requires_detail_refs && !(row.required_ref_fields || []).length) {
    violations.push({ kind: 'interface_payload_detail_refs_missing', detail: `${row.name} requires detail refs but declares no required ref fields.` });
  }
  if (!ALLOWED_OVERFLOW.has(row.overflow_strategy)) {
    violations.push({ kind: 'interface_payload_overflow_strategy_invalid', detail: `${row.name} must use omit or cursor_or_detail_ref overflow strategy.` });
  }
  for (const key of ['capability_or_lease_required', 'audit_receipt', 'nexus_checkpoint'] as const) {
    if (row[key] !== true) {
      violations.push({ kind: 'interface_payload_guardrail_missing', detail: `${row.name} must set ${key}=true.` });
    }
  }
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Interface Payload Budget Guard');
  lines.push('');
  lines.push(`- Generated at: ${payload.generated_at}`);
  lines.push(`- Revision: ${payload.revision}`);
  lines.push(`- Pass: ${payload.ok}`);
  lines.push(`- Contract: ${payload.contract_path}`);
  lines.push('');
  lines.push('## Summary');
  for (const [key, value] of Object.entries(payload.summary)) lines.push(`- ${key}: ${value}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) lines.push(`- ${violation.kind}: ${violation.path || ''} ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const sourceContract = readJson<Contract>(args.contractPath);
  const contract = args.includeControlledViolation ? applyControlledViolation(sourceContract) : sourceContract;
  const gateway = contract.gateway_contract_path && exists(contract.gateway_contract_path) ? readJson<GatewayContract>(contract.gateway_contract_path) : {};
  const shell = contract.shell_message_contract_path && exists(contract.shell_message_contract_path) ? readJson<ShellMessageContract>(contract.shell_message_contract_path) : {};
  const violations: Violation[] = [];

  validateDocs(contract, args.contractPath, violations);
  validateGlobalBudget(contract, violations);
  validateEndpointList(contract, gateway, violations);
  const forbidden = mergedForbiddenFields(contract, gateway, shell);
  for (const row of contract.default_endpoint_budgets || []) validateEndpoint(row, contract, forbidden, violations);

  const payload = {
    ok: violations.length === 0,
    type: 'interface_payload_budget_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      endpoint_budgets: (contract.default_endpoint_budgets || []).length,
      required_endpoint_budgets: (contract.required_endpoint_budgets || []).length,
      forbidden_default_payload_fields: forbidden.size,
      strict_violations: violations.length,
    },
    violations,
  };

  writeTextArtifact(args.outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: args.outJson });
  if (args.strict && !payload.ok) process.exitCode = 1;
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
