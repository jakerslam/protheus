#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type SupportLevel = 'public_stable' | 'experimental_opt_in' | 'internal_only';

type SurfaceRow = {
  id: string;
  path: string;
  support_level: SupportLevel;
  release_required: boolean;
};

type RuntimeLaneStateContract = {
  path: string;
  required_counter_keys: string[];
};

type Manifest = {
  version: number;
  allowed_support_levels: SupportLevel[];
  required_readme_markers: string[];
  runtime_lane_state: RuntimeLaneStateContract;
  surfaces: SurfaceRow[];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/agent_surface_status_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'tests/tooling/config/agent_surface_status_manifest.json',
      400,
    ),
    readmePath: cleanText(readFlag(argv, 'readme') || 'README.md', 400),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/AGENT_SURFACE_STATUS_GUARD_CURRENT.md',
      400,
    ),
  };
}

function readJsonBestEffort(filePath: string): { ok: boolean; payload: any; detail: string } {
  try {
    return {
      ok: true,
      payload: JSON.parse(fs.readFileSync(filePath, 'utf8')),
      detail: 'loaded',
    };
  } catch (error) {
    return {
      ok: false,
      payload: null,
      detail: cleanText((error as Error)?.message || 'json_unavailable', 240),
    };
  }
}

function readTextBestEffort(filePath: string): { ok: boolean; payload: string; detail: string } {
  try {
    return {
      ok: true,
      payload: fs.readFileSync(filePath, 'utf8'),
      detail: 'loaded',
    };
  } catch (error) {
    return {
      ok: false,
      payload: '',
      detail: cleanText((error as Error)?.message || 'text_unavailable', 240),
    };
  }
}

function toMarkdown(report: any): string {
  const lines = [
    '# Agent Surface Status Guard',
    '',
    `Generated: ${report.generated_at}`,
    `Revision: ${report.revision}`,
    `Pass: ${report.ok}`,
    '',
    '## Summary',
    `- surfaces_total: ${report.summary.surfaces_total}`,
    `- release_required_total: ${report.summary.release_required_total}`,
    `- support_levels: ${JSON.stringify(report.summary.support_levels)}`,
    `- violations: ${report.summary.violation_count}`,
    '',
    '| surface | support_level | release_required | exists |',
    '| --- | --- | :---: | :---: |',
  ];
  for (const row of report.surfaces) {
    lines.push(
      `| ${row.id} | ${row.support_level} | ${row.release_required ? 'yes' : 'no'} | ${
        row.exists ? 'yes' : 'no'
      } |`,
    );
  }
  lines.push('');
  lines.push('## Violations');
  if (!report.failures.length) {
    lines.push('- none');
  } else {
    for (const failure of report.failures) {
      lines.push(`- ${failure.id}: ${failure.detail}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  const manifestAbs = path.resolve(root, args.manifestPath);
  const manifestRaw = readJsonBestEffort(manifestAbs);
  if (!manifestRaw.ok) {
    const payload = {
      ok: false,
      type: 'agent_surface_status_guard',
      error: 'agent_surface_status_manifest_unavailable',
      detail: manifestRaw.detail,
      manifest_path: args.manifestPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }
  const manifest = manifestRaw.payload as Manifest;

  const failures: Array<{ id: string; detail: string }> = [];
  const ids = new Set<string>();
  const allowedLevels = new Set(
    (Array.isArray(manifest.allowed_support_levels) ? manifest.allowed_support_levels : []).map((row) =>
      cleanText(row, 80),
    ),
  );
  const surfaces = Array.isArray(manifest.surfaces) ? manifest.surfaces : [];
  if (surfaces.length === 0) {
    failures.push({ id: 'agent_surface_manifest_empty', detail: 'surfaces array is empty' });
  }

  const surfaceRows = surfaces.map((row) => {
    const id = cleanText(row?.id || '', 120);
    const relPath = cleanText(row?.path || '', 500);
    const supportLevel = cleanText(row?.support_level || '', 80);
    const releaseRequired = row?.release_required === true;
    const exists = relPath.length > 0 && fs.existsSync(path.resolve(root, relPath));
    if (!id) failures.push({ id: 'agent_surface_missing_id', detail: relPath || 'unknown_path' });
    if (id && ids.has(id)) failures.push({ id: 'agent_surface_duplicate_id', detail: id });
    ids.add(id);
    if (!relPath) failures.push({ id: 'agent_surface_missing_path', detail: id || 'unknown_surface' });
    if (supportLevel.length === 0 || !allowedLevels.has(supportLevel)) {
      failures.push({
        id: 'agent_surface_invalid_support_level',
        detail: `${id || 'unknown_surface'}:${supportLevel || 'missing'}`,
      });
    }
    if (!exists) {
      failures.push({
        id: 'agent_surface_path_missing',
        detail: `${id || 'unknown_surface'}:${relPath || 'missing_path'}`,
      });
    }
    return {
      id,
      path: relPath,
      support_level: supportLevel,
      release_required: releaseRequired,
      exists,
    };
  });

  const readmeAbs = path.resolve(root, args.readmePath);
  const readmeRaw = readTextBestEffort(readmeAbs);
  if (!readmeRaw.ok) {
    failures.push({ id: 'agent_surface_readme_unavailable', detail: readmeRaw.detail });
  } else {
    for (const marker of Array.isArray(manifest.required_readme_markers)
      ? manifest.required_readme_markers
      : []) {
      const normalized = cleanText(marker, 300);
      if (normalized && !readmeRaw.payload.includes(normalized)) {
        failures.push({ id: 'agent_surface_readme_marker_missing', detail: normalized });
      }
    }
  }

  const runtimeStatePath = cleanText(manifest.runtime_lane_state?.path || '', 400);
  const runtimeStateAbs = runtimeStatePath ? path.resolve(root, runtimeStatePath) : '';
  const runtimeStateRaw = runtimeStateAbs ? readJsonBestEffort(runtimeStateAbs) : { ok: false, payload: null, detail: 'missing_runtime_lane_state_path' };
  if (!runtimeStateRaw.ok) {
    failures.push({ id: 'agent_surface_runtime_lane_state_unavailable', detail: runtimeStateRaw.detail });
  } else {
    const counters = runtimeStateRaw.payload?.release_gate_counters || {};
    const requiredCounterKeys = Array.isArray(manifest.runtime_lane_state?.required_counter_keys)
      ? manifest.runtime_lane_state.required_counter_keys
      : [];
    for (const key of requiredCounterKeys) {
      if (!Object.prototype.hasOwnProperty.call(counters, key)) {
        failures.push({ id: 'agent_surface_runtime_lane_counter_missing', detail: cleanText(key, 80) });
      }
    }
  }

  const supportLevelCounts = surfaceRows.reduce(
    (acc, row) => {
      const key = cleanText(row.support_level || 'unknown', 80);
      acc[key] = (acc[key] || 0) + 1;
      return acc;
    },
    {} as Record<string, number>,
  );
  const releaseRequiredTotal = surfaceRows.filter((row) => row.release_required).length;

  const report = {
    ok: failures.length === 0,
    type: 'agent_surface_status_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    manifest_path: args.manifestPath,
    readme_path: args.readmePath,
    runtime_lane_state_path: runtimeStatePath,
    summary: {
      surfaces_total: surfaceRows.length,
      release_required_total: releaseRequiredTotal,
      support_levels: supportLevelCounts,
      violation_count: failures.length,
    },
    surfaces: surfaceRows,
    failures,
    artifact_paths: [args.markdownOutPath],
  };

  writeTextArtifact(args.markdownOutPath, toMarkdown(report));
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
