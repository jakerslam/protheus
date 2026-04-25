#!/usr/bin/env tsx

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type PackManifest = {
  version: number;
  artifact_groups?: Record<string, string[]>;
  category_completeness_min?: Record<string, number>;
  freshness_max_age_hours?: Record<string, number>;
  freshness_required_artifacts?: string[];
  required_artifacts: string[];
  optional_artifacts: string[];
};

const MANDATORY_RELEASE_PROOF_ARTIFACTS = [
  'core/local/artifacts/layer2_lane_parity_guard_current.json',
  'core/local/artifacts/layer2_receipt_replay_current.json',
  'core/local/artifacts/runtime_trusted_core_report_current.json',
  'core/local/artifacts/eval_regression_guard_current.json',
  'core/local/artifacts/eval_feedback_router_current.json',
  'core/local/artifacts/runtime_proof_reality_guard_current.json',
  'core/local/artifacts/runtime_soak_scenarios_current.json',
];
const MANDATORY_PASSING_RELEASE_PROOF_ARTIFACTS = [
  'core/local/artifacts/eval_regression_guard_current.json',
  'core/local/artifacts/eval_feedback_router_current.json',
  'core/local/artifacts/runtime_proof_reality_guard_current.json',
  'core/local/artifacts/runtime_soak_scenarios_current.json',
];
const DEFAULT_FRESHNESS_REQUIRED_ARTIFACTS = [
  'core/local/artifacts/runtime_proof_verify_current.json',
  'core/local/artifacts/gateway_runtime_chaos_gate_current.json',
  'core/local/artifacts/windows_installer_contract_guard_current.json',
  'core/local/artifacts/windows_install_reliability_current.json',
  'core/local/artifacts/installer_reliability_closure_guard_current.json',
  'core/local/artifacts/eval_regression_guard_current.json',
  'core/local/artifacts/eval_feedback_router_current.json',
  'core/local/artifacts/test_maturity_registry_current.json',
];
const DEFAULT_FRESHNESS_MAX_AGE_HOURS = 336;
const CURRENT_EVIDENCE_ALLOWED_SUFFIXES = [
  '_current.json',
  '_latest.json',
  'release_scorecard.json',
  'benchmark_matrix_run_latest.json',
  'support_bundle_latest.json',
];

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/release_proof_pack_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'tests/tooling/config/release_proof_pack_manifest.json',
      400,
    ),
    version: cleanText(readFlag(argv, 'version') || new Date().toISOString().slice(0, 10), 120),
  };
}

function ensureParent(absPath: string) {
  const parent = path.dirname(absPath);
  fs.mkdirSync(parent, { recursive: true });
}

function sha256File(absPath: string): string {
  const data = fs.readFileSync(absPath);
  return createHash('sha256').update(data).digest('hex');
}

function categoryLookup(manifest: PackManifest): Map<string, string> {
  const out = new Map<string, string>();
  const groups = manifest.artifact_groups || {};
  for (const [group, rows] of Object.entries(groups)) {
    for (const relPath of rows || []) {
      out.set(cleanText(relPath, 400), cleanText(group, 120));
    }
  }
  return out;
}

function copyIntoPack(root: string, relPath: string, packRoot: string) {
  const source = path.resolve(root, relPath);
  const exists = fs.existsSync(source);
  const destination = path.resolve(packRoot, relPath);
  let checksum = '';
  let sizeBytes = 0;
  let modifiedAt = '';
  let ageHours: number | null = null;
  if (exists) {
    ensureParent(destination);
    fs.copyFileSync(source, destination);
    checksum = sha256File(destination);
    const stat = fs.statSync(source);
    sizeBytes = stat.size;
    modifiedAt = stat.mtime.toISOString();
    ageHours = Math.max(0, (Date.now() - stat.mtimeMs) / (60 * 60 * 1000));
  }
  return {
    path: relPath,
    exists,
    source,
    destination,
    checksum,
    size_bytes: sizeBytes,
    source_modified_at: modifiedAt,
    source_age_hours: ageHours,
  };
}

