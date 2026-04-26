#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type CandidateRow = {
  source_path: string;
  source_field: string;
  candidate: any;
};

const DEFAULT_OUT = 'core/local/artifacts/issue_candidate_contract_guard_current.json';
const DEFAULT_MARKDOWN = 'local/workspace/reports/ISSUE_CANDIDATE_CONTRACT_GUARD_CURRENT.md';
const DEFAULT_SOURCE_PATHS = [
  'core/local/artifacts/kernel_sentinel_auto_run_current.json',
  'local/state/kernel_sentinel/top_system_holes_current.json',
  'core/local/artifacts/eval_quality_gate_v1_current.json',
  'artifacts/eval_quality_gate_v1_latest.json',
  'core/local/artifacts/runtime_proof_release_gate_rich_current.json',
  'core/local/artifacts/runtime_proof_release_gate_pure_current.json',
  'core/local/artifacts/runtime_proof_release_gate_tiny-max_current.json',
  'core/local/artifacts/release_proof_pack_current.json',
  'core/local/artifacts/issue_candidate_backlog_current.json',
];
const OPTIONAL_BOOTSTRAP_SOURCE_PATHS = new Set([
  'core/local/artifacts/release_proof_pack_current.json',
  'core/local/artifacts/issue_candidate_backlog_current.json',
]);
const SELF_SOURCE_ARTIFACTS = [
  'tests/tooling/scripts/ci/issue_candidate_contract_guard.ts',
  'tests/tooling/config/tooling_gate_registry.json',
];

const REQUIRED_STRING_FIELDS = [
  'type',
  'generated_at',
  'status',
  'source',
  'fingerprint',
  'dedupe_key',
  'owner',
  'route_to',
  'title',
  'severity',
] as const;

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT });
  const rawSources = cleanText(readFlag(argv, 'sources') || '', 4000);
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT, 400),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN, 400),
    sources: rawSources
      ? rawSources.split(',').map((row) => cleanText(row, 400)).filter(Boolean)
      : DEFAULT_SOURCE_PATHS,
  };
}

function readJsonMaybe(root: string, relPath: string): any {
  try {
    return JSON.parse(fs.readFileSync(path.resolve(root, relPath), 'utf8'));
  } catch {
    return null;
  }
}

function candidateRowsFromPayload(sourcePath: string, payload: any): CandidateRow[] {
  const rows: CandidateRow[] = [];
  if (!payload || typeof payload !== 'object') return rows;
  if (payload.issue_candidate && typeof payload.issue_candidate === 'object') {
    rows.push({
      source_path: sourcePath,
      source_field: 'issue_candidate',
      candidate: payload.issue_candidate,
    });
  }
  const topLevelCandidates = Array.isArray(payload.issue_candidates) ? payload.issue_candidates : [];
  for (const [index, candidate] of topLevelCandidates.entries()) {
    if (candidate && typeof candidate === 'object') {
      rows.push({
        source_path: sourcePath,
        source_field: `issue_candidates[${index}]`,
        candidate,
      });
    }
  }
  const backlogCandidates = Array.isArray(payload.candidates) ? payload.candidates : [];
  for (const [index, candidate] of backlogCandidates.entries()) {
    if (candidate && typeof candidate === 'object') {
      rows.push({
        source_path: sourcePath,
        source_field: `candidates[${index}]`,
        candidate,
      });
    }
  }
  const holesCandidates = Array.isArray(payload?.top_system_holes?.issue_candidates)
    ? payload.top_system_holes.issue_candidates
    : [];
  for (const [index, candidate] of holesCandidates.entries()) {
    if (candidate && typeof candidate === 'object') {
      rows.push({
        source_path: sourcePath,
        source_field: `top_system_holes.issue_candidates[${index}]`,
        candidate,
      });
    }
  }
  return rows;
}

function arrayNonEmpty(value: unknown): boolean {
  return Array.isArray(value) && value.length > 0;
}

function sourceArtifactExists(root: string, rawPath: unknown): boolean {
  const artifactPath = cleanText(rawPath || '', 500);
  if (!artifactPath) return false;
  return fs.existsSync(path.isAbsolute(artifactPath) ? artifactPath : path.resolve(root, artifactPath));
}

function existingSourceArtifacts(root: string, rows: string[]): string[] {
  const out: string[] = [];
  for (const row of [...rows, ...SELF_SOURCE_ARTIFACTS]) {
    const artifactPath = cleanText(row || '', 500);
    if (!artifactPath || out.includes(artifactPath)) continue;
    if (sourceArtifactExists(root, artifactPath)) out.push(artifactPath);
  }
  return out;
}

