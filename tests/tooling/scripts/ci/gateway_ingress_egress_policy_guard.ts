#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'client/runtime/config/gateway_ingress_egress_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/gateway_ingress_egress_policy_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/GATEWAY_INGRESS_EGRESS_POLICY_GUARD_CURRENT.md';

type RouteClass = {
  name: string;
  direction: string;
  source_domain: string;
  target_domain: string;
  owner_of_truth: string;
  payload_class: string;
  allowed_payload_fields?: string[];
  returns?: string[];
  bounded_response: boolean;
  capability_or_lease_required: boolean;
  audit_receipt: boolean;
  nexus_checkpoint: boolean;
};

type Contract = {
  version?: string;
  policy_doc_path: string;
  related_policy_paths?: string[];
  policy_doc_required_tokens?: string[];
  required_route_classes?: string[];
  route_classes?: RouteClass[];
  forbidden_route_shapes?: string[];
  forbidden_default_payload_fields?: string[];
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

const REQUIRED_ROUTE_CLASSES = [
  'request_ingress',
  'event_output_egress',
  'health_status',
  'detail_fetch',
  'bounded_search_query',
];

const ALLOWED_DIRECTIONS = new Set([
  'consumer_to_authority',
  'authority_to_consumer',
  'consumer_to_authority_to_consumer',
]);

const RAW_FIELD_PATTERN = /(^|_)(raw|root|full|all|trace_body|plan_graph|workflow_graph|observation|authorization_state|policy_decision)(_|$)/;

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

function duplicateValues(rows: string[]): string[] {
  const counts = new Map<string, number>();
  for (const row of rows) counts.set(row, (counts.get(row) || 0) + 1);
  return Array.from(counts.entries()).filter(([, count]) => count > 1).map(([row]) => row);
}

function cloneContract(contract: Contract): Contract {
  return JSON.parse(JSON.stringify(contract)) as Contract;
}

function applyControlledViolation(contract: Contract): Contract {
  const copy = cloneContract(contract);
  copy.required_route_classes = (copy.required_route_classes || []).filter((name) => name !== 'health_status');
  const ingress = (copy.route_classes || []).find((row) => row.name === 'request_ingress');
  if (ingress) {
    ingress.payload_class = 'full_state';
    ingress.allowed_payload_fields = [...(ingress.allowed_payload_fields || []), 'raw_tool_result', 'conversation_tree'];
    ingress.returns = [...(ingress.returns || []), 'all_messages'];
  }
  const detail = (copy.route_classes || []).find((row) => row.name === 'detail_fetch');
  if (detail) {
    detail.bounded_response = false;
    detail.audit_receipt = false;
    detail.nexus_checkpoint = false;
  }
  return copy;
}

function validateDocs(contract: Contract, contractPath: string, violations: Violation[]): void {
  const docs = [contract.policy_doc_path, ...(contract.related_policy_paths || [])];
  for (const docPath of docs) {
    if (!docPath || !exists(docPath)) {
      violations.push({ kind: 'missing_gateway_interface_policy_doc', path: docPath || contractPath, detail: 'Required policy document is missing.' });
    }
  }
  if (contract.policy_doc_path && exists(contract.policy_doc_path)) {
    const doc = readText(contract.policy_doc_path);
    for (const token of contract.policy_doc_required_tokens || []) {
      if (!doc.includes(token)) {
        violations.push({
          kind: 'gateway_interface_policy_doc_missing_token',
          path: contract.policy_doc_path,
          detail: `Missing required policy token: ${token}`,
        });
      }
    }
  }
}

function validateRouteClassList(contract: Contract, violations: Violation[]): void {
  const required = contract.required_route_classes || [];
  const routeNames = (contract.route_classes || []).map((row) => row.name);
  for (const name of REQUIRED_ROUTE_CLASSES) {
    if (!required.includes(name)) violations.push({ kind: 'gateway_required_route_class_not_declared', detail: `Required route class list is missing ${name}.` });
    if (!routeNames.includes(name)) violations.push({ kind: 'gateway_required_route_class_missing', detail: `Route class definition is missing ${name}.` });
  }
  for (const duplicate of duplicateValues(routeNames)) {
    violations.push({ kind: 'gateway_duplicate_route_class', detail: `Duplicate route class ${duplicate}.` });
  }
}

function validateRoute(row: RouteClass, contract: Contract, violations: Violation[]): void {
  if (!ALLOWED_DIRECTIONS.has(row.direction)) {
    violations.push({ kind: 'gateway_route_direction_invalid', detail: `${row.name} has invalid direction ${row.direction}.` });
  }
  for (const key of ['source_domain', 'target_domain', 'owner_of_truth', 'payload_class'] as const) {
    if (!cleanText(String(row[key] || ''), 300)) {
      violations.push({ kind: 'gateway_route_required_field_missing', detail: `${row.name} is missing ${key}.` });
    }
  }
  if (row.owner_of_truth !== 'core_or_orchestration') {
    violations.push({ kind: 'gateway_route_owner_invalid', detail: `${row.name} owner_of_truth must be core_or_orchestration.` });
  }
  if ((contract.forbidden_route_shapes || []).includes(row.payload_class)) {
    violations.push({ kind: 'gateway_forbidden_route_shape', detail: `${row.name} uses forbidden payload_class ${row.payload_class}.` });
  }
  const fields = [...(row.allowed_payload_fields || []), ...(row.returns || [])];
  for (const field of fields) {
    if ((contract.forbidden_default_payload_fields || []).includes(field) || RAW_FIELD_PATTERN.test(field)) {
      violations.push({ kind: 'gateway_forbidden_payload_field', detail: `${row.name} exposes forbidden/default-heavy field ${field}.` });
    }
  }
  for (const key of ['bounded_response', 'capability_or_lease_required', 'audit_receipt', 'nexus_checkpoint'] as const) {
    if (row[key] !== true) {
      violations.push({ kind: 'gateway_route_guardrail_missing', detail: `${row.name} must set ${key}=true.` });
    }
  }
  if (!(row.allowed_payload_fields || []).length) {
    violations.push({ kind: 'gateway_route_allowed_fields_missing', detail: `${row.name} must declare allowed payload fields.` });
  }
  if (!(row.returns || []).length) {
    violations.push({ kind: 'gateway_route_returns_missing', detail: `${row.name} must declare return projection fields.` });
  }
}

function validateRoutes(contract: Contract, violations: Violation[]): void {
  for (const row of contract.route_classes || []) validateRoute(row, contract, violations);
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Gateway Ingress/Egress Policy Guard');
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
  const violations: Violation[] = [];

  validateDocs(contract, args.contractPath, violations);
  validateRouteClassList(contract, violations);
  validateRoutes(contract, violations);

  const payload = {
    ok: violations.length === 0,
    type: 'gateway_ingress_egress_policy_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      required_route_classes: (contract.required_route_classes || []).length,
      route_classes: (contract.route_classes || []).length,
      forbidden_route_shapes: (contract.forbidden_route_shapes || []).length,
      forbidden_default_payload_fields: (contract.forbidden_default_payload_fields || []).length,
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
