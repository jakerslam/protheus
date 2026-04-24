#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact } from '../../lib/result.ts';
import { run as runBoundednessInspect } from './runtime_boundedness_inspect.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';

const PROFILES: ProfileId[] = ['rich', 'pure', 'tiny-max'];

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_boundedness_inspect_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    outMarkdownPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_CURRENT.md',
      400,
    ),
    profilesOutPath: cleanText(
      readFlag(argv, 'profiles-out') || 'core/local/artifacts/runtime_boundedness_profiles_current.json',
      400,
    ),
    metricsTemplate: cleanText(
      readFlag(argv, 'metrics-template') ||
        'core/local/artifacts/runtime_proof_release_gate_{profile}_current.json',
      400,
    ),
    inspectTemplate: cleanText(
      readFlag(argv, 'inspect-template') ||
        'core/local/artifacts/runtime_boundedness_inspect_{profile}_current.json',
      400,
    ),
    markdownTemplate: cleanText(
      readFlag(argv, 'markdown-template') ||
        'local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_{profile_upper}_CURRENT.md',
      400,
    ),
    reportTemplate: cleanText(
      readFlag(argv, 'report-template') ||
        'core/local/artifacts/runtime_boundedness_report_{profile}_current.json',
      400,
    ),
  };
}

function resolveTemplate(template: string, profile: ProfileId): string {
  return cleanText(
    template
      .replaceAll('{profile}', profile)
      .replaceAll('{profile_upper}', profile.toUpperCase()),
    400,
  );
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function copyArtifact(root: string, sourceRel: string, targetRel: string) {
  const source = path.resolve(root, sourceRel);
  const target = path.resolve(root, targetRel);
  if (!fs.existsSync(source)) return;
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(source, target);
}

function isCanonicalToken(raw: string, maxLen = 120): boolean {
  const token = cleanText(String(raw || ''), maxLen);
  return /^[a-z0-9][a-z0-9._:-]*$/i.test(token);
}

function isCanonicalPathToken(raw: string, maxLen = 400): boolean {
  const token = cleanText(String(raw || ''), maxLen);
  if (!token) return false;
  if (/^\s|\s$/.test(String(raw || ''))) return false;
  return /^[a-z0-9_./{}:-]+$/i.test(token);
}

function isFiniteNonNegativeNumber(raw: any): boolean {
  const value = Number(raw);
  return Number.isFinite(value) && value >= 0;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const failures: Array<{ id: string; detail: string }> = [];
  const profileRuns = PROFILES.map((profile) => {
    const metricsPath = resolveTemplate(args.metricsTemplate, profile);
    const inspectOutPath = resolveTemplate(args.inspectTemplate, profile);
    const markdownOutPath = resolveTemplate(args.markdownTemplate, profile);
    const reportOutPath = resolveTemplate(args.reportTemplate, profile);

    const exitCode = runBoundednessInspect([
      '--strict=0',
      `--profile=${profile}`,
      `--metrics=${metricsPath}`,
      `--out=${inspectOutPath}`,
      `--out-markdown=${markdownOutPath}`,
      `--out-boundedness-report=${reportOutPath}`,
    ]);
    const payload = readJsonBestEffort(path.resolve(root, inspectOutPath));
    if (!payload || typeof payload !== 'object') {
      failures.push({
        id: 'boundedness_inspect_payload_missing',
        detail: `${profile}:${inspectOutPath}`,
      });
    }
    if (payload?.ok !== true) {
      failures.push({
        id: 'boundedness_profile_not_ok',
        detail: `${profile}:ok=${String(payload?.ok === true)}`,
      });
    }
    if (exitCode !== 0) {
      failures.push({
        id: 'boundedness_inspect_exit_nonzero',
        detail: `${profile}:exit=${exitCode}`,
      });
    }

    return {
      profile,
      exit_code: exitCode,
      inspect_out_path: inspectOutPath,
      markdown_out_path: markdownOutPath,
      report_out_path: reportOutPath,
      payload,
    };
  });

  if (profileRuns.length !== PROFILES.length) {
    failures.push({
      id: 'runtime_boundedness_bundle_profile_run_count_contract_v2',
      detail: `runs=${profileRuns.length};required=${PROFILES.length}`,
    });
  }
  const profileRunIds = profileRuns.map((row) => cleanText(String(row.profile || ''), 40));
  if (new Set(profileRunIds).size !== profileRunIds.length) {
    failures.push({
      id: 'runtime_boundedness_bundle_profile_ids_unique_contract_v2',
      detail: profileRunIds.join(','),
    });
  }
  for (const profileId of profileRunIds) {
    if (!isCanonicalToken(profileId, 40)) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_id_token_contract_v2',
        detail: profileId || 'missing',
      });
    }
  }
  for (const expectedProfile of PROFILES) {
    if (!profileRunIds.includes(expectedProfile)) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_required_set_contract_v2',
        detail: expectedProfile,
      });
    }
  }
  for (const row of profileRuns) {
    if (!isCanonicalPathToken(row.inspect_out_path, 400)) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_inspect_path_token_contract_v2',
        detail: `${row.profile}:${row.inspect_out_path || 'missing'}`,
      });
    }
    if (!isCanonicalPathToken(row.markdown_out_path, 400)) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_markdown_path_token_contract_v2',
        detail: `${row.profile}:${row.markdown_out_path || 'missing'}`,
      });
    }
    if (!isCanonicalPathToken(row.report_out_path, 400)) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_report_path_token_contract_v2',
        detail: `${row.profile}:${row.report_out_path || 'missing'}`,
      });
    }
    if (!Number.isInteger(row.exit_code) || row.exit_code < 0) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_exit_code_scalar_contract_v2',
        detail: `${row.profile}:${String(row.exit_code)}`,
      });
    }
    const payload = row.payload;
    if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_payload_object_contract_v2',
        detail: row.profile,
      });
      continue;
    }
    const summary = payload?.summary;
    if (!summary || typeof summary !== 'object' || Array.isArray(summary)) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_summary_object_contract_v2',
        detail: row.profile,
      });
    }
    const rows = Array.isArray(payload?.rows) ? payload.rows : null;
    if (!rows) {
      failures.push({
        id: 'runtime_boundedness_bundle_profile_rows_array_contract_v2',
        detail: row.profile,
      });
      continue;
    }
    for (const metricRow of rows) {
      const metric = cleanText(String(metricRow?.metric || ''), 80);
      if (!isCanonicalToken(metric, 80)) {
        failures.push({
          id: 'runtime_boundedness_bundle_profile_rows_metric_token_contract_v2',
          detail: `${row.profile}:${metric || 'missing'}`,
        });
      }
      const metricClass = cleanText(String(metricRow?.class || ''), 40);
      if (!['resource', 'stability'].includes(metricClass)) {
        failures.push({
          id: 'runtime_boundedness_bundle_profile_rows_class_token_contract_v2',
          detail: `${row.profile}:${metric}:${metricClass || 'missing'}`,
        });
      }
      const baselineStatus = cleanText(String(metricRow?.baseline_status || ''), 80);
      if (!['within', 'regressed', 'no_baseline'].includes(baselineStatus)) {
        failures.push({
          id: 'runtime_boundedness_bundle_profile_rows_baseline_status_token_contract_v2',
          detail: `${row.profile}:${metric}:${baselineStatus || 'missing'}`,
        });
      }
      const numericFields = [
        ['current', metricRow?.current],
        ['baseline', metricRow?.baseline],
        ['max_allowed', metricRow?.max_allowed],
      ] as const;
      for (const [field, value] of numericFields) {
        if (value !== null && value !== undefined && !isFiniteNonNegativeNumber(value)) {
          failures.push({
            id: 'runtime_boundedness_bundle_profile_rows_numeric_shape_contract_v2',
            detail: `${row.profile}:${metric}:${field}=${String(value)}`,
          });
        }
      }
    }
  }

  const richRun = profileRuns.find((row) => row.profile === 'rich');
  if (richRun) {
    copyArtifact(root, richRun.inspect_out_path, args.outPath);
    copyArtifact(root, richRun.markdown_out_path, args.outMarkdownPath);
  } else {
    failures.push({
      id: 'runtime_boundedness_bundle_rich_profile_presence_contract_v2',
      detail: 'rich_profile_missing',
    });
  }
  if (!isCanonicalPathToken(args.outPath, 400) || !isCanonicalPathToken(args.outMarkdownPath, 400)) {
    failures.push({
      id: 'runtime_boundedness_bundle_copy_target_path_token_contract_v2',
      detail: `out=${args.outPath};markdown=${args.outMarkdownPath}`,
    });
  }

  const profilesPayload = {
    ok: failures.length === 0,
    type: 'runtime_boundedness_profiles',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      ok: row.payload?.ok === true,
      summary: row.payload?.summary || {},
      rows: Array.isArray(row.payload?.rows) ? row.payload.rows : [],
      source_artifact: row.inspect_out_path,
      source_markdown: row.markdown_out_path,
      boundedness_report_artifact: row.report_out_path,
    })),
    failures,
  };
  if (!Array.isArray(profilesPayload.profiles) || profilesPayload.profiles.length !== PROFILES.length) {
    failures.push({
      id: 'runtime_boundedness_bundle_profiles_payload_count_contract_v2',
      detail: `profiles=${Array.isArray(profilesPayload.profiles) ? profilesPayload.profiles.length : 'not_array'};required=${PROFILES.length}`,
    });
  }
  for (const row of failures) {
    const id = cleanText(String(row?.id || ''), 120);
    const detail = cleanText(String(row?.detail || ''), 400);
    if (!isCanonicalToken(id, 120) || !detail) {
      failures.push({
        id: 'runtime_boundedness_bundle_failure_row_shape_contract_v2',
        detail: `${id || 'missing'}:${detail || 'missing'}`,
      });
      break;
    }
  }
  profilesPayload.ok = failures.length === 0;
  writeJsonArtifact(args.profilesOutPath, profilesPayload);

  const bundlePayload = {
    ok: profilesPayload.ok,
    type: 'runtime_boundedness_inspect_bundle',
    generated_at: profilesPayload.generated_at,
    revision: profilesPayload.revision,
    out_path: args.outPath,
    out_markdown_path: args.outMarkdownPath,
    profiles_out_path: args.profilesOutPath,
    profiles: profilesPayload.profiles,
    failures,
    artifact_paths: [
      args.outPath,
      args.outMarkdownPath,
      args.profilesOutPath,
      ...profileRuns.map((row) => row.inspect_out_path),
      ...profileRuns.map((row) => row.markdown_out_path),
      ...profileRuns.map((row) => row.report_out_path),
    ],
  };
  const artifactPaths = Array.isArray(bundlePayload.artifact_paths) ? bundlePayload.artifact_paths : [];
  if (artifactPaths.length === 0 || artifactPaths.some((p) => !cleanText(String(p || ''), 400))) {
    failures.push({
      id: 'runtime_boundedness_bundle_artifact_paths_nonempty_contract_v2',
      detail: `artifact_paths=${artifactPaths.length}`,
    });
  }
  const artifactPathTokens = artifactPaths.map((p) => cleanText(String(p || ''), 400)).filter(Boolean);
  if (new Set(artifactPathTokens).size !== artifactPathTokens.length) {
    failures.push({
      id: 'runtime_boundedness_bundle_artifact_paths_unique_contract_v2',
      detail: artifactPathTokens.join(','),
    });
  }
  for (const artifactPath of artifactPathTokens) {
    if (!isCanonicalPathToken(artifactPath, 400)) {
      failures.push({
        id: 'runtime_boundedness_bundle_artifact_paths_token_contract_v2',
        detail: artifactPath,
      });
    }
  }
  bundlePayload.ok = failures.length === 0;

  return emitStructuredResult(bundlePayload, {
    outPath: '',
    strict: args.strict,
    ok: bundlePayload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
