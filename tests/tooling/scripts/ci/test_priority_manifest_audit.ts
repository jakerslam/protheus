#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_MANIFEST_PATH = 'client/runtime/config/test_priority_manifest.json';
const OUT_JSON = 'core/local/artifacts/test_priority_manifest_audit_current.json';
const OUT_MD = 'local/workspace/reports/TEST_PRIORITY_MANIFEST_AUDIT_CURRENT.md';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { strict: false });
  return {
    strict: common.strict,
    manifestPath: cleanText(readFlag(argv, 'manifest') || DEFAULT_MANIFEST_PATH, 260),
    outJson: cleanText(readFlag(argv, 'out-json') || OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || OUT_MD, 400),
  };
}

function readJson(filePath: string) {
  return JSON.parse(readFileSync(resolve(filePath), 'utf8'));
}

function toMarkdown(payload) {
  const lines = [];
  lines.push('# Test Priority Manifest Audit (Current)');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- pass: ${payload.summary.pass ? 'true' : 'false'}`);
  lines.push(`- strict: ${payload.summary.strict ? 'true' : 'false'}`);
  lines.push(`- tiers: ${payload.summary.tiers}`);
  lines.push(`- declared_scripts: ${payload.summary.declared_scripts}`);
  lines.push(`- missing_scripts: ${payload.summary.missing_scripts}`);
  lines.push(`- duplicate_assignments: ${payload.summary.duplicate_assignments}`);
  lines.push('');
  if (payload.missing_scripts.length > 0) {
    lines.push('## Missing Scripts');
    for (const row of payload.missing_scripts) {
      lines.push(`- ${row}`);
    }
    lines.push('');
  }
  if (payload.duplicate_assignments.length > 0) {
    lines.push('## Duplicate Tier Assignments');
    for (const row of payload.duplicate_assignments) {
      lines.push(`- ${row.script}: ${row.tiers.join(', ')}`);
    }
    lines.push('');
  }
  lines.push('## Tier Breakdown');
  lines.push('| Tier | Script Count |');
  lines.push('| --- | ---: |');
  for (const [tier, scripts] of Object.entries(payload.tiers)) {
    lines.push(`| ${tier} | ${scripts.length} |`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const manifest = readJson(args.manifestPath);
  const packageJson = readJson('package.json');
  const scripts = packageJson.scripts || {};
  const tiers = manifest.tiers || {};

  const missingScripts = [];
  const byScript = new Map();
  let declaredScripts = 0;

  for (const [tier, entries] of Object.entries(tiers)) {
    const list = Array.isArray(entries) ? entries : [];
    for (const script of list) {
      const name = String(script ?? '').trim();
      if (!name) continue;
      declaredScripts += 1;
      if (!Object.prototype.hasOwnProperty.call(scripts, name)) {
        missingScripts.push(`${tier}:${name}`);
      }
      if (!byScript.has(name)) byScript.set(name, []);
      byScript.get(name).push(tier);
    }
  }

  const duplicateAssignments = [];
  for (const [script, tierList] of byScript.entries()) {
    const unique = [...new Set(tierList)];
    if (unique.length > 1) {
      duplicateAssignments.push({ script, tiers: unique.sort() });
    }
  }
  duplicateAssignments.sort((a, b) => a.script.localeCompare(b.script));

  const payload = {
    ok: missingScripts.length === 0 && duplicateAssignments.length === 0,
    type: 'test_priority_manifest_audit',
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    manifest_path: args.manifestPath,
    artifact_paths: [args.outJson, args.outMarkdown],
    tiers,
    missing_scripts: missingScripts.sort(),
    duplicate_assignments: duplicateAssignments,
    summary: {
      strict: args.strict,
      tiers: Object.keys(tiers).length,
      declared_scripts: declaredScripts,
      missing_scripts: missingScripts.length,
      duplicate_assignments: duplicateAssignments.length,
      pass: missingScripts.length === 0 && duplicateAssignments.length === 0,
    },
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  process.exitCode = emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok: payload.summary.pass,
  });
}

main();
