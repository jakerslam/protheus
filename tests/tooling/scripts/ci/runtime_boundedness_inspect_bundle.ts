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
        'core/local/artifacts/runtime_proof_metrics_{profile}_current.json',
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
      inspect_out_path: inspectOutPath,
      markdown_out_path: markdownOutPath,
      report_out_path: reportOutPath,
      payload,
    };
  });

  const richRun = profileRuns.find((row) => row.profile === 'rich');
  if (richRun) {
    copyArtifact(root, richRun.inspect_out_path, args.outPath);
    copyArtifact(root, richRun.markdown_out_path, args.outMarkdownPath);
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
