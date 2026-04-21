#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type RedundancyRule = {
  path_pattern: string;
  min_reviewers: number;
  required_owners: string[];
};

type RedundancyPolicy = {
  schema_id: string;
  schema_version: string;
  codeowners_path: string;
  required_entries: RedundancyRule[];
};

type CodeownersRow = {
  pattern: string;
  owners: string[];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/release_lane_reviewer_redundancy_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 500),
    outMarkdown: cleanText(
      readFlag(argv, 'out-markdown') ||
        'local/workspace/reports/RELEASE_LANE_REVIEWER_REDUNDANCY_GUARD_CURRENT.md',
      500,
    ),
    policyPath: cleanText(
      readFlag(argv, 'policy') || 'tests/tooling/config/release_lane_reviewer_redundancy_policy.json',
      500,
    ),
  };
}

function parseCodeowners(source: string): CodeownersRow[] {
  const rows: CodeownersRow[] = [];
  for (const raw of source.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const parts = line.split(/\s+/).filter(Boolean);
    if (parts.length < 2) continue;
    const pattern = cleanText(parts[0], 300);
    const owners = parts
      .slice(1)
      .map((token) => cleanText(token, 200))
      .filter((token) => token.startsWith('@'));
    if (!pattern || owners.length === 0) continue;
    rows.push({ pattern, owners: Array.from(new Set(owners)) });
  }
  return rows;
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Release Lane Reviewer Redundancy Guard');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- required_rules: ${payload.summary.required_rules}`);
  lines.push(`- codeowners_rows: ${payload.summary.codeowners_rows}`);
  lines.push(`- failures: ${payload.summary.failures}`);
  lines.push('');
  lines.push('| path_pattern | owners | min_reviewers | pass |');
  lines.push('| --- | --- | ---: | --- |');
  for (const row of payload.rows || []) {
    lines.push(
      `| ${cleanText(row.path_pattern || '', 160)} | ${cleanText((row.owners || []).join(', '), 220)} | ${row.min_reviewers} | ${row.ok ? 'true' : 'false'} |`,
    );
  }
  lines.push('');
  lines.push('## Failures');
  if (!Array.isArray(payload.failures) || payload.failures.length === 0) {
    lines.push('- none');
  } else {
    for (const failure of payload.failures) {
      lines.push(`- ${cleanText(failure.id || '', 120)}: ${cleanText(failure.detail || '', 240)}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const policyAbs = path.resolve(root, args.policyPath);
  let policy: RedundancyPolicy;

  try {
    policy = JSON.parse(fs.readFileSync(policyAbs, 'utf8')) as RedundancyPolicy;
  } catch (error) {
    return emitStructuredResult(
      {
        ok: false,
        type: 'release_lane_reviewer_redundancy_guard',
        error: 'redundancy_policy_unavailable',
        detail: cleanText((error as Error)?.message || 'policy_unavailable', 240),
        policy_path: args.policyPath,
      },
      { outPath: args.outPath, strict: args.strict, ok: false },
    );
  }

  const codeownersRel = cleanText(policy.codeowners_path || '.github/CODEOWNERS', 500);
  const codeownersAbs = path.resolve(root, codeownersRel);
  if (!fs.existsSync(codeownersAbs)) {
    return emitStructuredResult(
      {
        ok: false,
        type: 'release_lane_reviewer_redundancy_guard',
        error: 'codeowners_missing',
        codeowners_path: codeownersRel,
      },
      { outPath: args.outPath, strict: args.strict, ok: false },
    );
  }

  const codeownersRows = parseCodeowners(fs.readFileSync(codeownersAbs, 'utf8'));
  const byPattern = new Map(codeownersRows.map((row) => [row.pattern, row]));
  const rules = Array.isArray(policy.required_entries) ? policy.required_entries : [];
  const failures: Array<{ id: string; detail: string }> = [];
  const rows: Array<any> = [];

  for (const rule of rules) {
    const pathPattern = cleanText(rule.path_pattern || '', 300);
    const minReviewers = Number(rule.min_reviewers || 0);
    const requiredOwners = Array.isArray(rule.required_owners)
      ? rule.required_owners.map((owner) => cleanText(owner, 200)).filter(Boolean)
      : [];
    if (!pathPattern) {
      failures.push({ id: 'redundancy_rule_invalid_path_pattern', detail: 'missing_path_pattern' });
      continue;
    }
    const row = byPattern.get(pathPattern);
    if (!row) {
      failures.push({ id: 'codeowners_required_pattern_missing', detail: pathPattern });
      rows.push({
        path_pattern: pathPattern,
        owners: [],
        min_reviewers: minReviewers,
        ok: false,
      });
      continue;
    }
    const owners = Array.from(new Set(row.owners));
    let ok = true;
    if (owners.length < minReviewers) {
      failures.push({
        id: 'codeowners_reviewer_count_insufficient',
        detail: `${pathPattern}:owners=${owners.length};min=${minReviewers}`,
      });
      ok = false;
    }
    for (const requiredOwner of requiredOwners) {
      if (!owners.includes(requiredOwner)) {
        failures.push({
          id: 'codeowners_required_owner_missing',
          detail: `${pathPattern}:owner=${requiredOwner}`,
        });
        ok = false;
      }
    }
    rows.push({
      path_pattern: pathPattern,
      owners,
      min_reviewers: minReviewers,
      ok,
    });
  }

  const payload = {
    ok: failures.length === 0,
    type: 'release_lane_reviewer_redundancy_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    policy_path: args.policyPath,
    codeowners_path: codeownersRel,
    summary: {
      required_rules: rules.length,
      codeowners_rows: codeownersRows.length,
      failures: failures.length,
    },
    rows,
    failures,
  };

  writeTextArtifact(args.outMarkdown, markdown(payload));
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
