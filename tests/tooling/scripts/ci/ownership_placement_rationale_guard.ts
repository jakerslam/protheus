#!/usr/bin/env node
/* eslint-disable no-console */
import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type Zone = 'core' | 'control-plane' | 'shell' | 'gateway' | 'apps' | 'other';

type Violation = {
  id: string;
  detail: string;
};

type Args = {
  strict: boolean;
  baseRef: string;
  outJsonPath: string;
  outMarkdownPath: string;
};

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/ownership_placement_rationale_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/OWNERSHIP_PLACEMENT_RATIONALE_GUARD_CURRENT.md';

function parseArgs(argv: string[]): Args {
  const strictOut = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: strictOut.strict,
    baseRef: cleanText(readFlag(argv, 'base-ref') || '', 160),
    outJsonPath: cleanText(readFlag(argv, 'out-json') || strictOut.out || DEFAULT_OUT_JSON, 400),
    outMarkdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function shell(cmd: string): string {
  return execSync(cmd, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
}

function resolveBaseRef(explicit: string): string {
  if (explicit) return explicit;
  const fromEnv = cleanText(String(process.env.GITHUB_BASE_REF || ''), 120);
  if (fromEnv) return `origin/${fromEnv}`;
  try {
    return shell('git rev-parse --verify HEAD~1');
  } catch {
    return shell('git rev-parse --verify HEAD');
  }
}

function changedFiles(baseRef: string): string[] {
  try {
    const raw = shell(`git diff --name-only --diff-filter=ACMR ${baseRef}...HEAD`);
    if (!raw) return [];
    return raw
      .split('\n')
      .map((row) => cleanText(row, 500))
      .filter(Boolean)
      .filter((row) => fs.existsSync(path.resolve(ROOT, row)));
  } catch {
    return [];
  }
}

function mapZone(file: string): Zone {
  const normalized = file.replace(/\\/g, '/');
  if (normalized.startsWith('core/')) return 'core';
  if (normalized.startsWith('surface/orchestration/')) return 'control-plane';
  if (normalized.startsWith('client/')) return 'shell';
  if (normalized.startsWith('adapters/')) return 'gateway';
  if (normalized.startsWith('apps/')) return 'apps';
  return 'other';
}

function loadPrBody(): { eventName: string; body: string } {
  const eventName = cleanText(String(process.env.GITHUB_EVENT_NAME || ''), 80).toLowerCase();
  const eventPath = cleanText(String(process.env.GITHUB_EVENT_PATH || ''), 500);
  if (eventName !== 'pull_request' || !eventPath || !fs.existsSync(eventPath)) {
    return { eventName, body: '' };
  }
  try {
    const payload = JSON.parse(fs.readFileSync(eventPath, 'utf8')) as any;
    return { eventName, body: cleanText(String(payload?.pull_request?.body || ''), 100000) };
  } catch {
    return { eventName, body: '' };
  }
}

function ensureZoneTokens(body: string, zones: Zone[]): Violation[] {
  const normalized = body.toLowerCase();
  const violations: Violation[] = [];
  const requiredCommon = [
    'placement rationale',
    'docs/workspace/orchestration_ownership_policy.md',
  ];
  for (const marker of requiredCommon) {
    if (!normalized.includes(marker)) {
      violations.push({
        id: 'placement_rationale_missing_marker',
        detail: marker,
      });
    }
  }

  const zoneTokenMap: Record<Exclude<Zone, 'other'>, string> = {
    core: 'placement-test:core',
    'control-plane': 'placement-test:control-plane',
    shell: 'placement-test:shell',
    gateway: 'placement-test:gateway',
    apps: 'placement-test:apps',
  };

  for (const zone of zones) {
    if (zone === 'other') continue;
    const token = zoneTokenMap[zone as Exclude<Zone, 'other'>];
    if (!normalized.includes(token)) {
      violations.push({
        id: 'placement_rationale_missing_zone_token',
        detail: `${zone}:${token}`,
      });
    }
  }
  return violations;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Ownership Placement Rationale Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Base ref: ${payload.base_ref}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push('');
  lines.push(`- Event: ${payload.event_name || 'unknown'}`);
  lines.push(`- Changed files: ${payload.summary.changed_file_count}`);
  lines.push(`- Ownership zones touched: ${payload.summary.ownership_zone_count}`);
  lines.push(`- Placement rationale required: ${payload.summary.rationale_required}`);
  lines.push(`- Violation count: ${payload.summary.violation_count}`);
  lines.push('');
  lines.push('## Zones');
  lines.push('');
  lines.push(`- ${payload.ownership_zones.join(', ') || '(none)'}`);
  lines.push('');
  lines.push('## Violations');
  lines.push('');
  if (!Array.isArray(payload.violations) || payload.violations.length === 0) {
    lines.push('- none');
  } else {
    for (const row of payload.violations) {
      lines.push(`- ${row.id}: ${row.detail}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const baseRef = resolveBaseRef(args.baseRef);
  const files = changedFiles(baseRef);
  const zones = Array.from(
    new Set(
      files
        .map((file) => mapZone(file))
        .filter((zone) => zone !== 'other'),
    ),
  ) as Zone[];
  const rationaleRequired = zones.length >= 2;

  const pr = loadPrBody();
  const violations: Violation[] = [];
  if (rationaleRequired && pr.eventName === 'pull_request') {
    if (!pr.body.trim()) {
      violations.push({ id: 'placement_rationale_missing_pr_body', detail: 'pull_request.body empty' });
    } else {
      violations.push(...ensureZoneTokens(pr.body, zones));
    }
  }

  const payload = {
    ok: violations.length === 0,
    type: 'ownership_placement_rationale_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    event_name: pr.eventName,
    base_ref: baseRef,
    ownership_zones: zones,
    changed_files: files,
    summary: {
      pass: violations.length === 0,
      changed_file_count: files.length,
      ownership_zone_count: zones.length,
      rationale_required: rationaleRequired,
      violation_count: violations.length,
    },
    violations,
    failures: violations.map((row) => ({
      id: row.id,
      detail: row.detail,
    })),
    inputs: {
      strict: args.strict,
      out_json: args.outJsonPath,
      out_markdown: args.outMarkdownPath,
    },
  };

  writeTextArtifact(path.resolve(ROOT, args.outMarkdownPath), toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: path.resolve(ROOT, args.outJsonPath),
    strict: args.strict,
    ok: payload.ok,
  });
}

const exitCode = main();
if (exitCode !== 0) process.exit(exitCode);

