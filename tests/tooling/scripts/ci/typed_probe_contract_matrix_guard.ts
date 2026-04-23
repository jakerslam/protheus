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

const EXPECTED_TYPED_ENUM_ORDER = [
  'WorkspaceRead',
  'WorkspaceSearch',
  'WebSearch',
  'WebFetch',
  'ToolRoute',
];

const EXPECTED_TYPED_KEY_ORDER = [
  'workspace_read',
  'workspace_search',
  'web_search',
  'web_fetch',
  'tool_route',
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
  const parserPath = resolve('surface/orchestration/src/ingress/parser.rs');
  const probeMatrixPath = resolve('surface/orchestration/tests/conformance/probe_matrix.rs');

  const classifierSource = readFileSync(classifierPath, 'utf8');
  const preconditionsSource = readFileSync(preconditionsPath, 'utf8');
  const contractsSource = readFileSync(contractsPath, 'utf8');
  const ingressSource = readFileSync(ingressPath, 'utf8');
  const parserSource = readFileSync(parserPath, 'utf8');
  const probeMatrixSource = readFileSync(probeMatrixPath, 'utf8');

  const checks: Check[] = [];
  const capabilityKeys = MATRIX_CAPABILITIES.map((row) => row.key);
  const capabilityEnums = MATRIX_CAPABILITIES.map((row) => row.enumName);
  const uniqueCapabilityKeyCount = new Set(capabilityKeys).size;
  const uniqueCapabilityEnumCount = new Set(capabilityEnums).size;
  const capabilityKeysJoined = capabilityKeys.join('|');
  const capabilityEnumsJoined = capabilityEnums.join('|');
  const expectedKeyOrderUniqueCount = new Set(EXPECTED_TYPED_KEY_ORDER).size;
  const expectedEnumOrderUniqueCount = new Set(EXPECTED_TYPED_ENUM_ORDER).size;
  const capabilityKeysSubsetExpected = capabilityKeys.every((key) =>
    EXPECTED_TYPED_KEY_ORDER.includes(key));
  const capabilityEnumsSubsetExpected = capabilityEnums.every((enumName) =>
    EXPECTED_TYPED_ENUM_ORDER.includes(enumName));
  const expectedKeysSubsetCapability = EXPECTED_TYPED_KEY_ORDER.every((key) =>
    capabilityKeys.includes(key));
  const expectedEnumsSubsetCapability = EXPECTED_TYPED_ENUM_ORDER.every((enumName) =>
    capabilityEnums.includes(enumName));
  const snakeCaseCapabilityKeys = capabilityKeys.filter((key) => /^[a-z]+(?:_[a-z0-9]+)*$/.test(key)).length;
  const pascalCaseCapabilityEnums = capabilityEnums.filter((enumName) =>
    /^[A-Z][A-Za-z0-9]*$/.test(enumName)).length;
  const matrixPairOrderAligned = MATRIX_CAPABILITIES.every((row, index) =>
    row.key === EXPECTED_TYPED_KEY_ORDER[index] && row.enumName === EXPECTED_TYPED_ENUM_ORDER[index]);

  checks.push({
    id: 'typed_probe_contract_matrix_capability_count_exactly_five',
    ok: MATRIX_CAPABILITIES.length === EXPECTED_TYPED_KEY_ORDER.length,
    detail: `typed capability matrix must carry exactly ${EXPECTED_TYPED_KEY_ORDER.length} capability families`,
  });

  checks.push({
    id: 'typed_probe_contract_matrix_capability_keys_unique',
    ok: uniqueCapabilityKeyCount === MATRIX_CAPABILITIES.length,
    detail: 'typed capability matrix keys must be unique',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_capability_enums_unique',
    ok: uniqueCapabilityEnumCount === MATRIX_CAPABILITIES.length,
    detail: 'typed capability matrix enum names must be unique',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_capability_key_order_canonical',
    ok: capabilityKeysJoined === EXPECTED_TYPED_KEY_ORDER.join('|'),
    detail: `typed capability matrix keys must match canonical order ${EXPECTED_TYPED_KEY_ORDER.join(',')}`,
  });

  checks.push({
    id: 'typed_probe_contract_matrix_capability_enum_order_canonical',
    ok: capabilityEnumsJoined === EXPECTED_TYPED_ENUM_ORDER.join('|'),
    detail: `typed capability matrix enums must match canonical order ${EXPECTED_TYPED_ENUM_ORDER.join(',')}`,
  });

  checks.push({
    id: 'typed_probe_contract_matrix_expected_key_order_unique',
    ok: expectedKeyOrderUniqueCount === EXPECTED_TYPED_KEY_ORDER.length,
    detail: 'expected typed capability key order list must not include duplicates',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_expected_enum_order_unique',
    ok: expectedEnumOrderUniqueCount === EXPECTED_TYPED_ENUM_ORDER.length,
    detail: 'expected typed capability enum order list must not include duplicates',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_expected_key_order_count_matches_matrix',
    ok: EXPECTED_TYPED_KEY_ORDER.length === MATRIX_CAPABILITIES.length,
    detail: 'expected typed capability key order length must match matrix capability count',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_expected_enum_order_count_matches_matrix',
    ok: EXPECTED_TYPED_ENUM_ORDER.length === MATRIX_CAPABILITIES.length,
    detail: 'expected typed capability enum order length must match matrix capability count',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_keys_subset_expected',
    ok: capabilityKeysSubsetExpected,
    detail: 'matrix capability keys must stay within expected typed capability key family',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_enums_subset_expected',
    ok: capabilityEnumsSubsetExpected,
    detail: 'matrix capability enums must stay within expected typed capability enum family',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_expected_keys_subset_matrix',
    ok: expectedKeysSubsetCapability,
    detail: 'expected typed capability keys must all be represented in capability matrix',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_expected_enums_subset_matrix',
    ok: expectedEnumsSubsetCapability,
    detail: 'expected typed capability enums must all be represented in capability matrix',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_keys_snake_case',
    ok: snakeCaseCapabilityKeys === capabilityKeys.length,
    detail: 'typed capability matrix keys must follow snake_case token contracts',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_enums_pascal_case',
    ok: pascalCaseCapabilityEnums === capabilityEnums.length,
    detail: 'typed capability matrix enum names must follow PascalCase token contracts',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_pair_order_alignment_canonical',
    ok: matrixPairOrderAligned,
    detail: 'typed capability key+enum pair ordering must align index-for-index with canonical lists',
  });

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

  for (const row of MATRIX_CAPABILITIES) {
    checks.push({
      id: `typed_probe_contract_matrix_ingress_missing_capability_reason_${row.key}`,
      ok: ingressSource.includes(`typed_probe_contract_missing:capability.${row.key}`),
      detail: `ingress regression includes explicit missing capability diagnostic for ${row.key}`,
    });
  }

  for (const row of MATRIX_CAPABILITIES) {
    checks.push({
      id: `typed_probe_contract_matrix_ingress_expected_probe_key_${row.key}`,
      ok: ingressSource.includes(`typed_probe_contract_expected:${row.key}`),
      detail: `ingress regression includes explicit expected-probe-key diagnostic for ${row.key}`,
    });
  }

  for (const row of MATRIX_CAPABILITIES) {
    const preconditionsCapabilityRegex = new RegExp(
      `"${reEscape(row.key)}"\\s*=>\\s*Some\\(Capability::${reEscape(row.enumName)}\\)`,
      'm',
    );
    checks.push({
      id: `typed_probe_contract_matrix_preconditions_capability_mapping_${row.key}`,
      ok: preconditionsCapabilityRegex.test(preconditionsSource),
      detail: `planner preconditions map ${row.key} to Capability::${row.enumName} via explicit authoritative capability lookup`,
    });
  }

  for (const row of MATRIX_CAPABILITIES) {
    const classifierProbeCapabilityRegex = new RegExp(
      `"${reEscape(row.key)}"\\s*=>\\s*Capability::${reEscape(row.enumName)}`,
      'm',
    );
    checks.push({
      id: `typed_probe_contract_matrix_classifier_capability_mapping_${row.key}`,
      ok: classifierProbeCapabilityRegex.test(classifierSource),
      detail: `classifier probe capability parsing maps ${row.key} to Capability::${row.enumName} without generic fallback collapse`,
    });
  }

  checks.push({
    id: 'typed_probe_contract_matrix_no_execute_tool_collapse_in_required_probe_key',
    ok: !/fn\s+required_probe_key[\s\S]*"execute_tool"/m.test(preconditionsSource),
    detail: 'required probe key function must not collapse tool-family authority onto execute_tool',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_conformance_no_tool_family_execute_tool_collapse',
    ok: !probeMatrixSource.includes('if capability.is_tool_family() {'),
    detail:
      'probe matrix conformance helper must not collapse typed tool-family probe keys to execute_tool',
  });

  const classifierRequiredContractCoverageCount = MATRIX_CAPABILITIES.filter((row) => {
    const contractRegex = new RegExp(
      `Capability::${reEscape(
        row.enumName,
      )}\\s*=>\\s*Some\\(\\(\\s*"${reEscape(row.key)}"\\s*,\\s*&\\[\\s*"tool_available"\\s*,\\s*"transport_available"\\s*\\]\\s*\\)\\)`,
      'm',
    );
    return contractRegex.test(classifierSource);
  }).length;
  checks.push({
    id: 'typed_probe_contract_matrix_required_contract_coverage_complete',
    ok: classifierRequiredContractCoverageCount === MATRIX_CAPABILITIES.length,
    detail: `classifier required-probe contract coverage must include all typed capability families (covered=${classifierRequiredContractCoverageCount}/${MATRIX_CAPABILITIES.length})`,
  });

  const contractsProbeKeyCoverageCount = MATRIX_CAPABILITIES.filter((row) => {
    const probeKeyRegex = new RegExp(
      `Capability::${reEscape(row.enumName)}\\s*=>\\s*&\\[\\s*"${reEscape(row.key)}"\\s*\\]`,
      'm',
    );
    return probeKeyRegex.test(contractsSource);
  }).length;
  checks.push({
    id: 'typed_probe_contract_matrix_contract_probe_key_coverage_complete',
    ok: contractsProbeKeyCoverageCount === MATRIX_CAPABILITIES.length,
    detail: `contracts probe-key matrix coverage must include all typed capability families (covered=${contractsProbeKeyCoverageCount}/${MATRIX_CAPABILITIES.length})`,
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
    id: 'typed_probe_contract_matrix_workspace_search_tool_field_reason',
    ok: ingressSource.includes('typed_probe_contract_missing:field.workspace_search.tool_available'),
    detail: 'ingress regression covers workspace_search missing tool field reason',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_web_fetch_tool_field_reason',
    ok: ingressSource.includes('typed_probe_contract_missing:field.web_fetch.tool_available'),
    detail: 'ingress regression covers web_fetch missing tool field reason',
  });

  const ingressReasonCoverageCount = MATRIX_CAPABILITIES.filter((row) => {
    const key = row.key;
    return (
      ingressSource.includes(`typed_probe_contract_missing:capability.${key}`)
      || ingressSource.includes(`typed_probe_contract_missing:field.${key}.tool_available`)
      || ingressSource.includes(`typed_probe_contract_missing:field.${key}.transport_available`)
    );
  }).length;
  const capabilityReasonCoverageCount = MATRIX_CAPABILITIES.filter((row) =>
    ingressSource.includes(`typed_probe_contract_missing:capability.${row.key}`)).length;
  const toolFieldReasonCoverageCount = MATRIX_CAPABILITIES.filter((row) =>
    ingressSource.includes(`typed_probe_contract_missing:field.${row.key}.tool_available`)).length;
  const transportFieldReasonCoverageCount = MATRIX_CAPABILITIES.filter((row) =>
    ingressSource.includes(`typed_probe_contract_missing:field.${row.key}.transport_available`)).length;
  const expectedProbeKeyCoverageCount = MATRIX_CAPABILITIES.filter((row) =>
    ingressSource.includes(`typed_probe_contract_expected:${row.key}`)).length;
  checks.push({
    id: 'typed_probe_contract_matrix_ingress_reason_surface_per_capability',
    ok: ingressReasonCoverageCount >= MATRIX_CAPABILITIES.length,
    detail: `ingress regression coverage should include capability/field-specific typed probe reasons across every capability family (covered=${ingressReasonCoverageCount}/${MATRIX_CAPABILITIES.length})`,
  });
  checks.push({
    id: 'typed_probe_contract_matrix_ingress_capability_reason_coverage_complete',
    ok: capabilityReasonCoverageCount >= MATRIX_CAPABILITIES.length,
    detail: `ingress regression coverage should include explicit missing-capability diagnostics across every capability family (covered=${capabilityReasonCoverageCount}/${MATRIX_CAPABILITIES.length})`,
  });
  checks.push({
    id: 'typed_probe_contract_matrix_ingress_tool_field_reason_coverage_complete',
    ok: toolFieldReasonCoverageCount >= 4,
    detail: `ingress regression coverage should include typed missing tool_available field diagnostics across web/workspace/tool-route capability families (covered=${toolFieldReasonCoverageCount};required>=4)`,
  });
  checks.push({
    id: 'typed_probe_contract_matrix_ingress_transport_field_reason_coverage_present',
    ok: transportFieldReasonCoverageCount >= 3,
    detail: `ingress regression coverage should include typed missing transport_available field diagnostics across multiple capability families (covered=${transportFieldReasonCoverageCount};required>=3)`,
  });
  checks.push({
    id: 'typed_probe_contract_matrix_ingress_expected_probe_key_coverage_complete',
    ok: expectedProbeKeyCoverageCount >= MATRIX_CAPABILITIES.length,
    detail: `ingress regression coverage should include explicit typed_probe_contract_expected diagnostics for each typed capability key (covered=${expectedProbeKeyCoverageCount}/${MATRIX_CAPABILITIES.length})`,
  });

  checks.push({
    id: 'typed_probe_contract_matrix_tool_route_capability_reason',
    ok: ingressSource.includes('typed_probe_contract_missing:capability.tool_route'),
    detail: 'ingress regression covers tool_route missing capability reason',
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

  const strictProbeFields = ['tool_available', 'transport_available'] as const;
  for (const row of MATRIX_CAPABILITIES) {
    for (const field of strictProbeFields) {
      const probeMatrixCaseRegex = new RegExp(
        `capability:\\s*Capability::${reEscape(row.enumName)}\\s*,\\s*missing_field:\\s*"${field}"`,
        'm',
      );
      checks.push({
        id: `typed_probe_contract_matrix_conformance_case_${row.key}_${field}`,
        ok: probeMatrixCaseRegex.test(probeMatrixSource),
        detail:
          `conformance matrix must include strict ${row.key} missing ${field} probe case`,
      });
    }
  }

  const expectedStrictMatrixRows = (MATRIX_CAPABILITIES.length * strictProbeFields.length) + 1 + 5 + 4;
  const expectedTotalExecutedCases = (expectedStrictMatrixRows * 4) + 2;
  checks.push({
    id: 'typed_probe_contract_matrix_conformance_case_count_canonical',
    ok: new RegExp(`executed_cases,\\s*${expectedTotalExecutedCases}`, 'm').test(probeMatrixSource),
    detail:
      `conformance matrix canonical executed-case count must remain ${expectedTotalExecutedCases} (strict surfaces + legacy compatibility cases)`,
  });

  checks.push({
    id: 'typed_probe_contract_matrix_legacy_execute_tool_is_explicit_compatibility',
    ok: contractsSource.includes('Legacy compatibility capability retained for older probe payloads.'),
    detail: 'contracts surface keeps execute_tool only as explicit legacy compatibility',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_workspace_directory_tokens',
    ok:
      parserSource.includes('"directory"')
      && parserSource.includes('"directories"')
      && parserSource.includes('"folder"')
      && parserSource.includes('"filesystem"')
      && parserSource.includes('"local"')
      && parserSource.includes('"repo"')
      && parserSource.includes('"repository"'),
    detail:
      'parser must classify directory/folder/local/filesystem vocabulary as workspace surface signals',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_workspace_disk_project_tokens',
    ok: parserSource.includes('"disk"') && parserSource.includes('"project"'),
    detail:
      'parser must classify disk/project vocabulary as workspace resource signals to avoid web/tool misrouting',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_workspace_read_verbs',
    ok:
      parserSource.includes('"list"')
      && parserSource.includes('"ls"')
      && parserSource.includes('"dir"')
      && parserSource.includes('"looking"')
      && parserSource.includes('"cat"')
      && parserSource.includes('"head"')
      && parserSource.includes('"tail"'),
    detail:
      'parser must classify local directory-style read verbs into read candidates for workspace signals',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_workspace_search_verbs',
    ok:
      parserSource.includes('"rg"')
      && parserSource.includes('"grep"')
      && parserSource.includes('"glob"')
      && parserSource.includes('"pattern"'),
    detail:
      'parser must classify local workspace search verbs (rg/grep/glob/pattern) into search candidates',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_local_workspace_hint_guard',
    ok:
      parserSource.includes('payload_local_workspace_intent')
      && parserSource.includes('payload_web_intent')
      && parserSource.includes('hints.retain(|hint| hint != "web_search" && hint != "web_fetch")'),
    detail:
      'parser must enforce local-workspace hint guard by stripping default web hints when no web intent exists',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_hint_alias_normalization',
    ok:
      parserSource.includes('normalize_tool_hint_alias')
      && parserSource.includes('"file_list"')
      && parserSource.includes('"workspace_search"'),
    detail:
      'parser tool hints must normalize alias tokens (e.g. file_list) into canonical typed capability keys',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_workspace_target_keys_directory_folder',
    ok:
      parserSource.includes('"workspace_path"')
      && parserSource.includes('"repo_path"')
      && parserSource.includes('"repository_path"')
      && parserSource.includes('"workspace_root"')
      && parserSource.includes('"repo_root"')
      && parserSource.includes('"root_path"')
      && parserSource.includes('"working_directory"')
      && parserSource.includes('"current_directory"')
      && parserSource.includes('"directory_path"')
      && parserSource.includes('"folder_path"')
      && parserSource.includes('"workspace_paths"')
      && parserSource.includes('"repo_paths"')
      && parserSource.includes('"repository_paths"')
      && parserSource.includes('"directories"')
      && parserSource.includes('"folders"'),
    detail: 'parser target extraction must accept directory/folder singular+plural keys as workspace targets',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_workspace_target_keys_cwd',
    ok: parserSource.includes('"cwd_path"') && parserSource.includes('"pwd_path"'),
    detail: 'parser workspace signal detection must include cwd_path payload keys for local file intents',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_workspace_cwd_pwd_tokens',
    ok: parserSource.includes('"cwd"') && parserSource.includes('"pwd"'),
    detail: 'parser workspace intent vocabulary must include cwd/pwd aliases to prevent local-routing misses',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_windows_path_detection',
    ok:
      parserSource.includes('looks_like_windows_drive_path')
      && parserSource.includes("trimmed.contains('\\\\')")
      && parserSource.includes("trimmed.starts_with(\"..\\\\\")"),
    detail:
      'parser generic target detection must classify Windows drive/backslash paths as workspace paths',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_parser_workspace_target_object_payloads',
    ok:
      parserSource.includes("extract_nested_target_scalar")
      && parserSource.includes('"value"')
      && parserSource.includes('"path"')
      && parserSource.includes('"workspace_path"')
      && parserSource.includes('"directory"')
      && parserSource.includes('"folder"'),
    detail:
      'parser target extraction must normalize object-shaped workspace payload values (value/path/directory/folder) to avoid file-route misses',
  });

  const matrixRows = MATRIX_CAPABILITIES.map((row) => ({
    capability_key: row.key,
    expected_missing_capability_reason: `typed_probe_contract_missing:capability.${row.key}`,
    expected_missing_tool_field_reason: `typed_probe_contract_missing:field.${row.key}.tool_available`,
    expected_missing_transport_field_reason: `typed_probe_contract_missing:field.${row.key}.transport_available`,
  }));
  const matrixRowCapabilityKeyCount = matrixRows.length;
  const matrixRowUniqueCapabilityKeyCount = new Set(matrixRows.map((row) => row.capability_key)).size;
  const matrixRowKeyOrderCanonical =
    matrixRows.map((row) => row.capability_key).join('|') === EXPECTED_TYPED_KEY_ORDER.join('|');
  const matrixRowsCapabilityReasonPatternCount = matrixRows.filter((row) =>
    row.expected_missing_capability_reason
      === `typed_probe_contract_missing:capability.${row.capability_key}`).length;
  const matrixRowsToolReasonPatternCount = matrixRows.filter((row) =>
    row.expected_missing_tool_field_reason
      === `typed_probe_contract_missing:field.${row.capability_key}.tool_available`).length;
  const matrixRowsTransportReasonPatternCount = matrixRows.filter((row) =>
    row.expected_missing_transport_field_reason
      === `typed_probe_contract_missing:field.${row.capability_key}.transport_available`).length;
  const matrixRowsCapabilityReasonUniqueCount = new Set(
    matrixRows.map((row) => row.expected_missing_capability_reason),
  ).size;
  const matrixRowsToolReasonUniqueCount = new Set(
    matrixRows.map((row) => row.expected_missing_tool_field_reason),
  ).size;
  const matrixRowsTransportReasonUniqueCount = new Set(
    matrixRows.map((row) => row.expected_missing_transport_field_reason),
  ).size;
  const matrixRowsNoLegacyExecuteToolReasons = matrixRows.every((row) =>
    !row.expected_missing_capability_reason.includes('execute_tool')
    && !row.expected_missing_tool_field_reason.includes('execute_tool')
    && !row.expected_missing_transport_field_reason.includes('execute_tool'));
  const defaultOutJsonCanonical =
    DEFAULT_OUT_JSON.startsWith('core/local/artifacts/')
    && DEFAULT_OUT_JSON.endsWith('_current.json')
    && !DEFAULT_OUT_JSON.includes('..')
    && !DEFAULT_OUT_JSON.includes('\\')
    && !DEFAULT_OUT_JSON.startsWith('/');
  const defaultOutMarkdownCanonical =
    DEFAULT_OUT_MARKDOWN.startsWith('local/workspace/reports/')
    && DEFAULT_OUT_MARKDOWN.endsWith('_CURRENT.md')
    && !DEFAULT_OUT_MARKDOWN.includes('..')
    && !DEFAULT_OUT_MARKDOWN.includes('\\')
    && !DEFAULT_OUT_MARKDOWN.startsWith('/');
  const matrixRowsIncludeLegacyExecuteTool = matrixRows.some((row) => row.capability_key === 'execute_tool');
  checks.push({
    id: 'typed_probe_contract_matrix_rows_cover_all_capabilities',
    ok: matrixRowCapabilityKeyCount === MATRIX_CAPABILITIES.length,
    detail: `typed probe matrix rows must include one row per typed capability family (rows=${matrixRowCapabilityKeyCount};capabilities=${MATRIX_CAPABILITIES.length})`,
  });
  checks.push({
    id: 'typed_probe_contract_matrix_rows_use_unique_capability_keys',
    ok: matrixRowUniqueCapabilityKeyCount === matrixRows.length,
    detail: 'typed probe matrix rows must not duplicate capability keys',
  });
  checks.push({
    id: 'typed_probe_contract_matrix_rows_exclude_legacy_execute_tool',
    ok: !matrixRowsIncludeLegacyExecuteTool,
    detail: 'typed probe matrix rows must not include legacy execute_tool capability key',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_rows_key_order_canonical',
    ok: matrixRowKeyOrderCanonical,
    detail: 'typed probe matrix rows must preserve canonical typed capability key ordering',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_rows_capability_reason_pattern_canonical',
    ok: matrixRowsCapabilityReasonPatternCount === matrixRows.length,
    detail: 'matrix row missing-capability reason templates must be canonical for every typed capability key',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_rows_tool_reason_pattern_canonical',
    ok: matrixRowsToolReasonPatternCount === matrixRows.length,
    detail: 'matrix row missing tool field reason templates must be canonical for every typed capability key',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_rows_transport_reason_pattern_canonical',
    ok: matrixRowsTransportReasonPatternCount === matrixRows.length,
    detail: 'matrix row missing transport field reason templates must be canonical for every typed capability key',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_rows_capability_reason_unique',
    ok: matrixRowsCapabilityReasonUniqueCount === matrixRows.length,
    detail: 'matrix row missing-capability reason values must be unique per typed capability key',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_rows_tool_reason_unique',
    ok: matrixRowsToolReasonUniqueCount === matrixRows.length,
    detail: 'matrix row missing tool field reason values must be unique per typed capability key',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_rows_transport_reason_unique',
    ok: matrixRowsTransportReasonUniqueCount === matrixRows.length,
    detail: 'matrix row missing transport field reason values must be unique per typed capability key',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_rows_reason_templates_exclude_legacy_execute_tool',
    ok: matrixRowsNoLegacyExecuteToolReasons,
    detail: 'matrix row reason templates must not regress to legacy execute_tool key surfaces',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_default_out_json_path_canonical',
    ok: defaultOutJsonCanonical,
    detail: 'default JSON artifact output path must remain canonical and release-proof safe',
  });

  checks.push({
    id: 'typed_probe_contract_matrix_default_out_markdown_path_canonical',
    ok: defaultOutMarkdownCanonical,
    detail: 'default markdown report output path must remain canonical and release-proof safe',
  });

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