function validateCandidate(row: CandidateRow, root: string): string[] {
  const candidate = row.candidate || {};
  const failures: string[] = [];
  for (const field of REQUIRED_STRING_FIELDS) {
    if (!cleanText(candidate[field] || '', 400)) {
      failures.push(`missing_required_string:${field}`);
    }
  }
  if (candidate.schema_version !== 1) {
    failures.push('schema_version_not_1');
  }
  if (cleanText(candidate.status || '', 80) !== 'candidate') {
    failures.push('status_not_candidate');
  }
  if (!arrayNonEmpty(candidate.labels)) {
    failures.push('labels_missing_or_empty');
  }
  if (!arrayNonEmpty(candidate.acceptance_criteria)) {
    failures.push('acceptance_criteria_missing_or_empty');
  }
  if (!arrayNonEmpty(candidate.source_artifacts)) {
    failures.push('source_artifacts_missing_or_empty');
  } else {
    for (const sourceArtifact of candidate.source_artifacts) {
      if (!sourceArtifactExists(root, sourceArtifact)) {
        failures.push(`source_artifact_missing:${cleanText(sourceArtifact || '', 220)}`);
      }
    }
  }
  const automationMode = cleanText(candidate?.automation_policy?.mode || '', 80);
  if (automationMode !== 'proposal_only') {
    failures.push('automation_policy_not_proposal_only');
  }
  if (candidate?.triage?.safe_to_auto_apply_patch !== false) {
    failures.push('triage_must_forbid_auto_apply_patch');
  }
  if (candidate?.triage?.safe_to_auto_file_issue !== true) {
    failures.push('triage_must_allow_auto_file_issue');
  }
  return failures;
}

