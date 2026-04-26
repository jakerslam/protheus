#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type CandidateSourceRow = {
  source_path: string;
  source_field: string;
  candidate: any;
};

const DEFAULT_OUT = 'core/local/artifacts/issue_candidate_backlog_current.json';
const DEFAULT_MARKDOWN = 'local/workspace/reports/ISSUE_CANDIDATE_BACKLOG_CURRENT.md';
const DEFAULT_SOURCE_PATHS = [
  'core/local/artifacts/kernel_sentinel_auto_run_current.json',
  'local/state/kernel_sentinel/top_system_holes_current.json',
  'core/local/artifacts/eval_quality_gate_v1_current.json',
  'artifacts/eval_quality_gate_v1_latest.json',
  'core/local/artifacts/runtime_proof_release_gate_rich_current.json',
  'core/local/artifacts/runtime_proof_release_gate_pure_current.json',
  'core/local/artifacts/runtime_proof_release_gate_tiny-max_current.json',
  'core/local/artifacts/release_proof_pack_current.json',
  'core/local/artifacts/issue_candidate_contract_guard_current.json',
];
const OPTIONAL_BOOTSTRAP_SOURCE_PATHS = new Set([
  'core/local/artifacts/release_proof_pack_current.json',
  'core/local/artifacts/issue_candidate_contract_guard_current.json',
]);
const SELF_SOURCE_ARTIFACTS = [
  'tests/tooling/scripts/ci/issue_candidate_backlog_assemble.ts',
  'tests/tooling/config/tooling_gate_registry.json',
];

const SEVERITY_RANK: Record<string, number> = {
  release_blocking: 0,
  critical: 0,
  high: 1,
  medium: 2,
  low: 3,
};

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

