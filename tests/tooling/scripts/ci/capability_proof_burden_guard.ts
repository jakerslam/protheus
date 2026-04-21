#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY = 'docs/workspace/policy/capability_proof_burden_policy.md';
const DEFAULT_PR_TEMPLATE = '.github/pull_request_template.md';
const DEFAULT_PR_TEMPLATE_LENSMAP = '.github/PULL_REQUEST_TEMPLATE/lensmap.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/capability_proof_burden_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/CAPABILITY_PROOF_BURDEN_GUARD_CURRENT.md';

function rel(value: string): string {
  return path.relative(ROOT, value).replace(/\\/g, '/');
}

type Args = {
  strict: boolean;
  policyPath: string;
  prTemplatePath: string;
  lensmapTemplatePath: string;
  outJson: string;
  outMarkdown: string;
};

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: common.strict,
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    prTemplatePath: cleanText(readFlag(argv, 'pr-template') || DEFAULT_PR_TEMPLATE, 400),
    lensmapTemplatePath: cleanText(
      readFlag(argv, 'lensmap-template') || DEFAULT_PR_TEMPLATE_LENSMAP,
      400,
    ),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 400),
  };
}

function readFileMaybe(abs: string): string {
  try {
    return fs.readFileSync(abs, 'utf8');
  } catch {
    return '';
  }
}

function requiredFields() {
  return [
    'Proof Artifact / Replay Fixture / Gate',
    'Invariant',
    'Failure Mode',
    'Receipt Surface',
    'Recovery Behavior',
    'Verifiable Runtime Truth Increase',
  ];
}

function templateMarkers() {
  return [
    'required for new or expanded capabilities',
    'gateway/lane/shell-state/control-plane feature',
  ];
}

function policyMarkers() {
  return [
    'Reject feature work that expands exterior capability without verifiable runtime truth increase',
    'Proof Artifact / Replay Fixture / Gate',
    'Invariant',
    'Failure Mode',
    'Receipt Surface',
    'Recovery Behavior',
    'Verifiable Runtime Truth Increase',
  ];
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Capability Proof Burden Guard');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- missing_files: ${payload.summary.missing_files}`);
  lines.push(`- pr_template_required_field_failures: ${payload.summary.pr_template_required_field_failures}`);
  lines.push(`- pr_template_marker_failures: ${payload.summary.pr_template_marker_failures}`);
  lines.push(`- lensmap_required_field_failures: ${payload.summary.lensmap_required_field_failures}`);
  lines.push(`- lensmap_marker_failures: ${payload.summary.lensmap_marker_failures}`);
  lines.push(`- policy_marker_failures: ${payload.summary.policy_marker_failures}`);
  lines.push(`- violation_count: ${payload.summary.violation_count}`);
  lines.push('');
  lines.push('## Missing Files');
  if (!payload.missing_files.length) lines.push('- none');
  else payload.missing_files.forEach((item: string) => lines.push(`- ${item}`));
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) lines.push('- none');
  else payload.violations.forEach((item: any) => lines.push(`- ${item.type}: ${item.detail}`));
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));

  const policyAbs = path.resolve(ROOT, args.policyPath);
  const prAbs = path.resolve(ROOT, args.prTemplatePath);
  const lensAbs = path.resolve(ROOT, args.lensmapTemplatePath);

  const missingFiles: string[] = [];
  for (const abs of [policyAbs, prAbs, lensAbs]) {
    if (!fs.existsSync(abs)) missingFiles.push(rel(abs));
  }

  const policySource = readFileMaybe(policyAbs);
  const prSource = readFileMaybe(prAbs);
  const lensSource = readFileMaybe(lensAbs);

  const required = requiredFields();
  const prFieldMissing = required.filter((token) => !prSource.includes(token));
  const lensFieldMissing = required.filter((token) => !lensSource.includes(token));
  const prMarkerMissing = templateMarkers().filter((token) => !prSource.toLowerCase().includes(token.toLowerCase()));
  const lensMarkerMissing = templateMarkers().filter((token) => !lensSource.toLowerCase().includes(token.toLowerCase()));
  const policyMarkerMissing = policyMarkers().filter((token) => !policySource.includes(token));

  const violations: Array<{ type: string; detail: string }> = [];
  for (const item of missingFiles) {
    violations.push({ type: 'missing_file', detail: item });
  }
  for (const token of prFieldMissing) {
    violations.push({ type: 'pr_template_missing_required_field', detail: token });
  }
  for (const token of prMarkerMissing) {
    violations.push({ type: 'pr_template_missing_marker', detail: token });
  }
  for (const token of lensFieldMissing) {
    violations.push({ type: 'lensmap_template_missing_required_field', detail: token });
  }
  for (const token of lensMarkerMissing) {
    violations.push({ type: 'lensmap_template_missing_marker', detail: token });
  }
  for (const token of policyMarkerMissing) {
    violations.push({ type: 'policy_missing_marker', detail: token });
  }

  const payload = {
    type: 'capability_proof_burden_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    policy_path: rel(policyAbs),
    pr_template_path: rel(prAbs),
    lensmap_template_path: rel(lensAbs),
    summary: {
      missing_files: missingFiles.length,
      pr_template_required_field_failures: prFieldMissing.length,
      pr_template_marker_failures: prMarkerMissing.length,
      lensmap_required_field_failures: lensFieldMissing.length,
      lensmap_marker_failures: lensMarkerMissing.length,
      policy_marker_failures: policyMarkerMissing.length,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    missing_files: missingFiles,
    violations,
  };

  writeTextArtifact(path.resolve(ROOT, args.outMarkdown), toMarkdown({ ...payload, ok: payload.summary.pass }));

  process.exit(
    emitStructuredResult(payload, {
      outPath: path.resolve(ROOT, args.outJson),
      strict: args.strict,
      ok: payload.summary.pass,
    }),
  );
}

main();
