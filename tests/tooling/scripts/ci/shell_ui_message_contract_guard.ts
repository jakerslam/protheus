#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'client/runtime/config/shell_ui_message_detail_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_ui_message_contract_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_UI_MESSAGE_CONTRACT_GUARD_CURRENT.md';

type ProjectionContract = {
  name: string;
  description?: string;
  required_fields?: string[];
  optional_fields?: string[];
  max_preview_chars?: number;
  max_summary_items?: number;
  max_display_actions?: number;
};

type LazyDetailClass = {
  name: string;
  id_field: string;
  covers?: string[];
};

type DetailRoute = {
  name: string;
  route_kind: string;
  id_field: string;
  capability_scope: string;
  bounded_response: boolean;
  audit_receipt: boolean;
  nexus_checkpoint: boolean;
};

type Contract = {
  version?: string;
  policy_doc_path: string;
  source_projection_policy_path: string;
  policy_doc_required_tokens?: string[];
  default_projection_contracts?: ProjectionContract[];
  prohibited_default_fields?: string[];
  lazy_detail_classes?: LazyDetailClass[];
  detail_routes?: DetailRoute[];
  history_windowing?: {
    default_payload_requires_cursor?: boolean;
    default_payload_forbids_complete_message_array?: boolean;
    cursor_fields?: string[];
  };
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

const REQUIRED_MESSAGE_FIELDS = [
  'id',
  'conversation_id',
  'origin_kind',
  'origin_display_name',
  'timestamp',
  'status',
  'content_preview',
  'line_count',
  'detail_ref',
  'allowed_display_actions',
];

const REQUIRED_SESSION_FIELDS = [
  'id',
  'title',
  'active_agent_id',
  'active_agent_name',
  'status',
  'last_message_preview',
  'last_message_at',
  'message_count',
  'detail_ref',
];

const REQUIRED_DETAIL_ROUTES = [
  'message_detail',
  'tool_result_detail',
  'artifact_detail',
  'trace_detail',
  'workflow_detail',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readJson<T>(relPath: string): T {
  return JSON.parse(fs.readFileSync(abs(relPath), 'utf8')) as T;
}

function exists(relPath: string): boolean {
  return fs.existsSync(abs(relPath));
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
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

function unique(rows: string[]): string[] {
  return Array.from(new Set(rows));
}

function duplicateValues(rows: string[]): string[] {
  const counts = new Map<string, number>();
  for (const row of rows) counts.set(row, (counts.get(row) || 0) + 1);
  return Array.from(counts.entries()).filter(([, count]) => count > 1).map(([row]) => row);
}

function includesAll(haystack: string[], needles: string[]): string[] {
  const set = new Set(haystack);
  return needles.filter((needle) => !set.has(needle));
}

function normalizedFieldSet(contract: ProjectionContract): string[] {
  return unique([...(contract.required_fields || []), ...(contract.optional_fields || [])]);
}

function isRawShapedField(field: string): boolean {
  return /(^|_)(raw|root|trace|graph|payload|observation|quality|authorization|policy_decision)(_|$)/.test(field);
}

function cloneContract(contract: Contract): Contract {
  return JSON.parse(JSON.stringify(contract)) as Contract;
}

function applyControlledViolation(contract: Contract): Contract {
  const copy = cloneContract(contract);
  const message = (copy.default_projection_contracts || []).find((row) => row.name === 'shell_chat_message_projection');
  if (message) {
    message.optional_fields = [...(message.optional_fields || []), 'tool_result', 'decision_trace'];
    message.max_preview_chars = 250000;
  }
  copy.detail_routes = (copy.detail_routes || []).filter((route) => route.name !== 'tool_result_detail');
  copy.history_windowing = {
    ...(copy.history_windowing || {}),
    default_payload_forbids_complete_message_array: false,
  };
  return copy;
}

function validateDoc(contract: Contract, contractPath: string, violations: Violation[]): void {
  const docs = [contract.policy_doc_path, contract.source_projection_policy_path];
  for (const docPath of docs) {
    if (!docPath || !exists(docPath)) {
      violations.push({ kind: 'missing_shell_ui_message_contract_doc', path: docPath || contractPath, detail: 'Required policy document is missing.' });
    }
  }
  if (contract.policy_doc_path && exists(contract.policy_doc_path)) {
    const doc = readText(contract.policy_doc_path);
    for (const token of contract.policy_doc_required_tokens || []) {
      if (!doc.includes(token)) {
        violations.push({
          kind: 'shell_ui_message_contract_doc_missing_token',
          path: contract.policy_doc_path,
          detail: `Missing required contract token: ${token}`,
        });
      }
    }
  }
}

function validateProjectionContracts(contract: Contract, violations: Violation[]): void {
  const projections = contract.default_projection_contracts || [];
  const projectionNames = projections.map((row) => row.name);
  for (const duplicate of duplicateValues(projectionNames)) {
    violations.push({ kind: 'duplicate_projection_contract_name', detail: `Duplicate default projection contract: ${duplicate}` });
  }

  const message = projections.find((row) => row.name === 'shell_chat_message_projection');
  const session = projections.find((row) => row.name === 'shell_session_projection');
  if (!message) violations.push({ kind: 'missing_message_projection_contract', detail: 'Missing shell_chat_message_projection contract.' });
  if (!session) violations.push({ kind: 'missing_session_projection_contract', detail: 'Missing shell_session_projection contract.' });

  for (const row of projections) {
    const fields = normalizedFieldSet(row);
    const required = row.name === 'shell_chat_message_projection'
      ? REQUIRED_MESSAGE_FIELDS
      : row.name === 'shell_session_projection'
        ? REQUIRED_SESSION_FIELDS
        : [];
    for (const missing of includesAll(row.required_fields || [], required)) {
      violations.push({ kind: 'projection_required_field_missing', detail: `${row.name} is missing required field ${missing}.` });
    }
    for (const duplicate of duplicateValues([...(row.required_fields || []), ...(row.optional_fields || [])])) {
      violations.push({ kind: 'projection_duplicate_field', detail: `${row.name} repeats field ${duplicate}.` });
    }
    for (const forbidden of contract.prohibited_default_fields || []) {
      if (fields.includes(forbidden)) {
        violations.push({ kind: 'projection_forbidden_field', detail: `${row.name} includes prohibited default field ${forbidden}.` });
      }
    }
    for (const field of fields.filter(isRawShapedField)) {
      violations.push({ kind: 'projection_raw_shaped_field', detail: `${row.name} includes raw-shaped default field ${field}.` });
    }
    if (Number(row.max_preview_chars || 0) <= 0 || Number(row.max_preview_chars || 0) > 12000) {
      violations.push({ kind: 'projection_preview_budget_invalid', detail: `${row.name} preview budget must be > 0 and <= 12000 chars.` });
    }
    if (Number(row.max_summary_items || 0) > 20) {
      violations.push({ kind: 'projection_summary_budget_invalid', detail: `${row.name} summary budget must be <= 20.` });
    }
    if (Number(row.max_display_actions || 0) > 12) {
      violations.push({ kind: 'projection_action_budget_invalid', detail: `${row.name} display action budget must be <= 12.` });
    }
  }
}

function validateLazyRoutes(contract: Contract, violations: Violation[]): void {
  const routes = contract.detail_routes || [];
  const routeNames = routes.map((row) => row.name);
  for (const duplicate of duplicateValues(routeNames)) {
    violations.push({ kind: 'duplicate_detail_route_name', detail: `Duplicate detail route: ${duplicate}` });
  }
  for (const required of REQUIRED_DETAIL_ROUTES) {
    if (!routeNames.includes(required)) {
      violations.push({ kind: 'missing_required_detail_route', detail: `Missing required detail route ${required}.` });
    }
  }

  const routeByName = new Map(routes.map((route) => [route.name, route]));
  for (const route of routes) {
    if (route.route_kind !== 'detail_fetch') {
      violations.push({ kind: 'detail_route_kind_invalid', detail: `${route.name} must use route_kind=detail_fetch.` });
    }
    if (!route.id_field || !route.id_field.endsWith('_id')) {
      violations.push({ kind: 'detail_route_id_field_invalid', detail: `${route.name} must declare an *_id id_field.` });
    }
    if (!route.capability_scope || !route.capability_scope.startsWith('shell.')) {
      violations.push({ kind: 'detail_route_capability_scope_invalid', detail: `${route.name} must declare a shell.* capability scope.` });
    }
    for (const key of ['bounded_response', 'audit_receipt', 'nexus_checkpoint'] as const) {
      if (route[key] !== true) {
        violations.push({ kind: 'detail_route_guardrail_missing', detail: `${route.name} must set ${key}=true.` });
      }
    }
  }

  for (const detailClass of contract.lazy_detail_classes || []) {
    const route = routeByName.get(detailClass.name);
    if (!route) {
      violations.push({ kind: 'lazy_class_missing_detail_route', detail: `${detailClass.name} has no matching detail route.` });
      continue;
    }
    if (route.id_field !== detailClass.id_field) {
      violations.push({
        kind: 'lazy_class_route_id_mismatch',
        detail: `${detailClass.name} declares ${detailClass.id_field} but route uses ${route.id_field}.`,
      });
    }
    if (!(detailClass.covers || []).length) {
      violations.push({ kind: 'lazy_class_empty_coverage', detail: `${detailClass.name} must cover at least one heavy detail class.` });
    }
  }
}

function validateWindowing(contract: Contract, violations: Violation[]): void {
  const windowing = contract.history_windowing || {};
  if (windowing.default_payload_requires_cursor !== true) {
    violations.push({ kind: 'history_windowing_cursor_required', detail: 'Default history payloads must require cursor/window fields.' });
  }
  if (windowing.default_payload_forbids_complete_message_array !== true) {
    violations.push({ kind: 'history_windowing_full_array_forbidden', detail: 'Default history payloads must forbid complete message arrays.' });
  }
  if (!(windowing.cursor_fields || []).length) {
    violations.push({ kind: 'history_windowing_cursor_fields_missing', detail: 'At least one cursor/window field must be declared.' });
  }
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell UI Message Contract Guard');
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

  validateDoc(contract, args.contractPath, violations);
  validateProjectionContracts(contract, violations);
  validateLazyRoutes(contract, violations);
  validateWindowing(contract, violations);

  const payload = {
    ok: violations.length === 0,
    type: 'shell_ui_message_contract_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      projection_contracts: (contract.default_projection_contracts || []).length,
      prohibited_default_fields: (contract.prohibited_default_fields || []).length,
      lazy_detail_classes: (contract.lazy_detail_classes || []).length,
      detail_routes: (contract.detail_routes || []).length,
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
