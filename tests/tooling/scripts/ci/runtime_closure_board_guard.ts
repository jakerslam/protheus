#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type Bucket = {
  id: string;
  label: string;
  owner: string;
  status: string;
  validation_gates: string[];
  evidence_artifacts: string[];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_closure_board_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    boardPath: cleanText(
      readFlag(argv, 'board') || 'client/runtime/config/runtime_closure_board.json',
      400,
    ),
    gateRegistryPath: cleanText(
      readFlag(argv, 'gate-registry') || 'tests/tooling/config/tooling_gate_registry.json',
      400,
    ),
    markdownPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/RUNTIME_CLOSURE_BOARD_GUARD_CURRENT.md',
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

function toStringList(raw: unknown, maxLen = 200): string[] {
  if (!Array.isArray(raw)) return [];
  const out: string[] = [];
  for (const value of raw) {
    const cleaned = cleanText(value || '', maxLen);
    if (!cleaned) continue;
    out.push(cleaned);
  }
  return out;
}

function renderMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Closure Board Guard');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`- pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- bucket_count: ${Number(payload?.summary?.bucket_count || 0)}`);
  lines.push(`- required_bucket_count: ${Number(payload?.summary?.required_bucket_count || 0)}`);
  lines.push(`- missing_required_bucket_count: ${Number(payload?.summary?.missing_required_bucket_count || 0)}`);
  lines.push(`- invalid_gate_ref_count: ${Number(payload?.summary?.invalid_gate_ref_count || 0)}`);
  lines.push(`- failure_count: ${Number(payload?.summary?.failure_count || 0)}`);
  lines.push('');
  lines.push('## Buckets');
  for (const bucket of Array.isArray(payload?.buckets) ? payload.buckets : []) {
    lines.push(
      `- ${cleanText(bucket?.id || 'unknown', 80)}: status=${cleanText(
        bucket?.status || '',
        40,
      )} gates=${Number(bucket?.validation_gate_count || 0)} artifacts=${Number(
        bucket?.evidence_artifact_count || 0,
      )} missing_gate_refs=${Number(bucket?.missing_gate_refs_count || 0)}`,
    );
  }
  const failures = Array.isArray(payload?.failures) ? payload.failures : [];
  if (failures.length > 0) {
    lines.push('');
    lines.push('## Failures');
    for (const failure of failures) {
      lines.push(
        `- ${cleanText(failure?.id || 'unknown', 120)}: ${cleanText(failure?.detail || '', 240)}`,
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
  const requiredBucketIds = new Set([
    'layer2_parity',
    'production_gateways',
    'boundedness',
    'dashboard_truth',
    'auto_heal_backpressure',
  ]);
  const allowedStatuses = new Set(['active', 'blocked', 'degraded', 'done']);
  const boardPayload = readJsonBestEffort(path.resolve(root, args.boardPath));
  const gateRegistry = readJsonBestEffort(path.resolve(root, args.gateRegistryPath));
  const knownGateIds = new Set<string>(Object.keys(gateRegistry?.gates || {}));
  const failures: Array<{ id: string; detail: string }> = [];

  if (!boardPayload) {
    failures.push({
      id: 'runtime_closure_board_missing',
      detail: args.boardPath,
    });
  }

  const bucketRowsRaw = Array.isArray(boardPayload?.buckets) ? boardPayload.buckets : [];
  const buckets: Bucket[] = bucketRowsRaw.map((row: any) => ({
    id: cleanText(row?.id || '', 80),
    label: cleanText(row?.label || '', 120),
    owner: cleanText(row?.owner || '', 80),
    status: cleanText(row?.status || '', 40),
    validation_gates: toStringList(row?.validation_gates, 120),
    evidence_artifacts: toStringList(row?.evidence_artifacts, 260),
  }));

  const byId = new Map<string, Bucket>();
  for (const bucket of buckets) {
    if (!bucket.id) {
      failures.push({
        id: 'runtime_closure_bucket_id_missing',
        detail: bucket.label || 'unknown',
      });
      continue;
    }
    if (byId.has(bucket.id)) {
      failures.push({
        id: 'runtime_closure_bucket_duplicate',
        detail: bucket.id,
      });
      continue;
    }
    byId.set(bucket.id, bucket);
  }

  for (const required of requiredBucketIds) {
    if (!byId.has(required)) {
      failures.push({
        id: 'runtime_closure_required_bucket_missing',
        detail: required,
      });
    }
  }

  const evaluatedBuckets = buckets.map((bucket) => {
    if (!bucket.label) {
      failures.push({
        id: 'runtime_closure_bucket_label_missing',
        detail: bucket.id,
      });
    }
    if (!bucket.owner) {
      failures.push({
        id: 'runtime_closure_bucket_owner_missing',
        detail: bucket.id,
      });
    }
    if (!allowedStatuses.has(bucket.status)) {
      failures.push({
        id: 'runtime_closure_bucket_status_invalid',
        detail: `${bucket.id}:${bucket.status || 'missing'}`,
      });
    }
    if (bucket.validation_gates.length === 0) {
      failures.push({
        id: 'runtime_closure_bucket_validation_gates_missing',
        detail: bucket.id,
      });
    }
    if (bucket.evidence_artifacts.length === 0) {
      failures.push({
        id: 'runtime_closure_bucket_evidence_artifacts_missing',
        detail: bucket.id,
      });
    }
    const missingGateRefs = bucket.validation_gates.filter((gateId) => !knownGateIds.has(gateId));
    if (missingGateRefs.length > 0) {
      failures.push({
        id: 'runtime_closure_bucket_validation_gate_ref_unknown',
        detail: `${bucket.id}:${missingGateRefs.join(',')}`,
      });
    }
    return {
      ...bucket,
      validation_gate_count: bucket.validation_gates.length,
      evidence_artifact_count: bucket.evidence_artifacts.length,
      missing_gate_refs: missingGateRefs,
      missing_gate_refs_count: missingGateRefs.length,
    };
  });

  const payload = {
    ok: failures.length === 0,
    type: 'runtime_closure_board_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    board_path: args.boardPath,
    gate_registry_path: args.gateRegistryPath,
    summary: {
      pass: failures.length === 0,
      bucket_count: evaluatedBuckets.length,
      required_bucket_count: requiredBucketIds.size,
      missing_required_bucket_count: failures.filter(
        (row) => row.id === 'runtime_closure_required_bucket_missing',
      ).length,
      invalid_gate_ref_count: failures.filter(
        (row) => row.id === 'runtime_closure_bucket_validation_gate_ref_unknown',
      ).length,
      failure_count: failures.length,
    },
    buckets: evaluatedBuckets,
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
