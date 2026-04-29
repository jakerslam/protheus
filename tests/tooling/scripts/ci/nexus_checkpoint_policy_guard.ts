#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT = 'core/local/artifacts/nexus_checkpoint_policy_guard_current.json';
const POLICY_PATH = 'docs/workspace/nexus_conduit_checkpoint_policy.md';
const FEDERATION_RESOLUTION_POLICY_PATH = 'docs/workspace/layered_nexus_federation_resolution_policy.md';

const REQUIRED_POLICY_PHRASES = [
  'Nexus-Conduit-Checkpoint Policy',
  'Modules do work.',
  'Nexuses manage relationships.',
  'Conduits carry traffic.',
  'Kernel policy decides what is allowed.',
  'Receipts prove what happened.',
  'Nexus checkpoint surface',
  'No direct cross-module code path may bypass a Nexus checkpoint surface.',
  'No cross-boundary traffic may bypass Conduit.',
  'route lease',
  'Scrambler',
  'Shell shows and collects',
  'If any answer is missing, the path is not compliant.',
];

const REQUIRED_REFERENCES = [
  'docs/workspace/codex_enforcer.md',
  'docs/workspace/orchestration_ownership_policy.md',
  'docs/client/architecture/LAYER_RULEBOOK.md',
  'docs/SYSTEM-ARCHITECTURE-SPECS.md',
];

const REQUIRED_FEDERATION_RESOLUTION_PHRASES = [
  'Layered Nexus Federation Resolution Policy',
  'The exact historical three-domain Layered Nexus Federation runtime shape is retired.',
  'core/layer2/nexus',
  'Nexus checkpoint surface',
  'Conduit',
  'route inventory',
];

const REQUIRED_NEXUS_FILES = [
  'core/layer2/nexus/src/lib.rs',
  'core/layer2/nexus/src/main_nexus.rs',
  'core/layer2/nexus/src/sub_nexus.rs',
  'core/layer2/nexus/src/conduit_manager.rs',
  'core/layer2/nexus/src/route_lease.rs',
  'core/layer2/nexus/src/template.rs',
  'core/layer2/nexus/src/registry.rs',
  'core/layer2/nexus/src/policy.rs',
];

function read(filePath: string): string {
  const abs = path.resolve(ROOT, filePath);
  if (!fs.existsSync(abs)) return '';
  return fs.readFileSync(abs, 'utf8');
}

function exists(filePath: string): boolean {
  return fs.existsSync(path.resolve(ROOT, filePath));
}

function missingPhrases(source: string, phrases: string[]): string[] {
  return phrases.filter((phrase) => !source.includes(phrase));
}

function srsHasStaleMissingNexusDoneEvidence(srs: string): boolean {
  return srs
    .split(/\r?\n/)
    .some((line) =>
      line.includes('| V6-ARCH-004 | done |') &&
      line.includes('core/layer0/nexus') &&
      !exists('core/layer0/nexus'),
    );
}