function jsonArtifactStatus(absPath: string): { declares_status: boolean; passing: boolean; detail: string } {
  try {
    const payload = JSON.parse(fs.readFileSync(absPath, 'utf8'));
    if (payload && typeof payload === 'object' && Object.prototype.hasOwnProperty.call(payload, 'ok')) {
      return { declares_status: true, passing: payload.ok === true, detail: 'ok' };
    }
    if (
      payload &&
      typeof payload === 'object' &&
      payload.summary &&
      typeof payload.summary === 'object' &&
      Object.prototype.hasOwnProperty.call(payload.summary, 'pass')
    ) {
      return { declares_status: true, passing: payload.summary.pass === true, detail: 'summary.pass' };
    }
    return { declares_status: false, passing: true, detail: 'no_status_field' };
  } catch {
    return { declares_status: true, passing: false, detail: 'json_parse_failed' };
  }
}

function jsonArtifactPasses(absPath: string): boolean {
  return jsonArtifactStatus(absPath).passing;
}

function isCurrentEvidencePath(relPath: string): boolean {
  const normalized = cleanText(relPath, 400);
  if (normalized.startsWith('releases/proof-packs/')) return false;
  if (normalized.includes('/releases/proof-packs/')) return false;
  return CURRENT_EVIDENCE_ALLOWED_SUFFIXES.some((suffix) => normalized.endsWith(suffix));
}

