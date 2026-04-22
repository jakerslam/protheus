#!/usr/bin/env node
/* eslint-disable no-console */
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

type MatrixCapability = {
  enumName: string;
  key: string;
};

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

const DEFAULT_OUT_JSON = 'core/local/artifacts/typed_probe_contract_matrix_guard_current.json';
const DEFAULT_OUT_MARKDOWN =
  'local/workspace/reports/TYPED_PROBE_CONTRACT_MATRIX_GUARD_CURRENT.md';

const MATRIX_CAPABILITIES: MatrixCapability[] = [
  { enumName: 'WorkspaceRead', key: 'workspace_read' },
  { enumName: 'WorkspaceSearch', key: 'workspace_search' },
  { enumName: 'WebSearch', key: 'web_search' },
  { enumName: 'WebFetch', key: 'web_fetch' },
  { enumName: 'ToolRoute', key: 'tool_route' },
];

function parseArgs(argv: string[]): Args {
  const map = new Map<string, string>();
  for (let i = 2; i < argv.length; i += 1) {
    const token = argv[i] || '';
    if (!token.startsWith('--')) continue;
    const [name, raw] = token.split('=', 2);
    if (raw !== undefined) {
      map.set(name.slice(2), raw);
      continue;
    }
    const next = argv[i + 1] || '';
    if (next.length > 0 && !next.startsWith('--')) {
      map.set(name.slice(2), next);
      i += 1;
    } else {
      map.set(name.slice(2), '1');
    }
  }
  const strictRaw = (map.get('strict') || '').toLowerCase();
  const strict = strictRaw === '1' || strictRaw === 'true' || strictRaw === 'yes';
  return {
    strict,
    outJson: (map.get('out-json') || DEFAULT_OUT_JSON).trim(),
    outMarkdown: (map.get('out-markdown') || DEFAULT_OUT_MARKDOWN).trim(),
  };
}

function ensureParent(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function reEscape(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function run(): number {
  const args = parseArgs(process.argv);
  const classifierPath = resolve('surface/orchestration/src/ingress/classifier.rs');
  const preconditionsPath = resolve('surface/orchestration/src/planner/preconditions.rs');
  const contractsPath = resolve('surface/orchestration/src/contracts.rs');
  const ingressPath = resolve('surface/orchestration/src/ingress.rs');

  const classifierSource = readFileSync(classifierPath, 'utf8');
  const preconditionsSource = readFileSync(preconditionsPath, 'utf8');
  const contractsSource = readFileSync(contractsPath, 'utf8');
  const ingressSource = readFileSync(ingressPath, 'utf8');

  const checks: Check[] = [];

  for (const row of MATRIX_CAPABILITIES) {
    const contractRegex = new RegExp(
      `Capability::${reEscape(
        row.enumName,
      )}\\s*=>\\s*Some\\(\\(\\s*"${reEscape(row.key)}"\\s*,\\s*&\\[\\s*"tool_available"\\s*,\\s*"transport_available"\\s*\\]\\s*\\)\\)`,
      'm',
    );
    checks.push({
      id: `typed_probe_contract_matrix_required_key_${row.key}`,
      ok: contractRegex.test(classifierSource),
      detail: `required probe contract maps ${row.enumName} to ${row.key} with tool+transport fields`,
    });

    const probeKeyRegex = new RegExp(
      `Capability::${reEscape(row.enumName)}\\s*=>\\s*&\\[\\s*"${reEscape(row.key)}"\\s*\\]`,
      'm',
    );
    checks.push({
      id: `typed_probe_contract_matrix_probe_keys_${row.key}`,
      ok: probeKeyRegex.test(contractsSource),
      detail: `capability probe key list uses distinct key ${row.key} without execute_tool fallback`,
    });
  }

  checks.push({
    id: 'typed_probe_contract_matrix_no_execute_tool_collapse_in_required_probe_key',
    ok: !/fn\s+required_probe_key[\s\S]*"execute_tool"/m.test(preconditionsSource),
    detail: 'required probe key function must not collapse tool-family authority onto execute_tool',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_reason_template_capability_specific',
    ok: classifierSource.includes('typed_probe_contract_missing:capability.{capability_key}'),
    detail: 'classifier emits capability-specific missing probe diagnostics',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_reason_template_field_specific',
    ok: classifierSource.includes('typed_probe_contract_missing:field.{capability_key}.{field}'),
    detail: 'classifier emits field-specific missing probe diagnostics',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_web_missing_envelope_expected_is_specific',
    ok: ingressSource.includes('typed_probe_contract_expected:web_search'),
    detail: 'typed web missing-envelope regression asserts web_search expected probe key',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_no_generic_execute_expected_in_regression',
    ok: !ingressSource.includes('typed_probe_contract_expected:execute_tool'),
    detail: 'typed regression suite does not collapse expected probe keys to execute_tool',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_no_execute_tool_collapse_in_classifier',
    ok: !classifierSource.includes('typed_probe_contract_expected:execute_tool'),
    detail: 'classifier does not emit execute_tool fallback diagnostics for typed probe routing',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_legacy_execute_tool_is_explicit_compatibility',
    ok: contractsSource.includes('Legacy compatibility capability retained for older probe payloads.'),
    detail: 'contracts surface keeps execute_tool only as explicit legacy compatibility',
  });

  const matrixRows = MATRIX_CAPABILITIES.map((row) => ({
    capability_key: row.key,
    expected_missing_capability_reason: `typed_probe_contract_missing:capability.${row.key}`,
    expected_missing_tool_field_reason: `typed_probe_contract_missing:field.${row.key}.tool_available`,
    expected_missing_transport_field_reason: `typed_probe_contract_missing:field.${row.key}.transport_available`,
  }));

  const ok = checks.every((row) => row.ok);
  const payload = {
    ok,
    strict: args.strict,
    checks,
    matrix_rows: matrixRows,
    generated_at: new Date().toISOString(),
  };

  const markdown = [
    '# TYPED PROBE CONTRACT MATRIX GUARD',
    '',
    `- ok: ${ok}`,
    `- strict: ${args.strict}`,
    '',
    '## Checks',
    ...checks.map(
      (row) => `- [${row.ok ? 'x' : ' '}] \`${row.id}\` — ${row.detail}`,
    ),
    '',
    '## Matrix Rows',
    '| Capability | Missing Capability Reason | Missing Tool Field | Missing Transport Field |',
    '| --- | --- | --- | --- |',
    ...matrixRows.map(
      (row) =>
        `| ${row.capability_key} | ${row.expected_missing_capability_reason} | ${row.expected_missing_tool_field_reason} | ${row.expected_missing_transport_field_reason} |`,
    ),
    '',
  ].join('\n');

  ensureParent(args.outJson);
  ensureParent(args.outMarkdown);
  writeFileSync(args.outJson, JSON.stringify(payload, null, 2));
  writeFileSync(args.outMarkdown, markdown);
  console.log(JSON.stringify(payload, null, 2));

  if (args.strict && !ok) return 1;
  return 0;
}

process.exit(run());
