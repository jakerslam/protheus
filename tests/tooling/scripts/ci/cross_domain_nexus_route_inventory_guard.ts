#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'client/runtime/config/cross_domain_nexus_route_inventory.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/cross_domain_nexus_route_inventory_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/CROSS_DOMAIN_NEXUS_ROUTE_INVENTORY_GUARD_CURRENT.md';

type Route = {
  route_id: string;
  route_class: string;
  source_domain: string;
  target_domain: string;
  source_checkpoint: string;
  target_checkpoint: string;
  conduit_path: string;
  conduit_security_posture: string;
  lease_or_capability_required: boolean;
  lifecycle_gate: boolean;
  receipt_required: boolean;
  nexus_checkpoint: boolean;
  owner_of_truth: string;
  default_projection: boolean;
  detail_route: boolean;
  endpoint_budget_refs?: string[];
};

type Contract = {
  inventory_doc_path: string;
  related_policy_paths?: string[];
  policy_doc_required_tokens?: string[];
  gateway_contract_path: string;
  interface_payload_budget_contract_path: string;
  conduit_scrambler_posture_contract_path: string;
  required_domains?: string[];
  required_route_classes?: string[];
  allowed_postures?: string[];
  forbidden_payload_markers?: string[];
  routes?: Route[];
};

