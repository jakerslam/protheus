#!/usr/bin/env tsx

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';

type SupportLevel = 'experimental' | 'candidate' | 'graduated';

type ChecklistStatus = 'pending' | 'in_progress' | 'complete';
type ChecklistKey =
  | 'health_checks'
  | 'fail_closed_behavior'
  | 'chaos_scenarios'
  | 'receipt_completeness'
  | 'fallback_degradation_declaration'
  | 'recovery_bounds';
type SupportLevelContractRow = {
  required_checklist_keys: ChecklistKey[];
  allowed_checklist_statuses: ChecklistStatus[];
};
type SupportLevelContract = Record<SupportLevel, SupportLevelContractRow>;

type GatewayChecklist = {
  health_checks: ChecklistStatus;
  fail_closed_behavior: ChecklistStatus;
  chaos_scenarios: ChecklistStatus;
  receipt_completeness: ChecklistStatus;
  fallback_degradation_declaration: ChecklistStatus;
  recovery_bounds: ChecklistStatus;
};

type AdapterLane = {
  id: string;
  bridgeCommand: string;
  framework: string;
  tier: string;
  readinessTrack: string;
  requiredForGraduation: boolean;
  supportLevel: SupportLevel;
  owner: string;
  blocker: string;
  checklist: GatewayChecklist;
};

type ScenarioRow = {
  adapter: string;
  scenario: string;
  expected_error: string;
  ok: boolean;
  detail: string;
  runtime_circuit_state?: string;
  runtime_quarantine_active?: boolean;
  transition_enforced?: boolean;
  transition_ok?: boolean;
};

type GraduationManifest = {
  version: number;
  required_hooks: string[];
  required_scenarios: string[];
  support_level_contract?: Partial<Record<SupportLevel, {
    required_checklist_keys?: string[];
    allowed_checklist_statuses?: string[];
  }>>;
  production_gateway_targets?: string[];
  adapters: Array<{
    id: string;
    framework: string;
    bridge_command: string;
    tier?: string;
    readiness_track?: string;
    required_for_graduation?: boolean;
    support_level?: string;
    owner?: string;
    blocker?: string;
    checklist?: Partial<GatewayChecklist>;
    notes?: string;
  }>;
};

type AdapterGraduationResult = {
  adapter: string;
  graduated: boolean;
  manifest_declared: boolean;
  missing_scenarios: string[];
  hook_pass_count: number;
  hook_total: number;
  hook_pass_ratio: number;
  scenario_pass_count: number;
  scenario_total: number;
  scenario_pass_ratio: number;
  hooks: Array<{ id: string; ok: boolean; detail: string }>;
};

const OPS_CARGO_BUILD_ARGS = ['build', '-q', '-p', 'infring-ops-core', '--bin', 'infring-ops'];
const BRIDGE_MAX_BUFFER_BYTES = 64 * 1024 * 1024;
const DEFAULT_OUT_JSON = 'core/local/artifacts/gateway_runtime_chaos_gate_current.json';
const DEFAULT_SUPPORT_LEVELS_OUT_JSON = 'core/local/artifacts/gateway_support_levels_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/GATEWAY_RUNTIME_CHAOS_GATE_CURRENT.md';

const DEFAULT_TARGET_GATEWAY_ADAPTER_IDS = [
  'ollama',
  'llama_cpp',
  'mcp_baseline',
  'otlp_exporter',
  'durable_memory_local',
] as const;
const EXPECTED_MANIFEST_VERSION = 3;
const EXPECTED_REQUIRED_HOOKS = [
  'health_check',
  'startup_timeout_policy',
  'request_timeout_policy',
  'fail_closed_policy_hooks',
  'receipt_schema_helpers',
  'circuit_breaker_behavior',
  'quarantine_hooks',
];
const EXPECTED_REQUIRED_SCENARIOS = [
  'process_never_starts',
  'starts_then_hangs',
  'invalid_schema_response',
  'response_too_large',
  'repeated_flapping',
];
const CHECKLIST_ALLOWED_STATUS = new Set<ChecklistStatus>(['pending', 'in_progress', 'complete']);
const CHECKLIST_KEYS: Array<keyof GatewayChecklist> = [
  'health_checks',
  'fail_closed_behavior',
  'chaos_scenarios',
  'receipt_completeness',
  'fallback_degradation_declaration',
  'recovery_bounds',
];
const SUPPORT_LEVEL_ORDER: SupportLevel[] = ['experimental', 'candidate', 'graduated'];
const EXPECTED_SUPPORT_LEVEL_CONTRACT: SupportLevelContract = {
  experimental: {
    required_checklist_keys: [
      'health_checks',
      'chaos_scenarios',
      'fallback_degradation_declaration',
      'receipt_completeness',
      'recovery_bounds',
    ],
    allowed_checklist_statuses: ['pending', 'in_progress', 'complete'],
  },
  candidate: {
    required_checklist_keys: [
      'health_checks',
      'chaos_scenarios',
      'fallback_degradation_declaration',
      'receipt_completeness',
      'recovery_bounds',
    ],
    allowed_checklist_statuses: ['in_progress', 'complete'],
  },
  graduated: {
    required_checklist_keys: [
      'health_checks',
      'fail_closed_behavior',
      'chaos_scenarios',
      'fallback_degradation_declaration',
      'receipt_completeness',
      'recovery_bounds',
    ],
    allowed_checklist_statuses: ['complete'],
  },
};

const CHAOS_CASES = [
  { id: 'process_never_starts', expected_error: 'adapter_startup_timeout' },
  { id: 'starts_then_hangs', expected_error: 'adapter_request_timeout' },
  { id: 'invalid_schema_response', expected_error: 'adapter_invalid_schema' },
  { id: 'response_too_large', expected_error: 'adapter_response_too_large' },
  { id: 'repeated_flapping', expected_error: 'adapter_circuit_open' },
];

