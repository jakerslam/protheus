#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type LaneRow = {
  lane_id?: string;
  id: string;
  owner?: string;
  status?: string;
  requested_action: string;
  expected_receipt_type: string;
  receipt_requirements?: string[];
  receipt_guarantees?: string[];
  timeout_model?: string;
  timeout_semantics?: string;
  retry_model?: string;
  retry_semantics?: string;
  runtime_path: string;
  fallback_path: string;
  fallback_conduit_only: boolean;
  replay_fixture_path?: string;
  replay_artifact_path?: string;
  parity_test_path?: string;
  deterministic_receipt_test_path?: string;
  proof_artifact_path?: string;
  day_one_replay_coverage?: boolean;
  day_one_deterministic_receipt_test?: boolean;
  experimental_exemption?: {
    ticket?: string;
    reason?: string;
    expires_at?: string;
  };
  provisional_exemption?: {
    ticket?: string;
    reason?: string;
    expires_at?: string;
  };
  invariants: string[];
  contract_test_id: string;
};

type Manifest = {
  version: number;
  lanes: LaneRow[];
};

const REQUIRED_INVARIANTS = [
  'receipt_emission',
  'cancellation_semantics',
  'timeout_semantics',
  'fail_closed',
] as const;

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/layer2_lane_parity_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'tests/tooling/config/layer2_parity_matrix.json',
      400,
    ),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/LAYER2_LANE_PARITY_GUARD_CURRENT.md',
      400,
    ),
  };
}

function loadManifest(root: string, relPath: string): Manifest {
  const abs = path.resolve(root, relPath);
  const raw = fs.readFileSync(abs, 'utf8');
  return JSON.parse(raw) as Manifest;
}

function hasConduitOnlyPath(input: string): boolean {
  const normalized = cleanText(input, 400).toLowerCase();
  return normalized.includes('conduit');
}

function parseIsoTimestamp(raw: string): number {
  const time = Date.parse(raw);
  return Number.isFinite(time) ? time : NaN;
}

function normalizeLaneRow(raw: LaneRow): LaneRow {
  const laneId = cleanText(raw.lane_id || raw.id || '', 120);
  const timeoutSemantics = cleanText(raw.timeout_model || raw.timeout_semantics || '', 200);
  const retrySemantics = cleanText(raw.retry_model || raw.retry_semantics || '', 200);
  const receiptRequirements =
    Array.isArray(raw.receipt_requirements) && raw.receipt_requirements.length > 0
      ? raw.receipt_requirements
      : Array.isArray(raw.receipt_guarantees)
        ? raw.receipt_guarantees
        : [];
  const replayArtifactPath = cleanText(raw.replay_artifact_path || raw.proof_artifact_path || '', 300);
  const receiptGuarantees =
    Array.isArray(raw.receipt_guarantees) && raw.receipt_guarantees.length > 0
      ? raw.receipt_guarantees
      : Array.isArray(raw.receipt_requirements)
        ? raw.receipt_requirements
        : [];
  return {
    ...raw,
    id: laneId,
    lane_id: laneId,
    timeout_semantics: timeoutSemantics,
    retry_semantics: retrySemantics,
    timeout_model: cleanText(raw.timeout_model || raw.timeout_semantics || '', 200),
    retry_model: cleanText(raw.retry_model || raw.retry_semantics || '', 200),
    receipt_requirements: receiptRequirements,
    receipt_guarantees: receiptGuarantees,
    proof_artifact_path: replayArtifactPath,
    replay_artifact_path: replayArtifactPath,
  };
}