type Violation = {
  kind: string;
  route_id?: string;
  path?: string;
  detail: string;
};

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function exists(relPath: string): boolean {
  return !!relPath && fs.existsSync(abs(relPath));
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function readJson<T>(relPath: string): T {
  return JSON.parse(readText(relPath)) as T;
}

function duplicateValues(values: string[]): string[] {
  const counts = new Map<string, number>();
  for (const value of values) counts.set(value, (counts.get(value) || 0) + 1);
  return Array.from(counts.entries()).filter(([, count]) => count > 1).map(([value]) => value);
}

function cloneContract(contract: Contract): Contract {
  return JSON.parse(JSON.stringify(contract)) as Contract;
}

function applyControlledViolation(contract: Contract): Contract {
  const copy = cloneContract(contract);
  copy.required_domains = (copy.required_domains || []).filter((domain) => domain !== 'app');
  const route = (copy.routes || []).find((row) => row.route_id === 'orchestration_contract_to_core_conduit');
  if (route) {
    route.conduit_security_posture = 'standard_conduit';
    route.receipt_required = false;
    route.target_checkpoint = 'missing/nexus/checkpoint.rs';
  }
  return copy;
}

function validateDocs(contract: Contract, violations: Violation[]): void {
  const docs = [contract.inventory_doc_path, ...(contract.related_policy_paths || [])];
  for (const docPath of docs) {
    if (!exists(docPath)) {
      violations.push({ kind: 'route_inventory_doc_missing', path: docPath, detail: 'Required policy or inventory document is missing.' });
    }
  }
  if (exists(contract.inventory_doc_path)) {
    const doc = readText(contract.inventory_doc_path);
    for (const token of contract.policy_doc_required_tokens || []) {
      if (!doc.includes(token)) {
        violations.push({ kind: 'route_inventory_doc_token_missing', path: contract.inventory_doc_path, detail: `Missing token: ${token}` });
      }
    }
  }
}

function validateCoverage(contract: Contract, routes: Route[], violations: Violation[]): void {
  const domains = new Set<string>();
  const classes = new Set<string>();
  for (const route of routes) {
    domains.add(route.source_domain);
    domains.add(route.target_domain);
    classes.add(route.route_class);
  }
  for (const domain of contract.required_domains || []) {
    if (!domains.has(domain)) violations.push({ kind: 'route_inventory_required_domain_missing', detail: `Missing required domain ${domain}.` });
  }
  for (const routeClass of contract.required_route_classes || []) {
    if (!classes.has(routeClass)) violations.push({ kind: 'route_inventory_required_class_missing', detail: `Missing required route class ${routeClass}.` });
  }
}

function sensitiveMinimumPosture(scramblerContract: any, routeClass: string): string {
  return String(scramblerContract?.minimum_posture_by_route_class?.[routeClass] || 'standard_conduit');
}

function validateRoute(route: Route, contract: Contract, scramblerContract: any, budgetNames: Set<string>, violations: Violation[]): void {
  for (const key of ['route_id', 'route_class', 'source_domain', 'target_domain', 'source_checkpoint', 'target_checkpoint', 'conduit_path', 'conduit_security_posture', 'owner_of_truth'] as const) {
    if (!cleanText(String(route[key] || ''), 600)) violations.push({ kind: 'route_inventory_required_field_missing', route_id: route.route_id, detail: `${key} is required.` });
  }
  for (const key of ['source_checkpoint', 'target_checkpoint', 'conduit_path'] as const) {
    if (!exists(route[key])) violations.push({ kind: 'route_inventory_path_missing', route_id: route.route_id, path: route[key], detail: `${key} does not exist.` });
  }
  if (!(contract.allowed_postures || []).includes(route.conduit_security_posture)) {
    violations.push({ kind: 'route_inventory_posture_invalid', route_id: route.route_id, detail: `${route.conduit_security_posture} is not allowed.` });
  }
  const minimum = sensitiveMinimumPosture(scramblerContract, route.route_class);
  if (minimum === 'strong_scrambler' && route.conduit_security_posture !== 'strong_scrambler') {
    violations.push({ kind: 'route_inventory_sensitive_route_posture_too_weak', route_id: route.route_id, detail: `${route.route_class} requires strong_scrambler.` });
  }
  for (const key of ['lease_or_capability_required', 'lifecycle_gate', 'receipt_required', 'nexus_checkpoint'] as const) {
    if (route[key] !== true) violations.push({ kind: 'route_inventory_guardrail_missing', route_id: route.route_id, detail: `${key} must be true.` });
  }
  if (route.owner_of_truth !== 'core_or_orchestration') {
    violations.push({ kind: 'route_inventory_owner_invalid', route_id: route.route_id, detail: 'owner_of_truth must be core_or_orchestration.' });
  }
  for (const ref of route.endpoint_budget_refs || []) {
    if (!budgetNames.has(ref)) violations.push({ kind: 'route_inventory_endpoint_budget_ref_unknown', route_id: route.route_id, detail: `Unknown endpoint budget ref ${ref}.` });
  }
  const fields = JSON.stringify(route).toLowerCase();
  for (const marker of contract.forbidden_payload_markers || []) {
    if (fields.includes(`"${marker.toLowerCase()}":`)) {
      violations.push({ kind: 'route_inventory_forbidden_payload_marker', route_id: route.route_id, detail: `Route metadata contains forbidden payload marker ${marker}.` });
    }
  }
}

function validateScramblerRequiredRoutes(routes: Route[], scramblerContract: any, violations: Violation[]): void {
  const routeIds = new Set(routes.map((route) => route.route_id));
  for (const requiredRoute of scramblerContract?.required_sensitive_routes || []) {
    if (!routeIds.has(requiredRoute)) {
      violations.push({ kind: 'route_inventory_missing_required_sensitive_route', route_id: requiredRoute, detail: 'Required sensitive route from scrambler posture contract is not inventoried.' });
    }
  }
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Cross-Domain Nexus Route Inventory Guard');
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
  for (const violation of payload.violations) lines.push(`- ${violation.kind}: ${violation.route_id || ''} ${violation.path || ''} ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  const contractPath = cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT, 600);
  const outJson = cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600);
  const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
  const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
  const sourceContract = readJson<Contract>(contractPath);
  const contract = includeControlledViolation ? applyControlledViolation(sourceContract) : sourceContract;
  const scramblerContract = readJson<any>(contract.conduit_scrambler_posture_contract_path);
  const payloadBudgetContract = readJson<any>(contract.interface_payload_budget_contract_path);
  const budgetNames = new Set<string>((payloadBudgetContract.default_endpoint_budgets || []).map((row: any) => String(row.name)));
  const routes = contract.routes || [];
  const violations: Violation[] = [];

  validateDocs(contract, violations);
  validateCoverage(contract, routes, violations);
  for (const duplicate of duplicateValues(routes.map((route) => route.route_id))) {
    violations.push({ kind: 'route_inventory_duplicate_route_id', route_id: duplicate, detail: `Duplicate route id ${duplicate}.` });
  }
  for (const route of routes) validateRoute(route, contract, scramblerContract, budgetNames, violations);
  validateScramblerRequiredRoutes(routes, scramblerContract, violations);

  const payload = {
    ok: violations.length === 0,
    type: 'cross_domain_nexus_route_inventory_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: common.strict,
    contract_path: contractPath,
    controlled_violation: includeControlledViolation,
    summary: {
      routes: routes.length,
      required_domains: (contract.required_domains || []).length,
      required_route_classes: (contract.required_route_classes || []).length,
      required_sensitive_routes: (scramblerContract.required_sensitive_routes || []).length,
      strict_violations: violations.length,
    },
    violations,
  };

  writeTextArtifact(outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: outJson });
  if (common.strict && !payload.ok) process.exitCode = 1;
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