function parseProfile(raw: string | undefined): ProfileId | null {
  const normalized = cleanText(raw || 'rich', 32).toLowerCase();
  if (normalized === 'rich') return 'rich';
  if (normalized === 'pure') return 'pure';
  if (normalized === 'tiny-max' || normalized === 'tiny' || normalized === 'tiny_max') return 'tiny-max';
  return null;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: DEFAULT_OUT_JSON,
  });
  const profile = parseProfile(readFlag(argv, 'profile'));
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    outSupportLevelsPath: cleanText(
      readFlag(argv, 'out-support-levels') || DEFAULT_SUPPORT_LEVELS_OUT_JSON,
      400,
    ),
    outMarkdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
    profile,
    graduationManifestPath: cleanText(
      readFlag(argv, 'graduation-manifest') || 'tests/tooling/config/gateway_graduation_manifest.json',
      400,
    ),
  };
}

function resolveGatewayTargetAdapterIds(manifest: GraduationManifest): string[] {
  const declared = Array.isArray(manifest.production_gateway_targets)
    ? manifest.production_gateway_targets
    : [];
  const normalized = declared
    .map((row) => cleanText(String(row || ''), 80))
    .filter(Boolean);
  if (normalized.length === 0) {
    return [...DEFAULT_TARGET_GATEWAY_ADAPTER_IDS];
  }
  return Array.from(new Set(normalized));
}

