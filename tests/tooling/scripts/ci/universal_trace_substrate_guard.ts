#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'observability/traces/universal_trace_substrate_contract.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/universal_trace_substrate_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/UNIVERSAL_TRACE_SUBSTRATE_GUARD_CURRENT.md';

type Contract = {
  policy_path: string;
  readme_path: string;
  root_envelope_schema_path: string;
  extension_registry_path: string;
  required_policy_tokens?: string[];
  required_readme_tokens?: string[];
  required_schema_fields?: string[];
  required_source_domains?: string[];
  required_event_kinds?: string[];
  required_authority_classes?: string[];
  required_extension_ids?: string[];
  allowed_root_trace_schema_paths?: string[];
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
  path: string;
  detail: string;
};

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function normalizePath(value: string): string {
  return value.replace(/\\/g, '/').replace(/^\.\//, '');
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
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 600),
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
  };
}

function trackedFiles(): string[] {
  return execFileSync('git', ['ls-files'], { cwd: ROOT, encoding: 'utf8' })
    .split(/\r?\n/)
    .map(normalizePath)
    .filter(Boolean);
}

function pushMissingToken(violations: Violation[], pathName: string, text: string, tokens: string[] = []): void {
  for (const token of tokens) {
    if (!text.includes(token)) {
      violations.push({
        kind: 'trace_contract_missing_doc_token',
        path: pathName,
        detail: `Missing required token: ${token}`,
      });
    }
  }
}

function requireValues(
  violations: Violation[],
  pathName: string,
  kind: string,
  actual: string[],
  expected: string[] = [],
): void {
  for (const value of expected) {
    if (!actual.includes(value)) {
      violations.push({ kind, path: pathName, detail: `Missing required value: ${value}` });
    }
  }
}

function validateSchema(contract: Contract, schema: any, violations: Violation[]): void {
  const schemaPath = contract.root_envelope_schema_path;
  const required = Array.isArray(schema.required) ? schema.required.map(String) : [];
  const properties = schema.properties || {};
  requireValues(violations, schemaPath, 'trace_schema_required_field_missing', required, [
    'schema_version',
    'trace_id',
    'span_id',
    'timestamp',
    'source_domain',
    'producer',
    'authority_class',
    'event_kind',
    'subject',
    'correlation',
  ]);
  for (const field of contract.required_schema_fields || []) {
    if (!properties[field]) {
      violations.push({
        kind: 'trace_schema_property_missing',
        path: schemaPath,
        detail: `Missing property: ${field}`,
      });
    }
  }
  requireValues(violations, schemaPath, 'trace_source_domain_missing', properties.source_domain?.enum || [], contract.required_source_domains);
  requireValues(violations, schemaPath, 'trace_event_kind_missing', properties.event_kind?.enum || [], contract.required_event_kinds);
  requireValues(violations, schemaPath, 'trace_authority_class_missing', properties.authority_class?.enum || [], contract.required_authority_classes);
  const traceDescription = String(properties.trace_id?.description || '');
  for (const token of ['minted once', 'unchanged', 'No component may remint']) {
    if (!traceDescription.includes(token)) {
      violations.push({
        kind: 'trace_identity_description_weak',
        path: schemaPath,
        detail: `trace_id description must include: ${token}`,
      });
    }
  }
}

function validateRegistry(contract: Contract, registry: any, violations: Violation[]): void {
  const registryPath = contract.extension_registry_path;
  const ids = new Set((registry.extensions || []).map((row: any) => String(row.id || '')));
  for (const id of contract.required_extension_ids || []) {
    if (!ids.has(id)) {
      violations.push({ kind: 'trace_extension_missing', path: registryPath, detail: `Missing extension: ${id}` });
    }
  }
  const identityRule = String(registry.trace_identity_rule || '');
  for (const token of ['One trace_id', 'unchanged', 'No component may remint', 'drop', 'fork']) {
    if (!identityRule.includes(token)) {
      violations.push({
        kind: 'trace_registry_identity_rule_weak',
        path: registryPath,
        detail: `trace_identity_rule must include: ${token}`,
      });
    }
  }
  for (const row of registry.extensions || []) {
    if (!row.id || !row.source_domain || !Array.isArray(row.event_kinds) || !row.required_projection) {
      violations.push({
        kind: 'trace_extension_shape_invalid',
        path: registryPath,
        detail: `Extension ${row.id || '<missing>'} must declare id, source_domain, event_kinds, and required_projection.`,
      });
    }
  }
}

function validateAntiFragmentation(contract: Contract, violations: Violation[]): void {
  const allowed = new Set((contract.allowed_root_trace_schema_paths || []).map(normalizePath));
  const rootTracePattern = /(^|\/)(trace|decision_trace|workflow_trace|sentinel_trace|trace_envelope|trace_contract).*(schema|contract|envelope)\.(json|ya?ml)$/i;
  for (const file of trackedFiles()) {
    if (!rootTracePattern.test(file)) continue;
    if (allowed.has(file)) continue;
    if (file.startsWith('observability/traces/') && file !== 'observability/traces/trace_envelope.schema.json') continue;
    if (file.startsWith('tests/fixtures/')) continue;
    violations.push({
      kind: 'fragmented_observability_root_trace_schema',
      path: file,
      detail: 'Root trace schemas/contracts must live in observability/traces/** and project into the universal trace envelope.',
    });
  }
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Universal Trace Substrate Guard');
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
  for (const violation of payload.violations) lines.push(`- ${violation.kind}: ${violation.path} ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const contract = readJson<Contract>(args.contractPath);
  const policyText = readText(contract.policy_path);
  const readmeText = readText(contract.readme_path);
  const schema = readJson<any>(contract.root_envelope_schema_path);
  const registry = readJson<any>(contract.extension_registry_path);
  const violations: Violation[] = [];

  pushMissingToken(violations, contract.policy_path, policyText, contract.required_policy_tokens);
  pushMissingToken(violations, contract.readme_path, readmeText, contract.required_readme_tokens);
  validateSchema(contract, schema, violations);
  validateRegistry(contract, registry, violations);
  validateAntiFragmentation(contract, violations);
  if (args.includeControlledViolation) {
    violations.push({
      kind: 'controlled_fragmented_observability_violation',
      path: args.contractPath,
      detail: 'Controlled failure proves strict mode rejects trace-fragmentation violations.',
    });
  }

  const payload = {
    ok: violations.length === 0,
    type: 'universal_trace_substrate_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      required_schema_fields: (contract.required_schema_fields || []).length,
      required_source_domains: (contract.required_source_domains || []).length,
      required_event_kinds: (contract.required_event_kinds || []).length,
      required_extensions: (contract.required_extension_ids || []).length,
      violations: violations.length,
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