function main() {
  const args = process.argv.slice(2);
  const common = parseStrictOutArgs(args, {});
  const out = cleanText(readFlag(args, 'out') || DEFAULT_OUT, 400);

  const failures: Array<{ reason: string; file?: string; markers?: string[] }> = [];
  const policy = read(POLICY_PATH);
  if (!policy) {
    failures.push({ reason: 'canonical_policy_missing', file: POLICY_PATH });
  } else {
    const missing = missingPhrases(policy, REQUIRED_POLICY_PHRASES);
    if (missing.length > 0) {
      failures.push({ reason: 'canonical_policy_required_phrase_missing', file: POLICY_PATH, markers: missing });
    }
  }

  for (const file of REQUIRED_REFERENCES) {
    const source = read(file);
    if (!source.includes(POLICY_PATH)) {
      failures.push({ reason: 'canonical_policy_reference_missing', file, markers: [POLICY_PATH] });
    }
    if (!source.includes(FEDERATION_RESOLUTION_POLICY_PATH)) {
      failures.push({
        reason: 'federation_resolution_policy_reference_missing',
        file,
        markers: [FEDERATION_RESOLUTION_POLICY_PATH],
      });
    }
  }

  const federationResolutionPolicy = read(FEDERATION_RESOLUTION_POLICY_PATH);
  if (!federationResolutionPolicy) {
    failures.push({ reason: 'federation_resolution_policy_missing', file: FEDERATION_RESOLUTION_POLICY_PATH });
  } else {
    const missing = missingPhrases(federationResolutionPolicy, REQUIRED_FEDERATION_RESOLUTION_PHRASES);
    if (missing.length > 0) {
      failures.push({
        reason: 'federation_resolution_policy_required_phrase_missing',
        file: FEDERATION_RESOLUTION_POLICY_PATH,
        markers: missing,
      });
    }
  }

  const missingNexusFiles = REQUIRED_NEXUS_FILES.filter((file) => !exists(file));
  if (missingNexusFiles.length > 0) {
    failures.push({ reason: 'required_layer2_nexus_file_missing', markers: missingNexusFiles });
  }

  const packageJson = read('package.json');
  const packageScripts = JSON.parse(packageJson).scripts ?? {};
  const nexusGovernanceScript = String(packageScripts['ops:nexus:governance'] ?? '');
  const nexusGovernanceCommands = nexusGovernanceScript
    .split('&&')
    .map((command) => command.trim().replace(/^npm run -s\s+/, ''))
    .filter(Boolean);
  if (!packageJson.includes('ops:nexus:checkpoint-policy:guard')) {
    failures.push({
      reason: 'package_script_missing',
      file: 'package.json',
      markers: ['ops:nexus:checkpoint-policy:guard'],
    });
  }
  if (nexusGovernanceCommands[0] !== 'ops:nexus:checkpoint-policy:guard') {
    failures.push({
      reason: 'nexus_governance_does_not_start_with_checkpoint_policy_guard',
      file: 'package.json',
      markers: ['ops:nexus:governance'],
    });
  }
  if (!nexusGovernanceCommands.includes('ops:nexus:route-inventory:guard')) {
    failures.push({
      reason: 'nexus_governance_missing_route_inventory_guard',
      file: 'package.json',
      markers: ['ops:nexus:governance', 'ops:nexus:route-inventory:guard'],
    });
  }

  const srs = read('docs/workspace/SRS.md');
  if (srsHasStaleMissingNexusDoneEvidence(srs)) {
    failures.push({
      reason: 'srs_done_row_cites_missing_layer0_nexus_evidence',
      file: 'docs/workspace/SRS.md',
      markers: ['V6-ARCH-004', 'core/layer0/nexus'],
    });
  }
  const srsDoneEvidenceIndex = read('tests/tooling/data/srs_done_evidence_index.json');
  if (srsDoneEvidenceIndex.includes('core/layer0/nexus') && !exists('core/layer0/nexus')) {
    failures.push({
      reason: 'srs_done_evidence_index_cites_missing_layer0_nexus_evidence',
      file: 'tests/tooling/data/srs_done_evidence_index.json',
      markers: ['core/layer0/nexus'],
    });
  }

  const payload = {
    type: 'nexus_checkpoint_policy_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      policy_path: POLICY_PATH,
      checked_reference_count: REQUIRED_REFERENCES.length,
      checked_nexus_file_count: REQUIRED_NEXUS_FILES.length,
      federation_resolution_policy_path: FEDERATION_RESOLUTION_POLICY_PATH,
      failure_count: failures.length,
      pass: failures.length === 0,
    },
    required_reference_files: REQUIRED_REFERENCES,
    required_nexus_files: REQUIRED_NEXUS_FILES,
    failures,
  };

  process.exit(
    emitStructuredResult(payload, {
      outPath: out,
      strict: common.strict,
      ok: payload.summary.pass,
    }),
  );
}

main();