function toMarkdown(report: any, supportLevels: any): string {
  const lines: string[] = [];
  lines.push('# Gateway Runtime Chaos Gate');
  lines.push('');
  lines.push(`- generated_at: ${report.generated_at}`);
  lines.push(`- revision: ${report.revision}`);
  lines.push(`- profile: ${report.profile}`);
  lines.push(`- pass: ${report.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- adapters_total: ${report.summary.adapters_total}`);
  lines.push(`- adapters_required_for_graduation: ${report.summary.adapters_required_for_graduation}`);
  lines.push(`- adapters_pending_roadmap: ${report.summary.adapters_pending_roadmap}`);
  lines.push(`- baseline_passed: ${report.summary.baseline_passed}/${report.summary.baseline_total}`);
  lines.push(
    `- chaos_fail_closed_passed: ${report.summary.chaos_fail_closed_passed}/${report.summary.chaos_total} (${report.summary.chaos_fail_closed_ratio})`,
  );
  lines.push(
    `- graduation_passed: ${report.summary.graduation_passed}/${report.summary.adapters_required_for_graduation} (${report.summary.graduation_ratio})`,
  );
  lines.push(`- manifest_violations: ${report.summary.manifest_violations}`);
  lines.push('');
  lines.push('## Gateway Support Levels');
  if (!Array.isArray(supportLevels.gateway_support_levels) || supportLevels.gateway_support_levels.length === 0) {
    lines.push('- none');
  } else {
    lines.push('| id | support_level | readiness_track | owner | blocker |');
    lines.push('| --- | --- | --- | --- | --- |');
    for (const row of supportLevels.gateway_support_levels) {
      lines.push(
        `| ${cleanText(row.id || '', 80)} | ${cleanText(row.support_level || '', 40)} | ${cleanText(
          row.readiness_track || '',
          80,
        )} | ${cleanText(row.owner || '', 120)} | ${cleanText(row.blocker || '', 200)} |`,
      );
    }
  }
  lines.push('');
  lines.push('## Manifest Violations');
  if (!Array.isArray(report.graduation_policy?.manifest_violations) || report.graduation_policy.manifest_violations.length === 0) {
    lines.push('- none');
  } else {
    for (const violation of report.graduation_policy.manifest_violations) {
      lines.push(`- ${cleanText(violation || '', 200)}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function parseLastJson(raw: string): any {
  const lines = String(raw || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    try {
      return JSON.parse(lines[idx]);
    } catch {
      // keep scanning
    }
  }
  return null;
}

type OpsBinary = {
  command: string;
  argsPrefix: string[];
};

function resolveOpsBinary(root: string): OpsBinary {
  const explicit = cleanText(process.env.INFRING_OPS_BIN || '', 400);
  if (explicit && fs.existsSync(path.resolve(root, explicit))) {
    return { command: path.resolve(root, explicit), argsPrefix: [] };
  }
  const defaultBin = path.resolve(root, 'target/debug/infring-ops');
  const skipBuild = cleanText(process.env.INFRING_ADAPTER_CHAOS_SKIP_BUILD || '', 8) === '1';
  if (!skipBuild) {
    const build = spawnSync('cargo', OPS_CARGO_BUILD_ARGS, {
      cwd: root,
      encoding: 'utf8',
    });
    if ((build.status ?? 1) !== 0) {
      throw new Error(
        cleanText(
          `adapter_runtime_chaos_build_failed:status=${build.status};stderr=${String(build.stderr || '')}`,
          600,
        ),
      );
    }
  } else if (!fs.existsSync(defaultBin)) {
    throw new Error('adapter_runtime_chaos_binary_missing_when_build_skipped');
  }
  return { command: defaultBin, argsPrefix: [] };
}

function encodePayload(payload: unknown): string {
  return Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
}

function runBridgeCommand(
  root: string,
  opsBinary: OpsBinary,
  adapter: AdapterLane,
  payload: Record<string, unknown>,
  statePath: string,
) {
  const encodedPayload = encodePayload(payload);
  const proc = spawnSync(
    opsBinary.command,
    opsBinary.argsPrefix.concat([
      adapter.bridgeCommand,
      'run-governed-workflow',
      `--payload-base64=${encodedPayload}`,
      `--state-path=${statePath}`,
    ]),
    {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: BRIDGE_MAX_BUFFER_BYTES,
    },
  );
  const spawnError = cleanText(proc.error?.message || '', 280);
  return {
    exitCode: proc.status ?? 1,
    stdout: String(proc.stdout || ''),
    stderr: String(proc.stderr || ''),
    spawnError,
    payload: parseLastJson(String(proc.stdout || '')),
  };
}

function scenarioStatePath(tempRoot: string, adapterId: string, scenarioId: string): string {
  const safeAdapter = adapterId.replace(/[^a-zA-Z0-9_-]/g, '_');
  const safeScenario = scenarioId.replace(/[^a-zA-Z0-9_-]/g, '_');
  return path.join(tempRoot, `${safeAdapter}_${safeScenario}.json`);
}

function baselinePayload(adapter: AdapterLane) {
  return {
    task_id: `gateway_chaos_baseline_${adapter.id}`,
    trace_id: `gateway_chaos_trace_${adapter.id}`,
    tool_name: 'web_search',
    tool_args: {
      query: `gateway chaos baseline ${adapter.id}`,
    },
    raw_result: {
      results: [
        {
          source: `${adapter.id}_runtime`,
          title: `baseline-${adapter.id}`,
          summary: 'baseline governed workflow path',
        },
      ],
    },
  };
}

function chaosPayload(adapter: AdapterLane, scenarioId: string) {
  return {
    task_id: `gateway_chaos_${adapter.id}_${scenarioId}`,
    trace_id: `gateway_chaos_trace_${adapter.id}_${scenarioId}`,
    tool_name: 'web_search',
    chaos_scenario: scenarioId,
    tool_args: {
      query: `gateway chaos ${scenarioId}`,
    },
  };
}

function loadGraduationManifest(root: string, relPath: string): GraduationManifest {
  const abs = path.resolve(root, relPath);
  const parsed = JSON.parse(fs.readFileSync(abs, 'utf8')) as GraduationManifest;
  return parsed;
}

function normalizeSupportLevel(raw: string | undefined): SupportLevel {
  const normalized = cleanText(raw || '', 40).toLowerCase();
  if (normalized === 'experimental') return 'experimental';
  if (normalized === 'graduated') return 'graduated';
  return 'candidate';
}

function normalizeChecklistStatus(raw: string | undefined): ChecklistStatus {
  const normalized = cleanText(raw || '', 40).toLowerCase();
  if (normalized === 'complete') return 'complete';
  if (normalized === 'in_progress') return 'in_progress';
  return 'pending';
}

function normalizeStringArray(raw: unknown, maxLen = 80): string[] {
  return Array.isArray(raw)
    ? raw.map((value) => cleanText(String(value || ''), maxLen)).filter(Boolean)
    : [];
}

function parseErrorToken(rawError: string, key: string): string {
  const safeKey = key.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = rawError.match(new RegExp(`${safeKey}=([a-zA-Z0-9_-]+)`));
  return cleanText(match?.[1] || '', 40).toLowerCase();
}

function readRuntimeCircuitState(payload: any, chaosError: string): string {
  const fromPayload = cleanText(payload?.runtime_contract?.circuit_breaker?.state || '', 40).toLowerCase();
  if (fromPayload) {
    return fromPayload;
  }
  return parseErrorToken(chaosError, 'runtime_circuit_state');
}

function readRuntimeQuarantineActive(payload: any, chaosError: string): boolean {
  if (payload?.runtime_contract?.quarantine?.active === true) {
    return true;
  }
  const token = parseErrorToken(chaosError, 'runtime_quarantine_active');
  return token === 'true';
}

function normalizeChecklist(row: GraduationManifest['adapters'][number]): GatewayChecklist {
  const raw = (row?.checklist || {}) as Record<string, unknown>;
  return {
    health_checks: normalizeChecklistStatus(typeof raw.health_checks === 'string' ? raw.health_checks : undefined),
    fail_closed_behavior: normalizeChecklistStatus(
      typeof raw.fail_closed_behavior === 'string' ? raw.fail_closed_behavior : undefined,
    ),
    chaos_scenarios: normalizeChecklistStatus(typeof raw.chaos_scenarios === 'string' ? raw.chaos_scenarios : undefined),
    receipt_completeness: normalizeChecklistStatus(
      typeof raw.receipt_completeness === 'string' ? raw.receipt_completeness : undefined,
    ),
    fallback_degradation_declaration: normalizeChecklistStatus(
      typeof raw.fallback_degradation_declaration === 'string' ? raw.fallback_degradation_declaration : undefined,
    ),
    recovery_bounds: normalizeChecklistStatus(
      typeof raw.recovery_bounds === 'string' ? raw.recovery_bounds : undefined,
    ),
  };
}

function adaptersFromManifest(manifest: GraduationManifest): AdapterLane[] {
  const rows = Array.isArray(manifest.adapters) ? manifest.adapters : [];
  const dedup = new Set<string>();
  const out: AdapterLane[] = [];
  for (const row of rows) {
    const id = cleanText(row?.id || '', 80);
    const framework = cleanText(row?.framework || '', 80);
    const bridgeCommand = cleanText(row?.bridge_command || '', 120);
    if (!id || !framework || !bridgeCommand || dedup.has(id)) continue;
    dedup.add(id);
    out.push({
      id,
      framework,
      bridgeCommand,
      tier: cleanText(row?.tier || 'candidate', 40).toLowerCase(),
      readinessTrack: cleanText(row?.readiness_track || 'unclassified', 80),
      requiredForGraduation: row?.required_for_graduation !== false,
      supportLevel: normalizeSupportLevel(row?.support_level),
      owner: cleanText(row?.owner || '', 120),
      blocker: cleanText(row?.blocker || '', 200),
      checklist: normalizeChecklist(row),
    });
  }
  return out;
}

function hasScenarioPass(chaosRows: ScenarioRow[], adapterId: string, scenarioId: string): boolean {
  return chaosRows.some((row) => row.adapter === adapterId && row.scenario === scenarioId && row.ok);
}

function buildGraduationResults(
  graduationAdapters: AdapterLane[],
  manifest: GraduationManifest,
  baselineRows: ScenarioRow[],
  chaosRows: ScenarioRow[],
): AdapterGraduationResult[] {
  const manifestByAdapter = new Map(
    (manifest.adapters || []).map((row) => [cleanText(row.id || '', 80), row]),
  );
  const requiredScenarios = Array.isArray(manifest.required_scenarios) ? manifest.required_scenarios : [];
  const requiredHooks = Array.isArray(manifest.required_hooks) ? manifest.required_hooks : [];

  return graduationAdapters.map((adapter) => {
    const baselineRow = baselineRows.find((row) => row.adapter === adapter.id);
    const declared = manifestByAdapter.has(adapter.id);
    const scenarioPassCount = requiredScenarios.filter((id) => hasScenarioPass(chaosRows, adapter.id, id)).length;
    const missingScenarios = requiredScenarios.filter((id) => !hasScenarioPass(chaosRows, adapter.id, id));
    const receiptSchemaOk =
      Boolean(baselineRow?.detail) &&
      chaosRows
        .filter((row) => row.adapter === adapter.id)
        .every((row) => Boolean(row.expected_error) && Boolean(row.detail));
    const allFailClosed = requiredScenarios.every((scenarioId) => {
      const row = chaosRows.find((item) => item.adapter === adapter.id && item.scenario === scenarioId);
      return Boolean(row?.ok) && cleanText(row?.detail || '', 80).startsWith('fail_closed:');
    });

    const hookChecks = requiredHooks.map((hookId) => {
      if (hookId === 'health_check') {
        return {
          id: hookId,
          ok: baselineRow?.ok === true,
          detail: baselineRow?.ok === true ? 'baseline_ok' : cleanText(baselineRow?.detail || 'baseline_missing', 180),
        };
      }
      if (hookId === 'startup_timeout_policy') {
        const ok = hasScenarioPass(chaosRows, adapter.id, 'process_never_starts');
        return { id: hookId, ok, detail: ok ? 'scenario_pass' : 'missing_or_failed:process_never_starts' };
      }
      if (hookId === 'request_timeout_policy') {
        const ok = hasScenarioPass(chaosRows, adapter.id, 'starts_then_hangs');
        return { id: hookId, ok, detail: ok ? 'scenario_pass' : 'missing_or_failed:starts_then_hangs' };
      }
      if (hookId === 'fail_closed_policy_hooks') {
        return {
          id: hookId,
          ok: allFailClosed,
          detail: allFailClosed ? 'all_required_scenarios_fail_closed' : 'fail_closed_coverage_incomplete',
        };
      }
      if (hookId === 'receipt_schema_helpers') {
        return {
          id: hookId,
          ok: receiptSchemaOk,
          detail: receiptSchemaOk ? 'receipt_fields_present' : 'receipt_fields_missing',
        };
      }
      if (hookId === 'circuit_breaker_behavior') {
        const ok = hasScenarioPass(chaosRows, adapter.id, 'repeated_flapping');
        return { id: hookId, ok, detail: ok ? 'scenario_pass' : 'missing_or_failed:repeated_flapping' };
      }
      if (hookId === 'quarantine_hooks') {
        const ok = hasScenarioPass(chaosRows, adapter.id, 'repeated_flapping');
        return { id: hookId, ok, detail: ok ? 'scenario_pass' : 'missing_or_failed:repeated_flapping' };
      }
      return {
        id: hookId,
        ok: false,
        detail: 'unsupported_hook_id_in_manifest',
      };
    });

    const hookPassCount = hookChecks.filter((row) => row.ok).length;
    const hookTotal = hookChecks.length;
    const scenarioTotal = requiredScenarios.length;
    const graduated = declared && hookPassCount === hookTotal && scenarioPassCount === scenarioTotal;

    return {
      adapter: adapter.id,
      graduated,
      manifest_declared: declared,
      missing_scenarios: missingScenarios,
      hook_pass_count: hookPassCount,
      hook_total: hookTotal,
      hook_pass_ratio: hookTotal === 0 ? 0 : Number((hookPassCount / hookTotal).toFixed(4)),
      scenario_pass_count: scenarioPassCount,
      scenario_total: scenarioTotal,
      scenario_pass_ratio: scenarioTotal === 0 ? 0 : Number((scenarioPassCount / scenarioTotal).toFixed(4)),
      hooks: hookChecks,
    };
  });
}

function manifestConformanceViolations(
  manifest: GraduationManifest,
  adapters: AdapterLane[],
  targetGatewayIds: readonly string[],
): string[] {
  const violations: string[] = [];
  if (!Number.isInteger(manifest.version)) {
    violations.push('manifest_version_missing_or_non_integer');
  } else if (manifest.version !== EXPECTED_MANIFEST_VERSION) {
    violations.push(`manifest_version_invalid:${manifest.version}`);
  }
  const manifestRows = Array.isArray(manifest.adapters) ? manifest.adapters : [];
  const ids = manifestRows.map((row) => cleanText(row?.id || '', 80)).filter(Boolean);
  const duplicates = ids.filter((id, idx) => ids.indexOf(id) !== idx);
  duplicates.forEach((id) => violations.push(`manifest_duplicate_adapter_id:${id}`));

  const requiredHooks = Array.isArray(manifest.required_hooks)
    ? manifest.required_hooks.map((row) => cleanText(String(row || ''), 80)).filter(Boolean)
    : [];
  const duplicateRequiredHooks = requiredHooks.filter((id, index, arr) => arr.indexOf(id) !== index);
  duplicateRequiredHooks.forEach((id) => violations.push(`manifest_required_hook_duplicate:${id}`));
  if (requiredHooks.length === 0) {
    violations.push('manifest_required_hooks_missing_or_empty');
  } else if (requiredHooks.join('|') !== EXPECTED_REQUIRED_HOOKS.join('|')) {
    violations.push('manifest_required_hooks_noncanonical_order_or_set');
  }
  const requiredScenarios = Array.isArray(manifest.required_scenarios)
    ? manifest.required_scenarios.map((row) => cleanText(String(row || ''), 80)).filter(Boolean)
    : [];
  const duplicateRequiredScenarios = requiredScenarios.filter((id, index, arr) => arr.indexOf(id) !== index);
  duplicateRequiredScenarios.forEach((id) => violations.push(`manifest_required_scenario_duplicate:${id}`));
  if (requiredScenarios.length === 0) {
    violations.push('manifest_required_scenarios_missing_or_empty');
  } else if (requiredScenarios.join('|') !== EXPECTED_REQUIRED_SCENARIOS.join('|')) {
    violations.push('manifest_required_scenarios_noncanonical_order_or_set');
  }

  const supportLevelContract: SupportLevelContract = {
    experimental: {
      required_checklist_keys: [...EXPECTED_SUPPORT_LEVEL_CONTRACT.experimental.required_checklist_keys],
      allowed_checklist_statuses: [...EXPECTED_SUPPORT_LEVEL_CONTRACT.experimental.allowed_checklist_statuses],
    },
    candidate: {
      required_checklist_keys: [...EXPECTED_SUPPORT_LEVEL_CONTRACT.candidate.required_checklist_keys],
      allowed_checklist_statuses: [...EXPECTED_SUPPORT_LEVEL_CONTRACT.candidate.allowed_checklist_statuses],
    },
    graduated: {
      required_checklist_keys: [...EXPECTED_SUPPORT_LEVEL_CONTRACT.graduated.required_checklist_keys],
      allowed_checklist_statuses: [...EXPECTED_SUPPORT_LEVEL_CONTRACT.graduated.allowed_checklist_statuses],
    },
  };
  const rawSupportLevelContract = manifest.support_level_contract;
  if (!rawSupportLevelContract || typeof rawSupportLevelContract !== 'object') {
    violations.push('manifest_support_level_contract_missing');
  } else {
    const declaredLevels = Object.keys(rawSupportLevelContract)
      .map((value) => cleanText(String(value || ''), 40).toLowerCase())
      .filter(Boolean);
    if (declaredLevels.join('|') !== SUPPORT_LEVEL_ORDER.join('|')) {
      violations.push('manifest_support_level_contract_levels_noncanonical');
    }
    for (const level of SUPPORT_LEVEL_ORDER) {
      const row = rawSupportLevelContract[level];
      if (!row || typeof row !== 'object') {
        violations.push(`manifest_support_level_contract_row_missing:${level}`);
        continue;
      }
      const requiredKeys = normalizeStringArray(row.required_checklist_keys, 80);
      const allowedStatuses = normalizeStringArray(row.allowed_checklist_statuses, 40);
      const expectedRequired = EXPECTED_SUPPORT_LEVEL_CONTRACT[level].required_checklist_keys;
      const expectedAllowed = EXPECTED_SUPPORT_LEVEL_CONTRACT[level].allowed_checklist_statuses;

      if (requiredKeys.join('|') !== expectedRequired.join('|')) {
        violations.push(`manifest_support_level_contract_required_keys_noncanonical:${level}`);
      }
      if (allowedStatuses.join('|') !== expectedAllowed.join('|')) {
        violations.push(`manifest_support_level_contract_allowed_statuses_noncanonical:${level}`);
      }

      const invalidRequiredKeys = requiredKeys.filter(
        (key) => !CHECKLIST_KEYS.includes(key as ChecklistKey),
      );
      if (invalidRequiredKeys.length > 0) {
        violations.push(`manifest_support_level_contract_required_keys_invalid:${level}`);
      }
      const invalidAllowedStatuses = allowedStatuses.filter(
        (status) => !CHECKLIST_ALLOWED_STATUS.has(status as ChecklistStatus),
      );
      if (invalidAllowedStatuses.length > 0) {
        violations.push(`manifest_support_level_contract_allowed_statuses_invalid:${level}`);
      }

      if (invalidRequiredKeys.length === 0 && invalidAllowedStatuses.length === 0) {
        supportLevelContract[level] = {
          required_checklist_keys: requiredKeys as ChecklistKey[],
          allowed_checklist_statuses: allowedStatuses as ChecklistStatus[],
        };
      }
    }
  }

  const byId = new Map(manifestRows.map((row) => [cleanText(row.id || '', 80), row]));
  const readinessTracks = new Set<string>();
  const requiredFlags = new Set<string>();
  const duplicateTargetIds = targetGatewayIds.filter((id, index, arr) => arr.indexOf(id) !== index);
  duplicateTargetIds.forEach((id) => violations.push(`manifest_gateway_target_duplicate:${id}`));
  const invalidTargetIds = targetGatewayIds.filter((id) => !/^[a-z0-9_]+$/.test(id));
  invalidTargetIds.forEach((id) => violations.push(`manifest_gateway_target_noncanonical:${id}`));
  if (targetGatewayIds.join('|') !== DEFAULT_TARGET_GATEWAY_ADAPTER_IDS.join('|')) {
    violations.push('manifest_gateway_target_noncanonical_order_or_set');
  }

  if (targetGatewayIds.length === 0) {
    violations.push('manifest_gateway_targets_missing_or_empty');
  }

  for (const gatewayId of targetGatewayIds) {
    const declared = byId.get(gatewayId);
    if (!declared) {
      violations.push(`manifest_missing_gateway_target:${gatewayId}`);
      continue;
    }

    const readinessTrack = cleanText(declared.readiness_track || '', 80);
    readinessTracks.add(readinessTrack || '__missing__');
    if (readinessTrack !== 'gateway_production_v1') {
      violations.push(`manifest_gateway_readiness_track_invalid:${gatewayId}`);
    }
    if (typeof declared.required_for_graduation !== 'boolean') {
      violations.push(`manifest_gateway_required_for_graduation_missing:${gatewayId}`);
    }
    if (declared.required_for_graduation !== false) {
      violations.push(`manifest_gateway_target_must_not_require_graduation:${gatewayId}`);
    }
    requiredFlags.add(String(declared.required_for_graduation !== false));

    const tier = cleanText(declared.tier || '', 40).toLowerCase();
    if (!(tier === 'candidate' || tier === 'production')) {
      violations.push(`manifest_gateway_tier_invalid:${gatewayId}`);
    }

    const supportLevel = cleanText(declared.support_level || '', 40).toLowerCase();
    if (!(supportLevel === 'experimental' || supportLevel === 'candidate' || supportLevel === 'graduated')) {
      violations.push(`manifest_gateway_support_level_invalid:${gatewayId}`);
    } else if (supportLevel === 'experimental') {
      violations.push(`manifest_gateway_support_level_experimental_not_allowed:${gatewayId}`);
    }

    const owner = cleanText(declared.owner || '', 120);
    if (!owner) {
      violations.push(`manifest_gateway_owner_missing:${gatewayId}`);
    }
    const blocker = cleanText(declared.blocker || '', 200);
    if (supportLevel === 'graduated' && blocker) {
      violations.push(`manifest_gateway_blocker_must_be_empty_when_graduated:${gatewayId}`);
    }
    if (supportLevel !== 'graduated' && !blocker) {
      violations.push(`manifest_gateway_blocker_missing:${gatewayId}`);
    }

    const checklistRaw = (declared.checklist || {}) as Record<string, unknown>;
    for (const checklistKey of CHECKLIST_KEYS) {
      const status = cleanText(String(checklistRaw[checklistKey] || ''), 40).toLowerCase();
      if (!CHECKLIST_ALLOWED_STATUS.has(status as ChecklistStatus)) {
        violations.push(`manifest_gateway_checklist_status_invalid:${gatewayId}:${checklistKey}`);
      }
    }
    const statusByChecklistKey = new Map<ChecklistKey, ChecklistStatus>();
    for (const checklistKey of CHECKLIST_KEYS) {
      const status = cleanText(String(checklistRaw[checklistKey] || ''), 40).toLowerCase() as ChecklistStatus;
      statusByChecklistKey.set(checklistKey, status);
    }
    const readinessContract = supportLevelContract[supportLevel];
    if (!readinessContract) {
      violations.push(`manifest_gateway_support_level_contract_missing:${gatewayId}:${supportLevel}`);
    } else {
      const allowedStatuses = new Set<ChecklistStatus>(readinessContract.allowed_checklist_statuses);
      for (const checklistKey of readinessContract.required_checklist_keys) {
        const status = statusByChecklistKey.get(checklistKey) || 'pending';
        if (!allowedStatuses.has(status)) {
          violations.push(
            `manifest_gateway_checklist_readiness_mismatch:${gatewayId}:${supportLevel}:${checklistKey}:${status}`,
          );
        }
      }
    }
  }

  if (targetGatewayIds.length > 0 && readinessTracks.size > 1) {
    violations.push('manifest_gateway_readiness_track_mismatch');
  }
  if (targetGatewayIds.length > 0 && requiredFlags.size > 1) {
    violations.push('manifest_gateway_required_for_graduation_mismatch');
  }
  if (adapters.length === 0) {
    violations.push('manifest_contains_no_executable_adapters');
  }
  return violations;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  if (!args.profile) {
    const payload = {
      ok: false,
      type: 'gateway_runtime_chaos_gate',
      error: 'runtime_proof_profile_invalid',
      profile: cleanText(readFlag(argv, 'profile') || '', 40),
      allowed_profiles: ['rich', 'pure', 'tiny-max'],
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  let graduationManifest: GraduationManifest;
  try {
    graduationManifest = loadGraduationManifest(root, args.graduationManifestPath);
  } catch (error) {
    const payload = {
      ok: false,
      type: 'gateway_runtime_chaos_gate',
      error: 'gateway_graduation_manifest_unavailable',
      detail: cleanText(error instanceof Error ? error.message : String(error), 500),
      manifest_path: args.graduationManifestPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const manifestAdapters = adaptersFromManifest(graduationManifest);
  const gatewayTargetIds = resolveGatewayTargetAdapterIds(graduationManifest);
  const manifestAdapterById = new Map(manifestAdapters.map((row) => [row.id, row] as const));
  const gatewayTargetAdapters = gatewayTargetIds
    .map((id) => manifestAdapterById.get(id))
    .filter((row): row is AdapterLane => Boolean(row));
  const gatewaySupportLevels = gatewayTargetAdapters.map((row) => ({
    id: row.id,
    support_level: row.supportLevel,
    owner: row.owner || '',
    blocker: row.blocker || '',
    checklist: row.checklist,
    readiness_track: row.readinessTrack,
    required_for_graduation: row.requiredForGraduation,
  }));
  const graduationAdapters = manifestAdapters.filter((row) => row.requiredForGraduation);
  if (graduationAdapters.length === 0) {
    const payload = {
      ok: false,
      type: 'gateway_runtime_chaos_gate',
      error: 'gateway_graduation_manifest_has_no_required_adapters',
      manifest_path: args.graduationManifestPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  let opsBinary: OpsBinary;
  try {
    opsBinary = resolveOpsBinary(root);
  } catch (error) {
    const payload = {
      ok: false,
      type: 'gateway_runtime_chaos_gate',
      error: 'adapter_runtime_chaos_binary_unavailable',
      detail: cleanText(error instanceof Error ? error.message : String(error), 500),
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const baselineRows: ScenarioRow[] = [];
  const chaosRows: ScenarioRow[] = [];
  const runtimeStateRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-gateway-chaos-'));

  for (const adapter of graduationAdapters) {
    const baseline = runBridgeCommand(
      root,
      opsBinary,
      adapter,
      baselinePayload(adapter),
      scenarioStatePath(runtimeStateRoot, adapter.id, 'baseline'),
    );
    const baselineOk = baseline.exitCode === 0 && baseline.payload?.ok === true;
    baselineRows.push({
      adapter: adapter.id,
      scenario: 'baseline',
      expected_error: '',
      ok: baselineOk,
      detail: baselineOk
        ? 'baseline_ok'
        : cleanText(
            `${baseline.payload?.error || 'missing_ok'};exit=${baseline.exitCode};stderr=${baseline.stderr};spawn_error=${
              baseline.spawnError || 'none'
            }`,
            280,
          ),
    });

    for (const chaosCase of CHAOS_CASES) {
      const chaosRun = runBridgeCommand(
        root,
        opsBinary,
        adapter,
        chaosPayload(adapter, chaosCase.id),
        scenarioStatePath(runtimeStateRoot, adapter.id, chaosCase.id),
      );
      const chaosError = cleanText(chaosRun.payload?.error || '', 120);
      const runtimeCircuitState = readRuntimeCircuitState(chaosRun.payload, chaosError);
      const runtimeQuarantineActive = readRuntimeQuarantineActive(chaosRun.payload, chaosError);
      const transitionEnforced = chaosCase.id === 'repeated_flapping';
      const transitionOk = !transitionEnforced || (
        runtimeCircuitState === 'open' && runtimeQuarantineActive === true
      );
      const chaosOk =
        chaosRun.exitCode !== 0 &&
        chaosRun.payload?.ok === false &&
        chaosError.includes(chaosCase.expected_error) &&
        transitionOk;
      chaosRows.push({
        adapter: adapter.id,
        scenario: chaosCase.id,
        expected_error: chaosCase.expected_error,
        ok: chaosOk,
        detail: chaosOk
          ? transitionEnforced
            ? `fail_closed:${chaosError};circuit=${runtimeCircuitState || 'missing'};quarantine=${String(runtimeQuarantineActive)}`
            : `fail_closed:${chaosError}`
          : cleanText(
              `${chaosError || 'missing_error'};exit=${chaosRun.exitCode};stderr=${chaosRun.stderr};spawn_error=${
                chaosRun.spawnError || 'none'
              };circuit=${runtimeCircuitState || 'missing'};quarantine=${String(runtimeQuarantineActive)}`,
              280,
            ),
        runtime_circuit_state: runtimeCircuitState || 'missing',
        runtime_quarantine_active: runtimeQuarantineActive,
        transition_enforced: transitionEnforced,
        transition_ok: transitionOk,
      });
    }
  }

  const requiredScenarioIds = normalizeStringArray(graduationManifest.required_scenarios, 80);
  const configuredChaosCaseIds = CHAOS_CASES.map((row) => cleanText(row.id, 80));
  const missingConfiguredChaosCases = requiredScenarioIds.filter(
    (scenarioId) => !configuredChaosCaseIds.includes(scenarioId),
  );
  const extraConfiguredChaosCases = configuredChaosCaseIds.filter(
    (scenarioId) => !requiredScenarioIds.includes(scenarioId),
  );
  const chaosCaseSetOk =
    missingConfiguredChaosCases.length === 0 &&
    extraConfiguredChaosCases.length === 0 &&
    configuredChaosCaseIds.join('|') === requiredScenarioIds.join('|');
  const missingScenarioRows: string[] = [];
  for (const adapter of graduationAdapters) {
    for (const scenarioId of requiredScenarioIds) {
      const hasRow = chaosRows.some((row) => row.adapter === adapter.id && row.scenario === scenarioId);
      if (!hasRow) {
        missingScenarioRows.push(`${adapter.id}:${scenarioId}`);
      }
    }
  }
  const transitionRows = chaosRows.filter((row) => row.transition_enforced === true);
  const transitionPassed = transitionRows.filter((row) => row.transition_ok === true).length;
  const transitionTotal = transitionRows.length;
  const transitionRatio = transitionTotal === 0 ? 0 : transitionPassed / transitionTotal;

  const baselinePassed = baselineRows.filter((row) => row.ok).length;
  const chaosPassed = chaosRows.filter((row) => row.ok).length;
  const allRows = baselineRows.concat(chaosRows);
  const baselineTotal = baselineRows.length;
  const chaosTotal = chaosRows.length;
  const baselinePassRatio = baselineTotal === 0 ? 0 : baselinePassed / baselineTotal;
  const chaosFailClosedRatio = chaosTotal === 0 ? 0 : chaosPassed / chaosTotal;

  const graduationResults = buildGraduationResults(
    graduationAdapters,
    graduationManifest,
    baselineRows,
    chaosRows,
  );
  const graduationPassed = graduationResults.filter((row) => row.graduated).length;
  const graduationRatio = graduationResults.length === 0 ? 0 : graduationPassed / graduationResults.length;
  const manifestViolations = manifestConformanceViolations(
    graduationManifest,
    graduationAdapters,
    gatewayTargetIds,
  );
  const pendingRoadmapAdapters = manifestAdapters
    .filter((row) => !row.requiredForGraduation)
    .map((row) => ({
      id: row.id,
      framework: row.framework,
      bridge_command: row.bridgeCommand,
      tier: row.tier,
      readiness_track: row.readinessTrack,
      support_level: row.supportLevel,
      owner: row.owner || '',
      blocker: row.blocker || '',
      checklist: row.checklist,
    }));

  const failures = allRows
    .filter((row) => !row.ok)
    .map((row) => ({
      id: `${row.adapter}:${row.scenario}`,
      detail: row.detail,
    }))
    .concat(
      graduationResults
        .filter((row) => !row.graduated)
        .map((row) => ({
          id: `adapter_graduation_failed:${row.adapter}`,
          detail: `missing_scenarios=${row.missing_scenarios.join(',') || 'none'};hook_pass=${row.hook_pass_count}/${row.hook_total};manifest_declared=${String(row.manifest_declared)}`,
        })),
    )
    .concat(
      manifestViolations.map((detail) => ({
        id: 'gateway_graduation_manifest_violation',
        detail,
      })),
    );
  if (!chaosCaseSetOk) {
    failures.push({
      id: 'gateway_chaos_case_set_mismatch',
      detail: `required=${requiredScenarioIds.join(',') || 'missing'};configured=${configuredChaosCaseIds.join(',') || 'missing'};missing=${missingConfiguredChaosCases.join(',') || 'none'};extra=${extraConfiguredChaosCases.join(',') || 'none'}`,
    });
  }
  for (const missingRow of missingScenarioRows) {
    failures.push({
      id: 'gateway_chaos_scenario_result_missing',
      detail: missingRow,
    });
  }
  for (const row of transitionRows.filter((item) => item.transition_ok !== true)) {
    failures.push({
      id: 'gateway_chaos_transition_contract_violation',
      detail: `${row.adapter}:${row.scenario}:circuit=${row.runtime_circuit_state || 'missing'};quarantine=${String(row.runtime_quarantine_active === true)}`,
    });
  }

  const metrics = {
    gateway_baseline_pass_ratio: Number(baselinePassRatio.toFixed(4)),
    gateway_chaos_fail_closed_ratio: Number(chaosFailClosedRatio.toFixed(4)),
    gateway_chaos_scenarios_total: chaosTotal,
    gateway_graduation_ratio: Number(graduationRatio.toFixed(4)),
    gateway_chaos_transition_ratio: Number(transitionRatio.toFixed(4)),
  };

  const report = {
    ok: failures.length === 0,
    type: 'gateway_runtime_chaos_gate',
    profile: args.profile,
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    graduation_manifest_path: args.graduationManifestPath,
    summary: {
      adapters_total: manifestAdapters.length,
      gateway_targets_expected: gatewayTargetIds.length,
      gateway_targets_total: gatewayTargetAdapters.length,
      gateway_support_levels_published: gatewaySupportLevels.length,
      adapters_required_for_graduation: graduationAdapters.length,
      adapters_pending_roadmap: pendingRoadmapAdapters.length,
      baseline_total: baselineTotal,
      baseline_passed: baselinePassed,
      chaos_total: chaosTotal,
      chaos_required_scenarios_declared: requiredScenarioIds.length,
      chaos_case_ids_configured: configuredChaosCaseIds.length,
      chaos_case_set_ok: chaosCaseSetOk,
      chaos_fail_closed_passed: chaosPassed,
      chaos_fail_closed_ratio: Number(chaosFailClosedRatio.toFixed(4)),
      chaos_transition_total: transitionTotal,
      chaos_transition_passed: transitionPassed,
      chaos_transition_ratio: Number(transitionRatio.toFixed(4)),
      graduation_passed: graduationPassed,
      graduation_ratio: Number(graduationRatio.toFixed(4)),
      manifest_violations: manifestViolations.length,
    },
    metrics,
    adapters: manifestAdapters.map((row) => ({
      id: row.id,
      bridge_command: row.bridgeCommand,
      framework: row.framework,
      tier: row.tier,
      readiness_track: row.readinessTrack,
      required_for_graduation: row.requiredForGraduation,
      support_level: row.supportLevel,
      owner: row.owner || '',
      blocker: row.blocker || '',
      checklist: row.checklist,
    })),
    gateway_support_levels: gatewaySupportLevels,
    pending_roadmap_adapters: pendingRoadmapAdapters,
    baseline_results: baselineRows,
    chaos_results: chaosRows,
    chaos_transition_results: transitionRows.map((row) => ({
      adapter: row.adapter,
      scenario: row.scenario,
      transition_ok: row.transition_ok === true,
      runtime_circuit_state: row.runtime_circuit_state || 'missing',
      runtime_quarantine_active: row.runtime_quarantine_active === true,
      detail: row.detail,
    })),
    graduation_results: graduationResults,
    graduation_policy: {
      version: graduationManifest.version,
      production_gateway_targets: gatewayTargetIds,
      required_hooks: graduationManifest.required_hooks || [],
      required_scenarios: graduationManifest.required_scenarios || [],
      configured_chaos_case_ids: configuredChaosCaseIds,
      chaos_case_set_ok: chaosCaseSetOk,
      manifest_violations: manifestViolations,
    },
    failures,
  };

  const supportLevelsReport = {
    ok: report.ok,
    type: 'gateway_support_levels',
    generated_at: report.generated_at,
    revision: report.revision,
    profile: report.profile,
    summary: {
      gateway_targets_expected: report.summary.gateway_targets_expected,
      gateway_targets_total: report.summary.gateway_targets_total,
      support_levels_published: report.summary.gateway_support_levels_published,
      pending_roadmap_adapters: report.summary.adapters_pending_roadmap,
      manifest_violations: report.summary.manifest_violations,
    },
    gateway_support_levels: gatewaySupportLevels,
    pending_roadmap_adapters: pendingRoadmapAdapters,
  };
  writeJsonArtifact(args.outSupportLevelsPath, supportLevelsReport);
  writeTextArtifact(args.outMarkdownPath, toMarkdown(report, supportLevelsReport));

  if (cleanText(process.env.INFRING_ADAPTER_CHAOS_KEEP_TMP || '', 8) !== '1') {
    try {
      fs.rmSync(runtimeStateRoot, {
        recursive: true,
        force: true,
      });
    } catch {
      // best effort cleanup only
    }
  }

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
