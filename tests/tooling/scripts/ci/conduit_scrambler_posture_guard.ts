#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'client/runtime/config/conduit_scrambler_posture_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/conduit_scrambler_posture_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/CONDUIT_SCRAMBLER_POSTURE_GUARD_CURRENT.md';

type QuantumPosture = {
  name: string;
  implemented: boolean;
  reserved_marker: string;
};

type RouteDeclaration = {
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
  downgrade_allowed: boolean;
  downgrade_owner?: string;
  downgrade_expiry?: string;
  downgrade_replacement_plan?: string;
  quantum_resistant_reserved?: boolean;
};

type Contract = {
  version?: string;
  policy_doc_path: string;
  gateway_contract_path: string;
  related_policy_paths?: string[];
  policy_doc_required_tokens?: string[];
  allowed_postures?: string[];
  forbidden_postures?: string[];
  quantum_resistant_posture?: QuantumPosture;
  minimum_posture_by_route_class?: Record<string, string>;
  required_sensitive_routes?: string[];
  route_declarations?: RouteDeclaration[];
};

type GatewayContract = {
  required_route_classes?: string[];
  route_classes?: Array<{ name: string }>;
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

const POSTURE_RANK: Record<string, number> = {
  standard_conduit: 1,
  strong_scrambler: 2,
  quantum_resistant_scrambler: 3,
};

const REQUIRED_SENSITIVE_CLASSES = new Set([
  'control_plane_coordination',
  'kernel_authority',
  'detail_fetch_sensitive',
  'external_agent_or_plugin_ingress',
  'emergency_recovery',
  'reserved_quantum_security',
]);

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

function applyControlledViolation(contract: Contract): Contract {
  const copy = cloneContract(contract);
  const firstSensitive = (copy.route_declarations || []).find((row) => REQUIRED_SENSITIVE_CLASSES.has(row.route_class));
  if (firstSensitive) {
    firstSensitive.conduit_security_posture = 'standard_conduit';
    firstSensitive.downgrade_allowed = true;
    firstSensitive.downgrade_owner = '';
    firstSensitive.downgrade_expiry = '';
    firstSensitive.downgrade_replacement_plan = '';
    firstSensitive.receipt_required = false;
  }
  const second = (copy.route_declarations || []).find((row) => row.route_id !== firstSensitive?.route_id);
  if (second) {
    second.conduit_security_posture = 'quantum_resistant_scrambler';
    second.quantum_resistant_reserved = false;
  }
  return copy;
}

function validateDocs(contract: Contract, contractPath: string, violations: Violation[]): void {
  const docs = [contract.policy_doc_path, contract.gateway_contract_path, ...(contract.related_policy_paths || [])];
  for (const docPath of docs) {
    if (!docPath || !exists(docPath)) {
      violations.push({ kind: 'scrambler_posture_reference_missing', path: docPath || contractPath, detail: 'Required policy or contract reference is missing.' });
    }
  }
  if (contract.policy_doc_path && exists(contract.policy_doc_path)) {
    const doc = readText(contract.policy_doc_path);
    for (const token of contract.policy_doc_required_tokens || []) {
      if (!doc.includes(token)) {
        violations.push({ kind: 'scrambler_posture_policy_missing_token', path: contract.policy_doc_path, detail: `Missing required policy token: ${token}` });
      }
    }
  }
}

function validatePostureVocabulary(contract: Contract, violations: Violation[]): void {
  const allowed = new Set(contract.allowed_postures || []);
  for (const required of ['standard_conduit', 'strong_scrambler']) {
    if (!allowed.has(required)) {
      violations.push({ kind: 'scrambler_posture_allowed_value_missing', detail: `Allowed postures must include ${required}.` });
    }
  }
  for (const forbidden of ['plain_transport', 'raw_transport', 'none']) {
    if (!(contract.forbidden_postures || []).includes(forbidden)) {
      violations.push({ kind: 'scrambler_posture_forbidden_value_missing', detail: `Forbidden postures must include ${forbidden}.` });
    }
  }
  if (!contract.quantum_resistant_posture || contract.quantum_resistant_posture.implemented !== false) {
    violations.push({ kind: 'scrambler_quantum_posture_not_reserved', detail: 'Quantum-resistant Scrambler must remain reserved until a milestone implements it.' });
  }
}

function validateGatewayCompatibility(contract: Contract, gateway: GatewayContract, violations: Violation[]): void {
  const routeNames = new Set([...(gateway.required_route_classes || []), ...(gateway.route_classes || []).map((row) => row.name)]);
  for (const required of ['request_ingress', 'event_output_egress', 'health_status', 'detail_fetch', 'bounded_search_query']) {
    if (!routeNames.has(required)) {
      violations.push({ kind: 'scrambler_gateway_route_class_missing', detail: `Gateway contract is missing route class ${required}.` });
    }
  }
}

function minimumPosture(contract: Contract, routeClass: string): string {
  return cleanText(contract.minimum_posture_by_route_class?.[routeClass] || '', 120);
}

function validateRouteDeclaration(route: RouteDeclaration, contract: Contract, violations: Violation[]): void {
  const pathLabel = `route:${route.route_id || '<missing>'}`;
  for (const key of ['route_id', 'route_class', 'source_domain', 'target_domain', 'source_checkpoint', 'target_checkpoint', 'conduit_path', 'conduit_security_posture'] as const) {
    if (!cleanText(route[key], 600)) {
      violations.push({ kind: 'scrambler_route_required_field_missing', path: pathLabel, detail: `Route declaration is missing ${key}.` });
    }
  }
  for (const key of ['source_checkpoint', 'target_checkpoint', 'conduit_path'] as const) {
    const relPath = cleanText(route[key], 600);
    if (relPath && !exists(relPath)) {
      violations.push({ kind: 'scrambler_route_checkpoint_missing', path: relPath, detail: `${route.route_id} references a missing ${key}.` });
    }
  }
  const posture = cleanText(route.conduit_security_posture, 120);
  if ((contract.forbidden_postures || []).includes(posture)) {
    violations.push({ kind: 'scrambler_route_forbidden_posture', path: pathLabel, detail: `${route.route_id} uses forbidden posture ${posture}.` });
  }
  const allowed = new Set([...(contract.allowed_postures || []), contract.quantum_resistant_posture?.name || '']);
  if (!allowed.has(posture)) {
    violations.push({ kind: 'scrambler_route_unknown_posture', path: pathLabel, detail: `${route.route_id} uses unknown posture ${posture}.` });
  }
  const minimum = minimumPosture(contract, route.route_class);
  if (!minimum) {
    violations.push({ kind: 'scrambler_route_class_minimum_missing', path: pathLabel, detail: `${route.route_class} has no minimum posture declaration.` });
  } else if ((POSTURE_RANK[posture] || 0) < (POSTURE_RANK[minimum] || 0)) {
    violations.push({ kind: 'scrambler_route_posture_downgrade', path: pathLabel, detail: `${route.route_id} declares ${posture}; ${route.route_class} requires at least ${minimum}.` });
  }
  if (REQUIRED_SENSITIVE_CLASSES.has(route.route_class) && posture !== 'strong_scrambler') {
    violations.push({ kind: 'scrambler_sensitive_route_not_strong', path: pathLabel, detail: `${route.route_id} is sensitive and must declare strong_scrambler.` });
  }
  for (const key of ['lease_or_capability_required', 'lifecycle_gate', 'receipt_required'] as const) {
    if (route[key] !== true) {
      violations.push({ kind: 'scrambler_route_guardrail_missing', path: pathLabel, detail: `${route.route_id} must set ${key}=true.` });
    }
  }
  if (route.downgrade_allowed === true) {
    for (const key of ['downgrade_owner', 'downgrade_expiry', 'downgrade_replacement_plan'] as const) {
      if (!cleanText(route[key], 500)) {
        violations.push({ kind: 'scrambler_downgrade_metadata_missing', path: pathLabel, detail: `${route.route_id} downgrade is missing ${key}.` });
      }
    }
  }
  if (posture === contract.quantum_resistant_posture?.name && contract.quantum_resistant_posture.implemented !== true) {
    violations.push({ kind: 'scrambler_quantum_posture_false_claim', path: pathLabel, detail: `${route.route_id} claims quantum-resistant posture before the milestone exists.` });
  }
  if (route.route_class === 'reserved_quantum_security' && route.quantum_resistant_reserved !== true) {
    violations.push({ kind: 'scrambler_quantum_reserved_marker_missing', path: pathLabel, detail: `${route.route_id} reserved quantum route must set quantum_resistant_reserved=true.` });
  }
}

function validateRoutes(contract: Contract, violations: Violation[]): void {
  const routes = contract.route_declarations || [];
  const routeIds = routes.map((row) => row.route_id);
  for (const duplicate of duplicateValues(routeIds)) {
    violations.push({ kind: 'scrambler_duplicate_route_id', detail: `Duplicate route declaration ${duplicate}.` });
  }
  for (const required of contract.required_sensitive_routes || []) {
    if (!routeIds.includes(required)) {
      violations.push({ kind: 'scrambler_required_sensitive_route_missing', detail: `Missing sensitive route declaration ${required}.` });
    }
  }
  for (const route of routes) validateRouteDeclaration(route, contract, violations);
}

function markdown(payload: any): string {
  const lines = [
    '# Conduit/Scrambler Posture Guard',
    '',
    `- Generated at: ${payload.generated_at}`,
    `- Revision: ${payload.revision}`,
    `- Pass: ${payload.ok}`,
    `- Contract: ${payload.contract_path}`,
    '',
    '## Summary',
  ];
  for (const [key, value] of Object.entries(payload.summary)) lines.push(`- ${key}: ${value}`);
  lines.push('', '## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) lines.push(`- ${violation.kind}: ${violation.path || ''} ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const sourceContract = readJson<Contract>(args.contractPath);
  const contract = args.includeControlledViolation ? applyControlledViolation(sourceContract) : sourceContract;
  const gateway = exists(contract.gateway_contract_path) ? readJson<GatewayContract>(contract.gateway_contract_path) : {};
  const violations: Violation[] = [];

  validateDocs(contract, args.contractPath, violations);
  validatePostureVocabulary(contract, violations);
  validateGatewayCompatibility(contract, gateway, violations);
  validateRoutes(contract, violations);

  const payload = {
    ok: violations.length === 0,
    type: 'conduit_scrambler_posture_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      route_declarations: (contract.route_declarations || []).length,
      required_sensitive_routes: (contract.required_sensitive_routes || []).length,
      allowed_postures: (contract.allowed_postures || []).length,
      forbidden_postures: (contract.forbidden_postures || []).length,
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
