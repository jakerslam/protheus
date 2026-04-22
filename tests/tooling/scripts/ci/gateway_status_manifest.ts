#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type SupportLevel = 'experimental' | 'candidate' | 'graduated';
type ChecklistStatus = 'pending' | 'in_progress' | 'complete';

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

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const manifest = readJsonBestEffort(path.resolve(root, args.manifestPath));
  const supportLevelsPayload = readJsonBestEffort(path.resolve(root, args.supportLevelsPath));
  const failures: Array<{ id: string; detail: string }> = [];

  if (!manifest) {
    failures.push({
      id: 'gateway_manifest_missing',
      detail: args.manifestPath,
    });
  }

  const targetIds: string[] = Array.isArray(manifest?.production_gateway_targets)
    ? manifest.production_gateway_targets
        .map((value: unknown) => cleanText(String(value || ''), 80))
        .filter(Boolean)
    : [];
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
  for (const row of Array.isArray(supportLevelsPayload?.gateway_support_levels)
    ? supportLevelsPayload.gateway_support_levels
    : []) {
    const id = cleanText(row?.id || '', 80);
    if (!id || supportRowsById.has(id)) continue;
    supportRowsById.set(id, row);
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
    if (!ALLOWED_SUPPORT_LEVELS.has(supportLevel)) {
      failures.push({
        id: 'gateway_support_level_invalid',
        detail: `${id}:${supportLevel || 'missing'}`,
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

    const uniformChecklistContractOk = REQUIRED_CHECKLIST_KEYS.every((key) =>
      ALLOWED_CHECKLIST_STATUS.has(normalizeStatus((checklistRaw as any)[key]) as ChecklistStatus),
    );
    if (!uniformChecklistContractOk) {
      failures.push({
        id: 'gateway_uniform_checklist_contract_incomplete',
        detail: id,
      });
    }

    const statusByKey = new Map<string, string>(
      checklistStatuses.map((row) => [row.key, row.status]),
    );
    const requiredForReadiness =
      supportLevel === 'graduated'
        ? REQUIRED_CHECKLIST_KEYS_WITH_FAIL_CLOSED
        : REQUIRED_CHECKLIST_KEYS;
    const supportLevelReadinessOk = requiredForReadiness.every((key) => {
      const status = statusByKey.get(key) || '';
      if (supportLevel === 'graduated') return status === 'complete';
      if (supportLevel === 'candidate') return status === 'complete' || status === 'in_progress';
      return status === 'complete' || status === 'in_progress' || status === 'pending';
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