function markdown(report: any): string {
  const lines = [
    '# Release Proof Pack',
    '',
    `- version: ${report.version}`,
    `- pack_root: ${report.pack_root}`,
    `- required_missing: ${report.summary.required_missing}`,
    '',
    '| artifact | category | required | exists | sha256 |',
    '| --- | --- | :---: | :---: | --- |',
  ];
  if (report.summary) {
    lines.splice(
      5,
      0,
      `- stale_artifacts: ${Number(report.summary.stale_artifacts || 0)}`,
      `- failed_artifacts: ${Number(report.summary.failed_artifacts || 0)}`,
      `- historical_snapshot_artifacts: ${Number(report.summary.historical_snapshot_artifacts || 0)}`,
    );
  }
  for (const row of report.artifacts) {
    lines.push(
      `| ${row.path} | ${row.category} | ${row.required ? 'yes' : 'no'} | ${row.exists ? 'yes' : 'no'} | ${
        row.exists ? row.checksum : 'missing'
      } |`,
    );
  }
  lines.push('');
  lines.push('## Category summary');
  for (const group of report.category_summary) {
    const threshold =
      report?.category_completeness_min &&
      Object.prototype.hasOwnProperty.call(report.category_completeness_min, group.category)
        ? Number(report.category_completeness_min[group.category])
        : null;
    lines.push(
      `- ${group.category}: present=${group.present}/${group.total};required=${group.required_present}/${group.required_total};required_missing=${group.required_missing};required_completeness=${group.required_completeness.toFixed(
        3,
      )}${threshold == null ? '' : `;required_min=${threshold.toFixed(3)}`}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  const manifestRaw = fs.readFileSync(path.resolve(root, args.manifestPath), 'utf8');
  const manifest = JSON.parse(manifestRaw) as PackManifest;
  const categoryByPath = categoryLookup(manifest);

  const packRoot = path.resolve(root, 'releases', 'proof-packs', args.version);
  fs.mkdirSync(packRoot, { recursive: true });

  const artifactRows: Array<{
    path: string;
    category: string;
    required: boolean;
    exists: boolean;
    source: string;
    destination: string;
    checksum: string;
    size_bytes: number;
    source_modified_at: string;
    source_age_hours: number | null;
  }> = [];

  for (const rel of manifest.required_artifacts || []) {
    const normalized = cleanText(rel, 400);
    artifactRows.push({
      ...copyIntoPack(root, normalized, packRoot),
      category: cleanText(categoryByPath.get(normalized) || 'ungrouped', 120),
      required: true,
    });
  }
  for (const rel of manifest.optional_artifacts || []) {
    const normalized = cleanText(rel, 400);
    artifactRows.push({
      ...copyIntoPack(root, normalized, packRoot),
      category: cleanText(categoryByPath.get(normalized) || 'ungrouped', 120),
      required: false,
    });
  }

  const requiredMissing = artifactRows.filter((row) => row.required && !row.exists).map((row) => row.path);
  const mandatoryArtifactFailures = MANDATORY_RELEASE_PROOF_ARTIFACTS.map((artifactPath) => {
    const row = artifactRows.find((candidate) => candidate.path === artifactPath);
    if (!row) {
      return {
        path: artifactPath,
        failure: 'missing_from_manifest',
      };
    }
    if (!row.required) {
      return {
        path: artifactPath,
        failure: 'not_required_in_manifest',
      };
    }
    if (!row.exists) {
      return {
        path: artifactPath,
        failure: 'required_artifact_missing',
      };
    }
    if (
      MANDATORY_PASSING_RELEASE_PROOF_ARTIFACTS.includes(artifactPath) &&
      !jsonArtifactPasses(row.source)
    ) {
      return {
        path: artifactPath,
        failure: 'required_artifact_not_passing',
      };
    }
    return null;
  }).filter((row): row is { path: string; failure: string } => !!row);
  const requiredFailedArtifacts = artifactRows
    .filter((row) => row.required && row.exists && row.path.endsWith('.json'))
    .map((row) => ({
      path: row.path,
      status: jsonArtifactStatus(row.source),
    }))
    .filter((row) => row.status.declares_status && !row.status.passing)
    .map((row) => ({
      path: row.path,
      failure: `required_artifact_failed:${row.status.detail}`,
    }));
  const freshnessMaxAgeHours = manifest.freshness_max_age_hours || {};
  const freshnessRequired = new Set(
    [
      ...DEFAULT_FRESHNESS_REQUIRED_ARTIFACTS,
      ...(Array.isArray(manifest.freshness_required_artifacts)
        ? manifest.freshness_required_artifacts
        : []),
    ].map((row) => cleanText(row, 400)).filter(Boolean),
  );
  const staleArtifactFailures = artifactRows
    .filter((row) => row.required && row.exists && freshnessRequired.has(row.path))
    .map((row) => {
      const category = cleanText(row.category, 120);
      const maxAge =
        Number(freshnessMaxAgeHours[row.path]) ||
        Number(freshnessMaxAgeHours[category]) ||
        DEFAULT_FRESHNESS_MAX_AGE_HOURS;
      const age = Number(row.source_age_hours || 0);
      return {
        path: row.path,
        category,
        age_hours: age,
        max_age_hours: maxAge,
        ok: age <= maxAge,
      };
    })
    .filter((row) => !row.ok);
  const currentEvidenceFailures = artifactRows
    .filter((row) => row.required)
    .filter((row) => !isCurrentEvidencePath(row.path))
    .map((row) => ({
      path: row.path,
      failure: row.path.includes('releases/proof-packs/')
        ? 'historical_snapshot_path_used_as_current_evidence'
        : 'not_current_or_latest_evidence_path',
    }));
  const categoryCompletenessMin = manifest.category_completeness_min || {};
  const categories = Array.from(new Set(artifactRows.map((row) => row.category)));
  const categorySummary = categories.map((category) => {
    const rows = artifactRows.filter((row) => row.category === category);
    const present = rows.filter((row) => row.exists).length;
    const requiredRows = rows.filter((row) => row.required);
    const requiredPresent = requiredRows.filter((row) => row.exists).length;
    const requiredMissingCount = requiredRows.length - requiredPresent;
    const requiredCompleteness = requiredRows.length <= 0 ? 1 : requiredPresent / requiredRows.length;
    return {
      category,
      total: rows.length,
      present,
      required_total: requiredRows.length,
      required_present: requiredPresent,
      required_missing: requiredMissingCount,
      required_completeness: requiredCompleteness,
    };
  });
  const categoryThresholdFailures = Object.entries(categoryCompletenessMin)
    .map(([category, thresholdRaw]) => {
      const threshold = Number(thresholdRaw);
      if (!Number.isFinite(threshold)) return null;
      const summary = categorySummary.find((row) => row.category === category);
      const actual = summary ? Number(summary.required_completeness) : 0;
      const ok = !!summary && actual + Number.EPSILON >= threshold;
      return {
        id: 'proof_pack_category_completeness_below_threshold',
        category,
        threshold,
        actual,
        ok,
        detail: `${category}: actual=${actual.toFixed(3)};required_min=${threshold.toFixed(3)}`,
      };
    })
    .filter((row): row is { id: string; category: string; threshold: number; actual: number; ok: boolean; detail: string } => !!row)
    .filter((row) => !row.ok);
  const pass =
    requiredMissing.length === 0 &&
    mandatoryArtifactFailures.length === 0 &&
    categoryThresholdFailures.length === 0 &&
    requiredFailedArtifacts.length === 0 &&
    staleArtifactFailures.length === 0 &&
    currentEvidenceFailures.length === 0;

  const packManifest = {
    ok: pass,
    type: 'release_proof_pack_manifest',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    version: args.version,
    pack_root: packRoot,
    source_manifest_path: args.manifestPath,
    category_completeness_min: categoryCompletenessMin,
    artifacts: artifactRows,
    required_missing: requiredMissing,
    mandatory_artifact_failures: mandatoryArtifactFailures,
    category_threshold_failures: categoryThresholdFailures,
    required_failed_artifacts: requiredFailedArtifacts,
    stale_artifact_failures: staleArtifactFailures,
    current_evidence_failures: currentEvidenceFailures,
    category_summary: categorySummary,
  };

  const packManifestPath = path.resolve(packRoot, 'manifest.json');
  ensureParent(packManifestPath);
  fs.writeFileSync(packManifestPath, `${JSON.stringify(packManifest, null, 2)}\n`, 'utf8');

  const reportPath = path.resolve(packRoot, 'README.md');
  writeTextArtifact(
    reportPath,
    markdown({
      ...packManifest,
      summary: {
        required_missing: requiredMissing.length,
        stale_artifacts: staleArtifactFailures.length,
        failed_artifacts: requiredFailedArtifacts.length + mandatoryArtifactFailures.length,
        historical_snapshot_artifacts: currentEvidenceFailures.length,
      },
    }),
  );

  const report = {
    ok: pass,
    type: 'release_proof_pack_assemble',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    version: args.version,
    pack_root: packRoot,
    source_manifest_path: args.manifestPath,
    summary: {
      artifact_count: artifactRows.length,
      required_missing: requiredMissing.length,
      mandatory_artifact_failure_count: mandatoryArtifactFailures.length,
      required_failed_artifact_count: requiredFailedArtifacts.length,
      stale_artifact_count: staleArtifactFailures.length,
      historical_or_noncurrent_artifact_count: currentEvidenceFailures.length,
      category_threshold_failure_count: categoryThresholdFailures.length,
      pass,
    },
    category_completeness_min: categoryCompletenessMin,
    artifacts: artifactRows,
    category_summary: categorySummary,
    mandatory_artifact_failures: mandatoryArtifactFailures,
    required_failed_artifacts: requiredFailedArtifacts,
    stale_artifact_failures: staleArtifactFailures,
    current_evidence_failures: currentEvidenceFailures,
    category_threshold_failures: categoryThresholdFailures,
    failures: [
      ...requiredMissing.map((detail) => ({ id: 'proof_pack_required_artifact_missing', detail })),
      ...mandatoryArtifactFailures.map((row) => ({
        id: 'proof_pack_mandatory_artifact_violation',
        detail: `${row.path}:${row.failure}`,
      })),
      ...requiredFailedArtifacts.map((row) => ({
        id: 'proof_pack_required_artifact_failed',
        detail: `${row.path}:${row.failure}`,
      })),
      ...staleArtifactFailures.map((row) => ({
        id: 'proof_pack_required_artifact_stale',
        detail: `${row.path}:age_hours=${row.age_hours.toFixed(3)};max_age_hours=${row.max_age_hours}`,
      })),
      ...currentEvidenceFailures.map((row) => ({
        id: 'proof_pack_current_evidence_contract_failed',
        detail: `${row.path}:${row.failure}`,
      })),
      ...categoryThresholdFailures.map((row) => ({ id: row.id, detail: row.detail })),
    ],
    artifact_paths: [packManifestPath, reportPath],
  };

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
