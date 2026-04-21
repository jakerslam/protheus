#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type LaneRow = {
  id: string;
  owner?: string;
  status?: string;
  requested_action: string;
  expected_receipt_type: string;
  receipt_requirements?: string[];
  timeout_semantics?: string;
  retry_semantics?: string;
  runtime_path: string;
  fallback_path: string;
  fallback_conduit_only: boolean;
  replay_fixture_path?: string;
  parity_test_path?: string;
  deterministic_receipt_test_path?: string;
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
      readFlag(argv, 'manifest') || 'tests/tooling/config/layer2_lane_parity_manifest.json',
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

function toMarkdown(rows: LaneRow[], violations: string[]): string {
  const lines = [
    '# Layer2 Lane Parity Guard',
    '',
    '| lane | owner | status | action | expected receipt | timeout | retry | parity test | replay fixture | deterministic receipt test | runtime path | fallback path | contract test |',
    '| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |',
  ];
  for (const row of rows) {
    lines.push(
      `| ${row.id} | ${cleanText(row.owner || 'unknown', 40)} | ${cleanText(row.status || 'unknown', 20)} | ${row.requested_action} | ${row.expected_receipt_type} | ${cleanText(row.timeout_semantics || 'missing', 80)} | ${cleanText(row.retry_semantics || 'missing', 80)} | ${cleanText(row.parity_test_path || 'missing', 160)} | ${cleanText(row.replay_fixture_path || 'missing', 160)} | ${cleanText(row.deterministic_receipt_test_path || 'missing', 160)} | ${row.runtime_path} | ${row.fallback_path} | ${row.contract_test_id} |`,
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
  let completeCount = 0;
  const ALLOWED_STATUSES = new Set(['complete', 'experimental']);
  for (const row of manifest.lanes || []) {
    if (!row.id) violations.push('lane_missing_id');
    const status = cleanText(row.status || '', 20).toLowerCase();
    if (!status) violations.push(`lane_missing_status:${row.id || 'unknown'}`);
    if (status && !ALLOWED_STATUSES.has(status)) {
      violations.push(`lane_status_not_allowed:${row.id || 'unknown'}:${status}`);
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
    if (!cleanText(row.retry_semantics || '', 120)) {
      violations.push(`lane_missing_retry_semantics:${row.id || 'unknown'}`);
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
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    lanes: manifest.lanes || [],
    contract_test_ids: Array.from(tests),
    failures: violations.map((detail) => ({ id: 'layer2_lane_parity_violation', detail })),
    artifact_paths: [args.markdownOutPath],
  };

  writeTextArtifact(args.markdownOutPath, toMarkdown(manifest.lanes || [], violations));

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
