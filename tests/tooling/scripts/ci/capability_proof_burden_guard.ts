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
const OWNERSHIP_PROOF_SECTION_HEADING =
  'Capability Ownership + Proof Coverage (required for each net-new capability)';

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
    'required for each net-new capability',
    'Capability Ownership + Proof Coverage',
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

type ParsedTable = {
  headers: string[];
  rows: string[][];
};

function sectionBody(markdown: string, heading: string): string {
  const escaped = heading.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const start = new RegExp(`^##\\s+${escaped}\\s*$`, 'im').exec(markdown);
  if (!start || start.index == null) return '';
  const rest = markdown.slice(start.index + start[0].length);
  const end = /^\s*##\s+/m.exec(rest);
  return (end ? rest.slice(0, end.index) : rest).trim();
}

function parseMarkdownTable(section: string): ParsedTable | null {
  const lines = section
    .split('\n')
    .map((line) => cleanText(line, 5000).trim())
    .filter((line) => line.startsWith('|') && line.endsWith('|'));
  if (lines.length < 2) return null;
  const parseRow = (line: string): string[] =>
    line
      .slice(1, -1)
      .split('|')
      .map((cell) => cleanText(cell, 500).trim());
  const headers = parseRow(lines[0]);
  const divider = parseRow(lines[1]);
  if (headers.length === 0 || headers.length !== divider.length) return null;
  if (!divider.every((cell) => /^:?-{2,}:?$/.test(cell))) return null;
  const rows = lines.slice(2).map(parseRow).filter((row) => row.length === headers.length);
  return { headers, rows };
}

function indexOfHeader(headers: string[], expected: string): number {
  const expectedNorm = cleanText(expected, 200).toLowerCase();
  return headers.findIndex((header) => cleanText(header, 200).toLowerCase() === expectedNorm);
}

function loadPrBody(): { eventName: string; body: string } {
  const eventName = cleanText(String(process.env.GITHUB_EVENT_NAME || ''), 80).toLowerCase();
  const eventPath = cleanText(String(process.env.GITHUB_EVENT_PATH || ''), 500);
  if (eventName !== 'pull_request' || !eventPath || !fs.existsSync(eventPath)) {
    return { eventName, body: '' };
  }
  try {
    const payload = JSON.parse(fs.readFileSync(eventPath, 'utf8')) as any;
    return { eventName, body: cleanText(String(payload?.pull_request?.body || ''), 200000) };
  } catch {
    return { eventName, body: '' };
  }
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
  lines.push(`- pr_metadata_mapping_failures: ${payload.summary.pr_metadata_mapping_failures}`);
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

  if (!prSource.includes(OWNERSHIP_PROOF_SECTION_HEADING)) {
    violations.push({
      type: 'pr_template_missing_marker',
      detail: OWNERSHIP_PROOF_SECTION_HEADING,
    });
  }
  if (!lensSource.includes(OWNERSHIP_PROOF_SECTION_HEADING)) {
    violations.push({
      type: 'lensmap_template_missing_marker',
      detail: OWNERSHIP_PROOF_SECTION_HEADING,
    });
  }

  const prRuntime = loadPrBody();
  if (prRuntime.eventName === 'pull_request' && prRuntime.body.trim().length > 0) {
    const capabilitySection = sectionBody(prRuntime.body, 'Capability Proof Burden');
    const capabilityTable = parseMarkdownTable(capabilitySection);
    if (!capabilityTable) {
      violations.push({
        type: 'pr_metadata_capability_table_missing',
        detail: 'Capability Proof Burden table not found',
      });
    } else {
      const capabilityIx = indexOfHeader(capabilityTable.headers, 'Capability');
      if (capabilityIx < 0) {
        violations.push({
          type: 'pr_metadata_capability_header_missing',
          detail: 'Capability',
        });
      } else {
        const netNewCapabilities = capabilityTable.rows
          .map((row) => cleanText(row[capabilityIx] || '', 260))
          .filter((value) => value.length > 0)
          .map((value) => value.toLowerCase());

        const ownershipSection = sectionBody(prRuntime.body, OWNERSHIP_PROOF_SECTION_HEADING);
        const ownershipTable = parseMarkdownTable(ownershipSection);
        if (!ownershipTable) {
          violations.push({
            type: 'pr_metadata_ownership_mapping_table_missing',
            detail: OWNERSHIP_PROOF_SECTION_HEADING,
          });
        } else {
          const mappingCapabilityIx = indexOfHeader(ownershipTable.headers, 'Capability');
          const ownerLayerIx = indexOfHeader(ownershipTable.headers, 'Owner Layer');
          const proofCoverageIx = indexOfHeader(ownershipTable.headers, 'Proof / Gate Coverage');
          if (mappingCapabilityIx < 0 || ownerLayerIx < 0 || proofCoverageIx < 0) {
            violations.push({
              type: 'pr_metadata_ownership_mapping_headers_missing',
              detail: ownershipTable.headers.join('|'),
            });
          } else {
            const allowedLayers = new Set([
              'kernel',
              'control_plane',
              'shell',
              'gateway',
              'apps',
            ]);
            const mappingByCapability = new Map<string, { owner: string; proof: string }>();
            for (const row of ownershipTable.rows) {
              const capability = cleanText(row[mappingCapabilityIx] || '', 260).toLowerCase();
              const owner = cleanText(row[ownerLayerIx] || '', 120).toLowerCase();
              const proof = cleanText(row[proofCoverageIx] || '', 320);
              if (!capability) continue;
              mappingByCapability.set(capability, { owner, proof });
              if (!allowedLayers.has(owner)) {
                violations.push({
                  type: 'pr_metadata_ownership_mapping_owner_invalid',
                  detail: `${capability}:${owner}`,
                });
              }
              const hasProofAnchor =
                proof.includes('ops:')
                || proof.includes('core/local/artifacts/')
                || proof.includes('local/workspace/reports/');
              if (!hasProofAnchor) {
                violations.push({
                  type: 'pr_metadata_ownership_mapping_proof_anchor_missing',
                  detail: `${capability}:${proof}`,
                });
              }
            }
            for (const capability of netNewCapabilities) {
              if (!mappingByCapability.has(capability)) {
                violations.push({
                  type: 'pr_metadata_ownership_mapping_missing_for_capability',
                  detail: capability,
                });
              }
            }
          }
        }
      }
    }
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
      pr_metadata_mapping_failures: violations.filter((row) =>
        row.type.startsWith('pr_metadata_'),
      ).length,
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