function renderMarkdown(payload: any): string {
  const lines = [
    '# Issue Candidate Contract Guard',
    '',
    `- generated_at: ${cleanText(payload.generated_at || '', 80)}`,
    `- revision: ${cleanText(payload.revision || '', 120)}`,
    `- pass: ${payload.ok === true ? 'true' : 'false'}`,
    `- candidate_count: ${Number(payload.summary?.candidate_count || 0)}`,
    `- violation_count: ${Number(payload.summary?.violation_count || 0)}`,
    `- missing_source_count: ${Number(payload.summary?.missing_source_count || 0)}`,
    `- primary_blocker: ${cleanText(payload.operator_summary?.primary_blocker || 'none', 160)}`,
    `- issue_candidate_ready: ${payload.operator_summary?.issue_candidate_ready === true ? 'true' : 'false'}`,
    '',
    '## Violations',
  ];
  const violations = Array.isArray(payload.violations) ? payload.violations : [];
  if (violations.length === 0) {
    lines.push('- none');
  } else {
    for (const row of violations) {
      lines.push(
        `- ${cleanText(row.source_path || '', 220)} ${cleanText(row.source_field || '', 120)}: ${cleanText(row.failure || '', 200)}`,
      );
    }
  }
  lines.push('');
  lines.push('## Missing Sources');
  const missingSources = Array.isArray(payload.missing_sources) ? payload.missing_sources : [];
  if (missingSources.length === 0) {
    lines.push('- none');
  } else {
    for (const source of missingSources) lines.push(`- ${cleanText(source, 240)}`);
  }
  lines.push('');
  if (payload.issue_candidate) {
    lines.push('## Issue candidate');
    lines.push(`- title: ${cleanText(payload.issue_candidate.title || '', 160)}`);
    lines.push(`- severity: ${cleanText(payload.issue_candidate.severity || '', 80)}`);
    lines.push(`- fingerprint: ${cleanText(payload.issue_candidate.fingerprint || '', 240)}`);
    lines.push(`- next_actions: ${Number(payload.issue_candidate.next_actions?.length || 0)}`);
    lines.push('');
  }
  lines.push('## Sources');
  for (const row of payload.sources || []) {
    lines.push(
      `- ${cleanText(row.path || '', 220)}: exists=${row.exists === true ? 'true' : 'false'}; optional_bootstrap=${row.optional_bootstrap === true ? 'true' : 'false'}; candidates=${Number(row.candidate_count || 0)}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const sourceSummaries = [];
  const candidateRows: CandidateRow[] = [];

  for (const sourcePath of args.sources) {
    const payload = readJsonMaybe(root, sourcePath);
    const rows = candidateRowsFromPayload(sourcePath, payload);
    candidateRows.push(...rows);
    sourceSummaries.push({
      path: sourcePath,
      exists: payload !== null,
      optional_bootstrap: OPTIONAL_BOOTSTRAP_SOURCE_PATHS.has(sourcePath),
      candidate_count: rows.length,
    });
  }

  const missingSources = sourceSummaries
    .filter((row) => row.exists !== true && !OPTIONAL_BOOTSTRAP_SOURCE_PATHS.has(row.path))
    .map((row) => row.path);
  const missingSourceViolations = missingSources
    .map((sourcePath) => ({
      source_path: sourcePath,
      source_field: 'source_artifact',
      dedupe_key: '',
      failure: 'missing_source_artifact',
    }));
  const violations = [
    ...missingSourceViolations,
    ...candidateRows.flatMap((row) =>
    validateCandidate(row, root).map((failure) => ({
      source_path: row.source_path,
      source_field: row.source_field,
      dedupe_key: cleanText(row.candidate?.dedupe_key || '', 240),
      failure,
    })),
  )];
  const dedupeKeys = candidateRows.map((row) => cleanText(row.candidate?.dedupe_key || '', 300)).filter(Boolean);
  const duplicateDedupeKeys = dedupeKeys.filter((key, index, arr) => arr.indexOf(key) !== index);
  for (const key of Array.from(new Set(duplicateDedupeKeys))) {
    violations.push({
      source_path: 'candidate_set',
      source_field: 'dedupe_key',
      dedupe_key: key,
      failure: 'duplicate_dedupe_key',
    });
  }
  const issueCandidate = violations.length === 0
    ? null
    : {
        type: 'issue_candidate_contract_guard_issue_candidate',
        schema_version: 1,
        generated_at: new Date().toISOString(),
        status: 'candidate',
        source: 'issue_candidate_contract_guard',
        fingerprint: `issue_candidate_contract_guard:${violations.map((row) => `${row.source_path}:${row.failure}`).join('|')}`,
        dedupe_key: `issue_candidate_contract_guard:${violations.map((row) => `${row.source_path}:${row.failure}`).join('|')}`,
        owner: 'ops/issue_candidate_contract',
        route_to: 'release_blocker_backlog',
        title: 'Issue candidate contract guard failed',
        severity: 'high',
        labels: ['issue-candidate', 'governance', 'release-gate'],
        source_artifacts: existingSourceArtifacts(root, args.sources),
        missing_source_artifacts: args.sources.filter((sourcePath) => !sourceArtifactExists(root, sourcePath)),
        triage: {
          state: 'ready_for_issue_synthesis',
          safe_to_auto_file_issue: true,
          safe_to_auto_apply_patch: false,
          requires_release_authority_receipt_to_close: true,
        },
        automation_policy: {
          mode: 'proposal_only',
          requires_release_authority_receipt_before_apply: true,
          autonomous_release_unblock_allowed: false,
        },
        next_actions: violations.map((row) => ({
          artifact: row.source_path,
          field: row.source_field,
          action: `repair issue candidate contract violation ${row.failure}`,
          dedupe_key: row.dedupe_key,
        })),
        acceptance_criteria: [
          'all issue candidates satisfy schema version 1',
          'all issue candidates have lifecycle, routing, source artifact, and acceptance-criteria fields',
          'all issue candidates are proposal-only and cannot auto-apply patches',
          'issue candidate dedupe keys are unique across the guarded source set',
        ],
      };

  const payload = {
    ok: violations.length === 0,
    type: 'issue_candidate_contract_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    sources: sourceSummaries,
    missing_sources: missingSources,
    summary: {
      pass: violations.length === 0,
      source_count: sourceSummaries.length,
      missing_source_count: missingSources.length,
      candidate_count: candidateRows.length,
      violation_count: violations.length,
      duplicate_dedupe_key_count: new Set(duplicateDedupeKeys).size,
      optional_bootstrap_source_count: OPTIONAL_BOOTSTRAP_SOURCE_PATHS.size,
      issue_candidate_ready: issueCandidate !== null,
      evidence_health: {
        configured_source_count: sourceSummaries.length,
        missing_source_count: missingSources.length,
        all_required_sources_present: missingSources.length === 0,
        all_candidates_have_valid_evidence: !violations.some((row) =>
          row.failure === 'missing_source_artifact' ||
          String(row.failure || '').startsWith('source_artifact_missing'),
        ),
      },
    },
    operator_summary: {
      pass: violations.length === 0,
      source_count: sourceSummaries.length,
      missing_source_count: missingSources.length,
      candidate_count: candidateRows.length,
      violation_count: violations.length,
      primary_blocker: violations[0]?.failure || '',
      issue_candidate_ready: issueCandidate !== null,
      next_actions: issueCandidate?.next_actions || [],
    },
    contract: {
      candidate_schema_version: 1,
      required_string_fields: REQUIRED_STRING_FIELDS,
      automation_mode_required: 'proposal_only',
      safe_to_auto_file_issue_required: true,
      safe_to_auto_apply_patch_required: false,
      source_artifacts_must_exist: true,
      self_issue_source_artifacts_only_existing: true,
      optional_bootstrap_source_paths: Array.from(OPTIONAL_BOOTSTRAP_SOURCE_PATHS),
      optional_sources_are_validated_when_present: true,
    },
    issue_candidate: issueCandidate,
    violations,
    candidates: candidateRows.map((row) => ({
      source_path: row.source_path,
      source_field: row.source_field,
      type: cleanText(row.candidate?.type || '', 120),
      dedupe_key: cleanText(row.candidate?.dedupe_key || '', 300),
      owner: cleanText(row.candidate?.owner || '', 160),
      severity: cleanText(row.candidate?.severity || '', 80),
    })),
  };
  writeTextArtifact(args.markdownPath, renderMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
    artifactPaths: [args.markdownPath],
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
