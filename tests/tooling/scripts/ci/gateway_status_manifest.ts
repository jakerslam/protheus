#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type SupportLevel = 'experimental' | 'candidate' | 'graduated';
type ChecklistStatus = 'pending' | 'in_progress' | 'complete';
type ChecklistKey =
  | 'health_checks'
  | 'fail_closed_behavior'
  | 'chaos_scenarios'
  | 'fallback_degradation_declaration'
  | 'receipt_completeness'
  | 'recovery_bounds';
type SupportLevelContractRow = {
  required_checklist_keys: ChecklistKey[];
  allowed_checklist_statuses: ChecklistStatus[];
};
type SupportLevelContract = Record<SupportLevel, SupportLevelContractRow>;
type ManifestSchema = {
  version: number;
  required_hooks: string[];
  required_scenarios: string[];
  support_level_contract?: Partial<Record<SupportLevel, {
    required_checklist_keys?: string[];
    allowed_checklist_statuses?: string[];
  }>>;
};

const ALLOWED_SUPPORT_LEVELS = new Set<SupportLevel>([
  'experimental',
  'candidate',
  'graduated',
]);
const ALLOWED_CHECKLIST_STATUS = new Set<ChecklistStatus>([
  'pending',
  'in_progress',
  'complete',
]);
const REQUIRED_CHECKLIST_KEYS = [
  'health_checks',
  'chaos_scenarios',
  'fallback_degradation_declaration',
  'receipt_completeness',
  'recovery_bounds',
] as const;
const REQUIRED_CHECKLIST_KEYS_WITH_FAIL_CLOSED = [
  'health_checks',
  'fail_closed_behavior',
  'chaos_scenarios',
  'fallback_degradation_declaration',
  'receipt_completeness',
  'recovery_bounds',
] as const;
const SUPPORT_LEVEL_ORDER: SupportLevel[] = [
  'experimental',
  'candidate',
  'graduated',
];
const EXPECTED_SUPPORT_LEVEL_CONTRACT: SupportLevelContract = {
  experimental: {
    required_checklist_keys: [...REQUIRED_CHECKLIST_KEYS],
    allowed_checklist_statuses: ['pending', 'in_progress', 'complete'],
  },
  candidate: {
    required_checklist_keys: [...REQUIRED_CHECKLIST_KEYS],
    allowed_checklist_statuses: ['in_progress', 'complete'],
  },
  graduated: {
    required_checklist_keys: [...REQUIRED_CHECKLIST_KEYS_WITH_FAIL_CLOSED],
    allowed_checklist_statuses: ['complete'],
  },
};
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
const EXPECTED_GATEWAY_TARGET_IDS = [
  'ollama',
  'llama_cpp',
  'mcp_baseline',
  'otlp_exporter',
  'durable_memory_local',
];

function normalizeStringArray(raw: unknown, maxLen = 80): string[] {
  return Array.isArray(raw)
    ? raw.map((value) => cleanText(String(value || ''), maxLen)).filter(Boolean)
    : [];
}

function cloneSupportLevelContract(contract: SupportLevelContract): SupportLevelContract {
  return {
    experimental: {
      required_checklist_keys: [...contract.experimental.required_checklist_keys],
      allowed_checklist_statuses: [...contract.experimental.allowed_checklist_statuses],
    },
    candidate: {
      required_checklist_keys: [...contract.candidate.required_checklist_keys],
      allowed_checklist_statuses: [...contract.candidate.allowed_checklist_statuses],
    },
    graduated: {
      required_checklist_keys: [...contract.graduated.required_checklist_keys],
      allowed_checklist_statuses: [...contract.graduated.allowed_checklist_statuses],
    },
  };
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/gateway_status_manifest_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    markdownPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/GATEWAY_STATUS_MANIFEST_CURRENT.md',
      400,
    ),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'tests/tooling/config/gateway_graduation_manifest.json',
      400,
    ),
    supportLevelsPath: cleanText(
      readFlag(argv, 'support-levels') ||
        'core/local/artifacts/gateway_support_levels_current.json',
      400,
    ),
    snapshotPath: cleanText(
      readFlag(argv, 'out-snapshot') ||
        'core/local/artifacts/gateway_graduation_status_snapshot_current.json',
      400,
    ),
    snapshotMarkdownPath: cleanText(
      readFlag(argv, 'out-snapshot-markdown') ||
        'local/workspace/reports/GATEWAY_GRADUATION_STATUS_SNAPSHOT_CURRENT.md',
      400,
    ),
  };
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function normalizeStatus(raw: unknown): string {
  return cleanText(String(raw || ''), 80).toLowerCase();
}

function renderMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Gateway Status Manifest (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`- pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- target_gateway_count: ${Number(payload?.summary?.target_gateway_count || 0)}`);
  lines.push(`- validated_gateway_count: ${Number(payload?.summary?.validated_gateway_count || 0)}`);
  lines.push(
    `- graduated_ready_count: ${Number(payload?.summary?.graduated_ready_count || 0)}`,
  );
  lines.push(`- failure_count: ${Number(payload?.summary?.failure_count || 0)}`);
  lines.push('');
  lines.push('## Target Gateways');
  const rows = Array.isArray(payload?.targets) ? payload.targets : [];
  if (rows.length === 0) {
    lines.push('- none');
  } else {
    lines.push('| id | support_level | checklist_uniform | checklist_ready | owner | blocker |');
    lines.push('| --- | --- | --- | --- | --- | --- |');
    for (const row of rows) {
      lines.push(
        `| ${cleanText(row?.id || '', 80)} | ${cleanText(
          row?.support_level || '',
          40,
        )} | ${row?.uniform_checklist_contract_ok === true ? 'true' : 'false'} | ${
          row?.support_level_readiness_ok === true ? 'true' : 'false'
        } | ${cleanText(row?.owner || '', 120)} | ${cleanText(row?.blocker || '', 160)} |`,
      );
    }
  }
  const failures = Array.isArray(payload?.failures) ? payload.failures : [];
  if (failures.length > 0) {
    lines.push('');
    lines.push('## Failures');
    for (const failure of failures) {
      lines.push(
        `- ${cleanText(failure?.id || 'unknown', 120)}: ${cleanText(
          failure?.detail || '',
          240,
        )}`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function writeMarkdown(filePath: string, body: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, body, 'utf8');
}

function writeJson(filePath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function renderSnapshotMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Gateway Graduation Status Snapshot (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`- pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- target_gateway_count: ${Number(payload?.summary?.target_gateway_count || 0)}`);
  lines.push(`- graduated_ready_count: ${Number(payload?.summary?.graduated_ready_count || 0)}`);
  lines.push(`- candidate_ready_count: ${Number(payload?.summary?.candidate_ready_count || 0)}`);
  lines.push(`- experimental_ready_count: ${Number(payload?.summary?.experimental_ready_count || 0)}`);
  lines.push(`- failure_count: ${Number(payload?.summary?.failure_count || 0)}`);
  lines.push('');
  lines.push('## Support Level Counts');
  const counts = payload?.summary?.support_level_counts || {};
  lines.push(`- graduated: ${Number(counts.graduated || 0)}`);
  lines.push(`- candidate: ${Number(counts.candidate || 0)}`);
  lines.push(`- experimental: ${Number(counts.experimental || 0)}`);
  lines.push('');
  lines.push('## Gateways');
  const rows = Array.isArray(payload?.gateways) ? payload.gateways : [];
  if (rows.length === 0) {
    lines.push('- none');
  } else {
    lines.push('| id | support_level | readiness_ok | owner | blocker |');
    lines.push('| --- | --- | --- | --- | --- |');
    for (const row of rows) {
      lines.push(
        `| ${cleanText(row?.id || '', 80)} | ${cleanText(
          row?.support_level || '',
          40,
        )} | ${row?.support_level_readiness_ok === true ? 'true' : 'false'} | ${cleanText(
          row?.owner || '',
          120,
        )} | ${cleanText(row?.blocker || '', 160)} |`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const manifest = readJsonBestEffort(path.resolve(root, args.manifestPath));
  const supportLevelsPayload = readJsonBestEffort(path.resolve(root, args.supportLevelsPath));
  const failures: Array<{ id: string; detail: string }> = [];

  const supportLevelContract: SupportLevelContract = cloneSupportLevelContract(
    EXPECTED_SUPPORT_LEVEL_CONTRACT,
  );

  if (!manifest) {
    failures.push({
      id: 'gateway_manifest_missing',
      detail: args.manifestPath,
    });
  } else {
    const manifestVersion = Number((manifest as ManifestSchema).version || 0);
    if (manifestVersion !== EXPECTED_MANIFEST_VERSION) {
      failures.push({
        id: 'gateway_manifest_version_invalid',
        detail: Number.isFinite(manifestVersion) ? String(manifestVersion) : 'missing',
      });
    }
    const requiredHooks = Array.isArray((manifest as ManifestSchema).required_hooks)
      ? (manifest as ManifestSchema).required_hooks.map((value) => cleanText(String(value || ''), 80))
      : [];
    const requiredScenarios = Array.isArray((manifest as ManifestSchema).required_scenarios)
      ? (manifest as ManifestSchema).required_scenarios.map((value) => cleanText(String(value || ''), 80))
      : [];
    if (requiredHooks.join('|') !== EXPECTED_REQUIRED_HOOKS.join('|')) {
      failures.push({
        id: 'gateway_manifest_required_hooks_noncanonical',
        detail: requiredHooks.join(',') || 'missing',
      });
    }
    if (requiredScenarios.join('|') !== EXPECTED_REQUIRED_SCENARIOS.join('|')) {
      failures.push({
        id: 'gateway_manifest_required_scenarios_noncanonical',
        detail: requiredScenarios.join(',') || 'missing',
      });
    }

    const rawSupportLevelContract = (manifest as ManifestSchema).support_level_contract;
    if (!rawSupportLevelContract || typeof rawSupportLevelContract !== 'object') {
      failures.push({
        id: 'gateway_manifest_support_level_contract_missing',
        detail: 'support_level_contract',
      });
    } else {
      const declaredLevels = Object.keys(rawSupportLevelContract)
        .map((value) => cleanText(String(value || ''), 40).toLowerCase())
        .filter(Boolean);
      if (declaredLevels.join('|') !== SUPPORT_LEVEL_ORDER.join('|')) {
        failures.push({
          id: 'gateway_manifest_support_level_contract_levels_noncanonical',
          detail: declaredLevels.join(',') || 'missing',
        });
      }

      for (const level of SUPPORT_LEVEL_ORDER) {
        const contractRow = rawSupportLevelContract[level];
        if (!contractRow || typeof contractRow !== 'object') {
          failures.push({
            id: 'gateway_manifest_support_level_contract_row_missing',
            detail: level,
          });
          continue;
        }

        const requiredChecklistKeys = normalizeStringArray(
          contractRow.required_checklist_keys,
          80,
        );
        const allowedChecklistStatuses = normalizeStringArray(
          contractRow.allowed_checklist_statuses,
          40,
        );

        const expectedRequired = EXPECTED_SUPPORT_LEVEL_CONTRACT[level].required_checklist_keys;
        if (requiredChecklistKeys.join('|') !== expectedRequired.join('|')) {
          failures.push({
            id: 'gateway_manifest_support_level_contract_required_keys_noncanonical',
            detail: `${level}:${requiredChecklistKeys.join(',') || 'missing'}`,
          });
        }

        const expectedAllowed = EXPECTED_SUPPORT_LEVEL_CONTRACT[level].allowed_checklist_statuses;
        if (allowedChecklistStatuses.join('|') !== expectedAllowed.join('|')) {
          failures.push({
            id: 'gateway_manifest_support_level_contract_allowed_statuses_noncanonical',
            detail: `${level}:${allowedChecklistStatuses.join(',') || 'missing'}`,
          });
        }

        const invalidRequiredKeys = requiredChecklistKeys.filter((key) =>
          !REQUIRED_CHECKLIST_KEYS_WITH_FAIL_CLOSED.includes(key as ChecklistKey),
        );
        if (invalidRequiredKeys.length > 0) {
          failures.push({
            id: 'gateway_manifest_support_level_contract_required_keys_invalid',
            detail: `${level}:${invalidRequiredKeys.join(',')}`,
          });
        }

        const invalidAllowedStatuses = allowedChecklistStatuses.filter((status) =>
          !ALLOWED_CHECKLIST_STATUS.has(status as ChecklistStatus),
        );
        if (invalidAllowedStatuses.length > 0) {
          failures.push({
            id: 'gateway_manifest_support_level_contract_allowed_statuses_invalid',
            detail: `${level}:${invalidAllowedStatuses.join(',')}`,
          });
        }

        if (invalidRequiredKeys.length === 0 && invalidAllowedStatuses.length === 0) {
          supportLevelContract[level] = {
            required_checklist_keys: requiredChecklistKeys as ChecklistKey[],
            allowed_checklist_statuses: allowedChecklistStatuses as ChecklistStatus[],
          };
        }
      }
    }
  }

  const targetIds: string[] = Array.isArray(manifest?.production_gateway_targets)
    ? manifest.production_gateway_targets
        .map((value: unknown) => cleanText(String(value || ''), 80))
        .filter(Boolean)
    : [];
  const duplicateTargetIds = targetIds.filter((id, index, arr) => arr.indexOf(id) !== index);
  if (duplicateTargetIds.length > 0) {
    failures.push({
      id: 'gateway_manifest_target_ids_duplicate',
      detail: Array.from(new Set(duplicateTargetIds)).join(','),
    });
  }
  const invalidTargetIds = targetIds.filter((id) => !/^[a-z0-9_]+$/.test(id));
  if (invalidTargetIds.length > 0) {
    failures.push({
      id: 'gateway_manifest_target_ids_noncanonical',
      detail: invalidTargetIds.join(','),
    });
  }
  if (targetIds.join('|') !== EXPECTED_GATEWAY_TARGET_IDS.join('|')) {
    failures.push({
      id: 'gateway_manifest_target_ids_noncanonical_order_or_set',
      detail: targetIds.join(',') || 'missing',
    });
  }
  if (targetIds.length === 0) {
    failures.push({
      id: 'gateway_manifest_target_ids_missing',
      detail: 'production_gateway_targets',
    });
  }

  const adaptersById = new Map<string, any>();
  for (const row of Array.isArray(manifest?.adapters) ? manifest.adapters : []) {
    const id = cleanText(row?.id || '', 80);
    if (!id || adaptersById.has(id)) continue;
    adaptersById.set(id, row);
  }
  const supportRowsById = new Map<string, any>();
  const supportRowsRaw = Array.isArray(supportLevelsPayload?.gateway_support_levels)
    ? supportLevelsPayload.gateway_support_levels
    : [];
  if (!supportLevelsPayload) {
    failures.push({
      id: 'gateway_support_levels_payload_missing',
      detail: args.supportLevelsPath,
    });
  } else {
    const supportType = cleanText(supportLevelsPayload?.type || '', 80);
    if (supportType !== 'gateway_support_levels') {
      failures.push({
        id: 'gateway_support_levels_payload_type_invalid',
        detail: supportType || 'missing',
      });
    }
  }
  const duplicateSupportIds = supportRowsRaw
    .map((row: any) => cleanText(row?.id || '', 80))
    .filter(Boolean)
    .filter((id, index, arr) => arr.indexOf(id) !== index);
  if (duplicateSupportIds.length > 0) {
    failures.push({
      id: 'gateway_support_levels_duplicate_id',
      detail: Array.from(new Set(duplicateSupportIds)).join(','),
    });
  }
  for (const row of supportRowsRaw) {
    const id = cleanText(row?.id || '', 80);
    if (!id || supportRowsById.has(id)) continue;
    supportRowsById.set(id, row);
  }
  const extraSupportIds = Array.from(supportRowsById.keys()).filter((id) => !targetIds.includes(id));
  if (extraSupportIds.length > 0) {
    failures.push({
      id: 'gateway_support_levels_unknown_target',
      detail: extraSupportIds.join(','),
    });
  }
  const missingSupportIds = targetIds.filter((id) => !supportRowsById.has(id));
  if (missingSupportIds.length > 0) {
    failures.push({
      id: 'gateway_support_levels_target_missing',
      detail: missingSupportIds.join(','),
    });
  }

  const targets = targetIds.map((id) => {
    const adapter = adaptersById.get(id) || {};
    if (!adaptersById.has(id)) {
      failures.push({
        id: 'gateway_target_missing_from_manifest_adapters',
        detail: id,
      });
    }
    const supportRow = supportRowsById.get(id) || {};
    const supportLevel = normalizeStatus(
      supportRow.support_level || adapter.support_level,
    ) as SupportLevel;
    const manifestSupportLevel = normalizeStatus(adapter.support_level || '') as SupportLevel;
    const supportPayloadLevel = normalizeStatus(supportRow.support_level || '') as SupportLevel;
    if (!ALLOWED_SUPPORT_LEVELS.has(supportLevel)) {
      failures.push({
        id: 'gateway_support_level_invalid',
        detail: `${id}:${supportLevel || 'missing'}`,
      });
    }
    if (
      supportPayloadLevel
      && manifestSupportLevel
      && supportPayloadLevel !== manifestSupportLevel
    ) {
      failures.push({
        id: 'gateway_support_level_mismatch_manifest_vs_support_payload',
        detail: `${id}:${manifestSupportLevel}->${supportPayloadLevel}`,
      });
    }

    const checklistRaw =
      typeof adapter.checklist === 'object' && adapter.checklist ? adapter.checklist : {};
    const checklistStatuses = REQUIRED_CHECKLIST_KEYS_WITH_FAIL_CLOSED.map((key) => ({
      key,
      status: normalizeStatus((checklistRaw as any)[key]) as ChecklistStatus,
    }));
    for (const row of checklistStatuses) {
      if (!ALLOWED_CHECKLIST_STATUS.has(row.status)) {
        failures.push({
          id: 'gateway_checklist_status_invalid',
          detail: `${id}:${row.key}:${row.status || 'missing'}`,
        });
      }
    }

    const uniformChecklistContractOk = REQUIRED_CHECKLIST_KEYS_WITH_FAIL_CLOSED.every((key) =>
      ALLOWED_CHECKLIST_STATUS.has(normalizeStatus((checklistRaw as any)[key]) as ChecklistStatus),
    );
    if (!uniformChecklistContractOk) {
      failures.push({
        id: 'gateway_uniform_checklist_contract_incomplete',
        detail: id,
      });
    }

    const statusByKey = new Map<string, ChecklistStatus>(
      checklistStatuses.map((row) => [row.key, row.status]),
    );
    const readinessContract = supportLevelContract[supportLevel];
    const requiredForReadiness = readinessContract.required_checklist_keys;
    const allowedStatusesForReadiness = new Set<ChecklistStatus>(
      readinessContract.allowed_checklist_statuses,
    );
    const supportLevelReadinessOk = requiredForReadiness.every((key) => {
      const status = statusByKey.get(key) || 'pending';
      return allowedStatusesForReadiness.has(status);
    });

    if (!supportLevelReadinessOk) {
      failures.push({
        id: 'gateway_support_level_checklist_status_mismatch',
        detail: `${id}:${supportLevel}`,
      });
    }

    return {
      id,
      support_level: supportLevel,
      readiness_track: cleanText(
        supportRow.readiness_track || adapter.readiness_track || '',
        80,
      ),
      tier: cleanText(adapter.tier || '', 40),
      owner: cleanText(supportRow.owner || adapter.owner || '', 120),
      blocker: cleanText(supportRow.blocker || adapter.blocker || '', 180),
      checklist: Object.fromEntries(
        checklistStatuses.map((row) => [row.key, row.status]),
      ),
      uniform_checklist_contract_ok: uniformChecklistContractOk,
      support_level_readiness_ok: supportLevelReadinessOk,
    };
  });

  const payload = {
    ok: failures.length === 0,
    type: 'gateway_status_manifest',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    manifest_path: args.manifestPath,
    support_levels_path: args.supportLevelsPath,
    summary: {
      pass: failures.length === 0,
      target_gateway_count: targetIds.length,
      validated_gateway_count: targets.length,
      graduated_ready_count: targets.filter(
        (row) => row.support_level === 'graduated' && row.support_level_readiness_ok,
      ).length,
      failure_count: failures.length,
    },
    targets,
    failures,
  };

  const supportLevelCounts = targets.reduce(
    (acc, row) => {
      if (row.support_level === 'graduated') acc.graduated += 1;
      else if (row.support_level === 'candidate') acc.candidate += 1;
      else acc.experimental += 1;
      return acc;
    },
    {
      graduated: 0,
      candidate: 0,
      experimental: 0,
    },
  );
  const snapshot = {
    ok: payload.ok,
    type: 'gateway_graduation_status_snapshot',
    generated_at: payload.generated_at,
    revision: payload.revision,
    manifest_path: payload.manifest_path,
    support_levels_path: payload.support_levels_path,
    source_artifact: args.outPath,
    summary: {
      pass: payload.ok,
      target_gateway_count: targets.length,
      graduated_ready_count: targets.filter(
        (row) => row.support_level === 'graduated' && row.support_level_readiness_ok,
      ).length,
      candidate_ready_count: targets.filter(
        (row) => row.support_level === 'candidate' && row.support_level_readiness_ok,
      ).length,
      experimental_ready_count: targets.filter(
        (row) => row.support_level === 'experimental' && row.support_level_readiness_ok,
      ).length,
      support_level_counts: supportLevelCounts,
      failure_count: failures.length,
    },
    gateways: targets,
    failures,
  };

  writeJson(path.resolve(root, args.snapshotPath), snapshot);
  writeMarkdown(path.resolve(root, args.snapshotMarkdownPath), renderSnapshotMarkdown(snapshot));
  writeMarkdown(path.resolve(root, args.markdownPath), renderMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