function candidatesFromPayload(sourcePath: string, payload: any): CandidateSourceRow[] {
  const rows: CandidateSourceRow[] = [];
  if (!payload || typeof payload !== 'object') return rows;
  if (payload.issue_candidate && typeof payload.issue_candidate === 'object') {
    rows.push({ source_path: sourcePath, source_field: 'issue_candidate', candidate: payload.issue_candidate });
  }
  const topCandidates = Array.isArray(payload.issue_candidates) ? payload.issue_candidates : [];
  for (const [index, candidate] of topCandidates.entries()) {
    if (candidate && typeof candidate === 'object') {
      rows.push({ source_path: sourcePath, source_field: `issue_candidates[${index}]`, candidate });
    }
  }
  const holeCandidates = Array.isArray(payload?.top_system_holes?.issue_candidates)
    ? payload.top_system_holes.issue_candidates
    : [];
  for (const [index, candidate] of holeCandidates.entries()) {
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

function severityRank(value: unknown): number {
  const severity = cleanText(value || '', 80).toLowerCase();
  return SEVERITY_RANK[severity] ?? 4;
}

function artifactExists(root: string, rawPath: unknown): boolean {
  const artifactPath = cleanText(rawPath || '', 500);
  if (!artifactPath) return false;
  return fs.existsSync(path.isAbsolute(artifactPath) ? artifactPath : path.resolve(root, artifactPath));
}

function existingSourceArtifacts(root: string, rows: string[]): string[] {
  const out: string[] = [];
  for (const row of [...rows, ...SELF_SOURCE_ARTIFACTS]) {
    const artifactPath = cleanText(row || '', 500);
    if (!artifactPath || out.includes(artifactPath)) continue;
    if (artifactExists(root, artifactPath)) out.push(artifactPath);
  }
  return out;
}

function normalizeCandidate(row: CandidateSourceRow) {
  const candidate = row.candidate || {};
  const dedupeKey =
    cleanText(candidate.dedupe_key || '', 300) ||
    cleanText(candidate.fingerprint || '', 300) ||
    `${row.source_path}:${row.source_field}`;
  const nextActions = Array.isArray(candidate.next_actions)
    ? candidate.next_actions
    : Array.isArray(candidate.required_actions)
      ? candidate.required_actions.map((action: unknown) => ({ action: cleanText(action || '', 500) }))
      : [];
  return {
    schema_version: 1,
    generated_at: cleanText(candidate.generated_at || new Date().toISOString(), 80),
    dedupe_key: dedupeKey,
    fingerprint: cleanText(candidate.fingerprint || dedupeKey, 300),
    type: cleanText(candidate.type || 'issue_candidate', 120),
    status: cleanText(candidate.status || 'candidate', 80),
    source: cleanText(candidate.source || row.source_path, 200),
    source_path: row.source_path,
    source_field: row.source_field,
    owner: cleanText(candidate.owner || 'unknown', 160),
    route_to: cleanText(candidate.route_to || 'issue_backlog', 160),
    title: cleanText(candidate.title || 'Untitled issue candidate', 240),
    severity: cleanText(candidate.severity || 'unknown', 80),
    severity_rank: severityRank(candidate.severity),
    labels: Array.isArray(candidate.labels) ? candidate.labels.map((label: unknown) => cleanText(label, 80)).filter(Boolean) : [],
    impact: cleanText(candidate.impact || '', 500),
    next_actions: nextActions,
    acceptance_criteria: Array.isArray(candidate.acceptance_criteria)
      ? candidate.acceptance_criteria.map((row: unknown) => cleanText(row, 300)).filter(Boolean)
      : [],
    source_artifacts: Array.isArray(candidate.source_artifacts)
      ? candidate.source_artifacts.map((row: unknown) => cleanText(row, 400)).filter(Boolean)
      : [row.source_path],
    automation_policy: candidate.automation_policy || {},
    triage: candidate.triage || {},
  };
}

function renderMarkdown(payload: any): string {
  const lines = [
    '# Issue Candidate Backlog',
    '',
    `- generated_at: ${cleanText(payload.generated_at || '', 80)}`,
    `- revision: ${cleanText(payload.revision || '', 120)}`,
    `- pass: ${payload.ok === true ? 'true' : 'false'}`,
    `- candidate_count: ${Number(payload.summary?.candidate_count || 0)}`,
    `- release_blocking_count: ${Number(payload.summary?.release_blocking_count || 0)}`,
    `- missing_source_count: ${Number(payload.summary?.missing_source_count || 0)}`,
    `- missing_candidate_source_artifact_count: ${Number(payload.summary?.missing_candidate_source_artifact_count || 0)}`,
    '',
    '## Top Candidates',
  ];
  const rows = Array.isArray(payload.candidates) ? payload.candidates.slice(0, 20) : [];
  if (rows.length === 0) {
    lines.push('- none');
  } else {
    for (const row of rows) {
      lines.push(
        `- [${cleanText(row.severity || '', 40)}] ${cleanText(row.title || '', 180)} (${cleanText(row.route_to || '', 120)}; ${cleanText(row.dedupe_key || '', 220)})`,
      );
    }
  }
  lines.push('');
  lines.push('## Missing Sources');
  const missing = Array.isArray(payload.missing_sources) ? payload.missing_sources : [];
  if (missing.length === 0) {
    lines.push('- none');
  } else {
    for (const source of missing) lines.push(`- ${cleanText(source, 240)}`);
  }
  lines.push('');
  lines.push('## Missing Candidate Evidence Artifacts');
  const missingCandidateArtifacts = Array.isArray(payload.missing_candidate_source_artifacts)
    ? payload.missing_candidate_source_artifacts
    : [];
  if (missingCandidateArtifacts.length === 0) {
    lines.push('- none');
  } else {
    for (const row of missingCandidateArtifacts) {
      lines.push(`- ${cleanText(row.dedupe_key || '', 240)} -> ${cleanText(row.artifact || '', 240)}`);
    }
  }
  lines.push('');
  if (payload.issue_candidate) {
    lines.push('## Backlog Issue Candidate');
    lines.push(`- title: ${cleanText(payload.issue_candidate.title || '', 180)}`);
    lines.push(`- severity: ${cleanText(payload.issue_candidate.severity || '', 80)}`);
    lines.push(`- fingerprint: ${cleanText(payload.issue_candidate.fingerprint || '', 240)}`);
    lines.push(`- next_actions: ${Number(payload.issue_candidate.next_actions?.length || 0)}`);
  }
  lines.push('');
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
  const missingSources: string[] = [];
  const sourceRows: CandidateSourceRow[] = [];
  const sourceSummary = [];

  for (const sourcePath of args.sources) {
    const payload = readJsonMaybe(root, sourcePath);
    const optionalBootstrap = OPTIONAL_BOOTSTRAP_SOURCE_PATHS.has(sourcePath);
    if (!payload && !optionalBootstrap) missingSources.push(sourcePath);
    const rows = candidatesFromPayload(sourcePath, payload);
    sourceRows.push(...rows);
    sourceSummary.push({
      path: sourcePath,
      exists: payload !== null,
      optional_bootstrap: optionalBootstrap,
      candidate_count: rows.length,
    });
  }

  const byDedupeKey = new Map<string, ReturnType<typeof normalizeCandidate>>();
  for (const row of sourceRows.map(normalizeCandidate)) {
    const existing = byDedupeKey.get(row.dedupe_key);
    if (!existing || row.severity_rank < existing.severity_rank) {
      byDedupeKey.set(row.dedupe_key, row);
    }
  }
  const candidates = Array.from(byDedupeKey.values()).sort(
    (a, b) => a.severity_rank - b.severity_rank || a.route_to.localeCompare(b.route_to) || a.title.localeCompare(b.title),
  );
  const releaseBlocking = candidates.filter((row) => row.severity_rank === 0);
  const missingCandidateSourceArtifacts = candidates.flatMap((row) =>
    row.source_artifacts
      .filter((artifactPath: string) => !artifactExists(root, artifactPath))
      .map((artifactPath: string) => ({
        dedupe_key: row.dedupe_key,
        artifact: artifactPath,
      })),
  );
  const routeCounts = candidates.reduce<Record<string, number>>((acc, row) => {
    acc[row.route_to] = (acc[row.route_to] || 0) + 1;
    return acc;
  }, {});
  const pass =
    missingSources.length === 0 &&
    releaseBlocking.length === 0 &&
    missingCandidateSourceArtifacts.length === 0;
  const issueCandidate = pass
    ? null
    : {
        type: 'issue_candidate_backlog_issue_candidate',
        schema_version: 1,
        generated_at: new Date().toISOString(),
        status: 'candidate',
        source: 'issue_candidate_backlog_assemble',
        fingerprint: `issue_candidate_backlog:${[
          ...missingSources,
          ...releaseBlocking.map((row) => row.dedupe_key),
          ...missingCandidateSourceArtifacts.map((row) => `${row.dedupe_key}:${row.artifact}`),
        ].join('|')}`,
        dedupe_key: `issue_candidate_backlog:${[
          ...missingSources,
          ...releaseBlocking.map((row) => row.dedupe_key),
          ...missingCandidateSourceArtifacts.map((row) => `${row.dedupe_key}:${row.artifact}`),
        ].join('|')}`,
        owner: 'ops/issue_candidate_backlog',
        route_to: 'release_blocker_backlog',
        title: missingSources.length > 0
          ? 'Issue candidate backlog is missing source artifacts'
          : missingCandidateSourceArtifacts.length > 0
            ? 'Issue candidate backlog contains candidates with missing evidence artifacts'
          : 'Issue candidate backlog contains release-blocking candidates',
        severity: missingSources.length > 0 ? 'high' : 'release_blocking',
        labels: ['issue-candidate', 'backlog', 'release-gate'],
        impact: 'operators and future RSI loops cannot trust the remediation queue until the issue backlog is complete and release blockers are resolved',
        source_artifacts: existingSourceArtifacts(root, args.sources),
        missing_source_artifacts: args.sources.filter((sourcePath) => !artifactExists(root, sourcePath)),
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
        next_actions: [
          ...missingSources.map((source) => ({
            action: `produce issue candidate source artifact ${source}`,
            artifact: source,
          })),
          ...missingCandidateSourceArtifacts.map((row) => ({
            action: `restore missing issue-candidate source artifact ${row.artifact}`,
            artifact: row.artifact,
            dedupe_key: row.dedupe_key,
          })),
          ...releaseBlocking.map((row) => ({
            action: `resolve release-blocking issue candidate ${row.dedupe_key}`,
            route_to: row.route_to,
            artifact: row.source_path,
          })),
        ],
        acceptance_criteria: [
          'all configured issue candidate sources exist',
          'all issue candidate source_artifacts exist',
          'issue candidate backlog assembly passes',
          'release-blocking issue candidates are resolved or waived by release authority',
          'backlog remains deduped by dedupe_key',
        ],
      };

  const payload = {
    ok: pass,
    type: 'issue_candidate_backlog',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    sources: sourceSummary,
    missing_sources: missingSources,
    missing_candidate_source_artifacts: missingCandidateSourceArtifacts,
    summary: {
      pass,
      source_count: sourceSummary.length,
      missing_source_count: missingSources.length,
      optional_bootstrap_source_count: OPTIONAL_BOOTSTRAP_SOURCE_PATHS.size,
      raw_candidate_count: sourceRows.length,
      candidate_count: candidates.length,
      deduped_candidate_count: sourceRows.length - candidates.length,
      release_blocking_count: releaseBlocking.length,
      missing_candidate_source_artifact_count: missingCandidateSourceArtifacts.length,
      route_counts: routeCounts,
      issue_candidate_ready: issueCandidate !== null,
      evidence_health: {
        configured_source_count: sourceSummary.length,
        missing_source_count: missingSources.length,
        missing_candidate_source_artifact_count: missingCandidateSourceArtifacts.length,
        all_required_sources_present: missingSources.length === 0,
        all_candidate_source_artifacts_present: missingCandidateSourceArtifacts.length === 0,
      },
    },
    operator_summary: {
      pass,
      primary_blocker: missingSources[0] || missingCandidateSourceArtifacts[0]?.artifact || releaseBlocking[0]?.dedupe_key || '',
      next_actions: [
        ...missingSources.map((source) => ({ action: `produce issue candidate source artifact ${source}`, artifact: source })),
        ...missingCandidateSourceArtifacts.map((row) => ({ action: `restore missing issue-candidate source artifact ${row.artifact}`, artifact: row.artifact, dedupe_key: row.dedupe_key })),
        ...releaseBlocking.map((row) => ({ action: `resolve release-blocking issue candidate ${row.dedupe_key}`, route_to: row.route_to })),
      ],
      issue_candidate_ready: issueCandidate !== null,
    },
    backlog_contract: {
      candidates_are_deduped: true,
      sorted_by_severity_then_route: true,
      proposal_only_candidates_expected: true,
      source_artifacts_required: true,
      source_artifacts_must_exist: true,
      self_issue_source_artifacts_only_existing: true,
      missing_candidate_source_artifacts_fail_backlog: true,
      backlog_emits_issue_candidate_when_unhealthy: true,
      release_blocking_candidates_fail_backlog: true,
      optional_bootstrap_source_paths: Array.from(OPTIONAL_BOOTSTRAP_SOURCE_PATHS),
      optional_sources_are_ingested_when_present: true,
    },
    issue_candidate_contract: {
      candidate_schema_version: 1,
      normalized_candidates_preserve_generated_at: true,
      normalized_candidates_preserve_source_artifacts: true,
      normalized_candidates_preserve_triage_and_automation_policy: true,
      safe_to_auto_file_issue: true,
      safe_to_auto_apply_patch: false,
    },
    issue_candidate: issueCandidate,
    candidates,
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