function toMarkdown(rows: LaneRow[], violations: string[]): string {
  const lines = [
    '# Layer2 Lane Parity Guard',
    '',
    '| lane | owner | status | action | expected receipt | timeout | retry | parity test | replay fixture | deterministic receipt test | proof artifact | day-one replay | day-one deterministic receipt | runtime path | fallback path | contract test |',
    '| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |',
  ];
  for (const row of rows) {
    lines.push(
      `| ${row.id} | ${cleanText(row.owner || 'unknown', 40)} | ${cleanText(row.status || 'unknown', 20)} | ${row.requested_action} | ${row.expected_receipt_type} | ${cleanText(row.timeout_semantics || 'missing', 80)} | ${cleanText(row.retry_semantics || 'missing', 80)} | ${cleanText(row.parity_test_path || 'missing', 160)} | ${cleanText(row.replay_fixture_path || 'missing', 160)} | ${cleanText(row.deterministic_receipt_test_path || 'missing', 160)} | ${cleanText(row.proof_artifact_path || 'missing', 160)} | ${row.day_one_replay_coverage === true ? 'true' : 'false'} | ${row.day_one_deterministic_receipt_test === true ? 'true' : 'false'} | ${row.runtime_path} | ${row.fallback_path} | ${row.contract_test_id} |`,
    );
  }
  lines.push('');
  lines.push('## Violations');
  if (violations.length === 0) {
    lines.push('- none');
  } else {
    for (const row of violations) lines.push(`- ${row}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  const violations: string[] = [];
  let manifest: Manifest;
  try {
    manifest = loadManifest(root, args.manifestPath);
  } catch (err) {
    const payload = {
      ok: false,
      type: 'layer2_lane_parity_guard',
      error: 'layer2_lane_parity_manifest_read_failed',
      detail: cleanText(String(err), 400),
      manifest_path: args.manifestPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  if (!Array.isArray(manifest.lanes) || manifest.lanes.length === 0) {
    violations.push('layer2_lane_manifest_empty');
  }

  const ids = new Set<string>();
  const tests = new Set<string>();
  const nowEpoch = Date.now();
  let completeCount = 0;
  let experimentalCount = 0;
  let blockedCount = 0;
  let provisionalCount = 0;
  let provisionalWithExemptionCount = 0;
  const ALLOWED_STATUSES = new Set(['complete', 'experimental', 'provisional', 'blocked']);
  const lanes = (manifest.lanes || []).map((row) => normalizeLaneRow(row));
  for (const row of lanes) {
    if (!cleanText(row.lane_id || '', 120)) violations.push(`lane_missing_lane_id:${row.id || 'unknown'}`);
    if (row.lane_id && row.id && row.lane_id !== row.id) {
      violations.push(`lane_id_mismatch:${row.id}:${row.lane_id}`);
    }
    if (!row.id) violations.push('lane_missing_id');
    const status = cleanText(row.status || '', 20).toLowerCase();
    if (!status) violations.push(`lane_missing_status:${row.id || 'unknown'}`);
    if (status && !ALLOWED_STATUSES.has(status)) {
      violations.push(`lane_status_not_allowed:${row.id || 'unknown'}:${status}`);
    }
    if (status === 'experimental') experimentalCount += 1;
    if (status === 'blocked') blockedCount += 1;
    if (status === 'provisional') {
      provisionalCount += 1;
    }
    if (status === 'experimental' || status === 'provisional') {
      const exemption = row.experimental_exemption || row.provisional_exemption || {};
      const ticket = cleanText(exemption.ticket || '', 120);
      const reason = cleanText(exemption.reason || '', 200);
      const expiresAt = cleanText(exemption.expires_at || '', 80);
      if (!ticket || !reason || !expiresAt) {
        if (status === 'experimental') {
          violations.push(`lane_experimental_without_explicit_exemption:${row.id || 'unknown'}`);
        } else {
          violations.push(`lane_provisional_without_explicit_exemption:${row.id || 'unknown'}`);
        }
      } else {
        const expiresEpoch = parseIsoTimestamp(expiresAt);
        if (!Number.isFinite(expiresEpoch)) {
          if (status === 'experimental') {
            violations.push(`lane_experimental_exemption_expiry_invalid:${row.id || 'unknown'}:${expiresAt}`);
          } else {
            violations.push(`lane_provisional_exemption_expiry_invalid:${row.id || 'unknown'}:${expiresAt}`);
          }
        } else if (expiresEpoch <= nowEpoch) {
          if (status === 'experimental') {
            violations.push(`lane_experimental_exemption_expired:${row.id || 'unknown'}:${expiresAt}`);
          } else {
            violations.push(`lane_provisional_exemption_expired:${row.id || 'unknown'}:${expiresAt}`);
          }
        } else {
          provisionalWithExemptionCount += 1;
        }
      }
    }
    if (status === 'complete') completeCount += 1;
    if (!cleanText(row.owner || '', 80)) violations.push(`lane_missing_owner:${row.id || 'unknown'}`);
    if (!row.requested_action) violations.push(`lane_missing_action:${row.id || 'unknown'}`);
    if (!row.expected_receipt_type) violations.push(`lane_missing_receipt_type:${row.id || 'unknown'}`);
    const receiptRequirements = Array.isArray(row.receipt_requirements) ? row.receipt_requirements : [];
    if (receiptRequirements.length === 0) {
      violations.push(`lane_missing_receipt_requirements:${row.id || 'unknown'}`);
    } else if (!receiptRequirements.includes(row.expected_receipt_type)) {
      violations.push(`lane_receipt_requirements_missing_expected_receipt:${row.id || 'unknown'}`);
    }
    if (!cleanText(row.timeout_semantics || '', 120)) {
      violations.push(`lane_missing_timeout_semantics:${row.id || 'unknown'}`);
    }
    if (!cleanText(row.timeout_model || '', 120)) {
      violations.push(`lane_missing_timeout_model:${row.id || 'unknown'}`);
    }
    if (!cleanText(row.retry_semantics || '', 120)) {
      violations.push(`lane_missing_retry_semantics:${row.id || 'unknown'}`);
    }
    if (!cleanText(row.retry_model || '', 120)) {
      violations.push(`lane_missing_retry_model:${row.id || 'unknown'}`);
    }
    if (!row.runtime_path) violations.push(`lane_missing_runtime_path:${row.id || 'unknown'}`);
    if (!row.fallback_path) violations.push(`lane_missing_fallback_path:${row.id || 'unknown'}`);
    if (!cleanText(row.replay_fixture_path || '', 300)) {
      violations.push(`lane_missing_replay_fixture_path:${row.id || 'unknown'}`);
    } else if (!fs.existsSync(path.resolve(root, cleanText(row.replay_fixture_path || '', 300)))) {
      violations.push(`lane_replay_fixture_missing:${row.id || 'unknown'}:${cleanText(row.replay_fixture_path || '', 300)}`);
    }
    if (!cleanText(row.parity_test_path || '', 300)) {
      violations.push(`lane_missing_parity_test_path:${row.id || 'unknown'}`);
    } else if (!fs.existsSync(path.resolve(root, cleanText(row.parity_test_path || '', 300)))) {
      violations.push(`lane_parity_test_path_missing:${row.id || 'unknown'}:${cleanText(row.parity_test_path || '', 300)}`);
    }
    if (!cleanText(row.deterministic_receipt_test_path || '', 300)) {
      violations.push(`lane_missing_deterministic_receipt_test_path:${row.id || 'unknown'}`);
    } else if (!fs.existsSync(path.resolve(root, cleanText(row.deterministic_receipt_test_path || '', 300)))) {
      violations.push(
        `lane_deterministic_receipt_test_path_missing:${row.id || 'unknown'}:${cleanText(row.deterministic_receipt_test_path || '', 300)}`,
      );
    }
    if (!cleanText(row.replay_artifact_path || row.proof_artifact_path || '', 300)) {
      violations.push(`lane_missing_replay_artifact_path:${row.id || 'unknown'}`);
    }
    if (row.day_one_replay_coverage !== true) {
      violations.push(`lane_day_one_replay_coverage_not_true:${row.id || 'unknown'}`);
    }
    if (row.day_one_deterministic_receipt_test !== true) {
      violations.push(`lane_day_one_deterministic_receipt_test_not_true:${row.id || 'unknown'}`);
    }
    if (!row.contract_test_id) violations.push(`lane_missing_contract_test_id:${row.id || 'unknown'}`);
    if (row.id && ids.has(row.id)) violations.push(`lane_id_duplicate:${row.id}`);
    if (row.contract_test_id && tests.has(row.contract_test_id)) {
      violations.push(`lane_contract_test_duplicate:${row.contract_test_id}`);
    }
    ids.add(row.id);
    tests.add(row.contract_test_id);

    if (row.fallback_path && row.runtime_path && row.fallback_path === row.runtime_path) {
      violations.push(`lane_fallback_equals_runtime:${row.id}`);
    }
    if (row.fallback_conduit_only && !hasConduitOnlyPath(row.fallback_path || '')) {
      violations.push(`lane_fallback_not_conduit_only:${row.id}`);
    }

    const invariants = Array.isArray(row.invariants) ? row.invariants : [];
    const receiptGuarantees = Array.isArray(row.receipt_guarantees) ? row.receipt_guarantees : [];
    if (receiptGuarantees.length === 0) {
      violations.push(`lane_missing_receipt_guarantees:${row.id || 'unknown'}`);
    } else if (!receiptGuarantees.includes(row.expected_receipt_type)) {
      violations.push(`lane_receipt_guarantees_missing_expected_receipt:${row.id || 'unknown'}`);
    }
    for (const invariant of REQUIRED_INVARIANTS) {
      if (!invariants.includes(invariant)) {
        violations.push(`lane_missing_invariant:${row.id}:${invariant}`);
      }
    }
  }

  const report = {
    ok: violations.length === 0,
    type: 'layer2_lane_parity_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    manifest_path: args.manifestPath,
    markdown_path: args.markdownOutPath,
    summary: {
      lane_count: (manifest.lanes || []).length,
      complete_lane_count: completeCount,
      experimental_lane_count: experimentalCount,
      blocked_lane_count: blockedCount,
      provisional_lane_count: provisionalCount,
      provisional_lane_with_exemption_count: provisionalWithExemptionCount,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    lanes,
    contract_test_ids: Array.from(tests),
    failures: violations.map((detail) => ({ id: 'layer2_lane_parity_violation', detail })),
    artifact_paths: [args.markdownOutPath],
  };

  writeTextArtifact(args.markdownOutPath, toMarkdown(lanes, violations));

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
