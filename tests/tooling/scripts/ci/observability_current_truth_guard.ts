#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_CONTRACT = 'observability/contracts/current_truth_freshness_contract.json';
const DEFAULT_NOTE = 'observability/contracts/CURRENT_TRUTH_FRESHNESS_CONTRACT.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/observability_current_truth_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/OBSERVABILITY_CURRENT_TRUTH_GUARD_CURRENT.md';

type Violation = { kind: string; path: string; detail: string };

const REQUIRED_TIERS = [
  'current_live_truth',
  'recent_but_not_current',
  'historical_trend',
  'stale_reference_only',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function readJson<T>(relPath: string): T {
  return JSON.parse(readText(relPath)) as T;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    contractPath: cleanText(readFlag(argv, 'contract') || DEFAULT_CONTRACT, 600),
    notePath: cleanText(readFlag(argv, 'note') || DEFAULT_NOTE, 600),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 600),
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
  };
}

function validateContract(contract: any, contractPath: string, violations: Violation[]): void {
  if (contract.type !== 'observability_current_truth_freshness_contract') {
    violations.push({ kind: 'current_truth_contract_type_invalid', path: contractPath, detail: 'Contract type mismatch.' });
  }
  if (contract.owner_domain !== 'observability') {
    violations.push({ kind: 'current_truth_owner_invalid', path: contractPath, detail: 'Owner domain must be observability.' });
  }
  const tiers = Array.isArray(contract.freshness_tiers) ? contract.freshness_tiers : [];
  const tierIds = tiers.map((tier: any) => String(tier.id || ''));
  for (const id of REQUIRED_TIERS) {
    if (!tierIds.includes(id)) {
      violations.push({ kind: 'current_truth_tier_missing', path: contractPath, detail: `Missing freshness tier: ${id}` });
    }
  }
  for (const tier of tiers) {
    const id = String(tier.id || '<missing>');
    if (id === 'current_live_truth') {
      if (tier.decision_authoritative !== true || tier.promotion_allowed !== true) {
        violations.push({ kind: 'current_truth_authority_invalid', path: contractPath, detail: 'current_live_truth must be authoritative and promotable.' });
      }
    } else if (tier.decision_authoritative !== false) {
      violations.push({ kind: 'stale_truth_authority_invalid', path: contractPath, detail: `${id} must not be decision-authoritative.` });
    }
    if (id === 'stale_reference_only' && tier.promotion_allowed !== false) {
      violations.push({ kind: 'stale_truth_promotion_invalid', path: contractPath, detail: 'stale_reference_only must not be promotable.' });
    }
  }
  const consumers = Array.isArray(contract.required_consumers) ? contract.required_consumers : [];
  if (consumers.length < 4) {
    violations.push({ kind: 'current_truth_consumers_incomplete', path: contractPath, detail: 'At least four required consumers are expected.' });
  }
  for (const consumer of consumers) {
    const consumerPath = String(consumer.path || '');
    if (!consumerPath || !fs.existsSync(abs(consumerPath))) {
      violations.push({ kind: 'current_truth_consumer_missing', path: contractPath, detail: `Missing consumer path: ${consumerPath || '<empty>'}` });
      continue;
    }
    const text = readText(consumerPath);
    if (consumerPath.endsWith('freshness.rs')) {
      for (const tier of REQUIRED_TIERS) {
        if (!text.includes(tier)) {
          violations.push({ kind: 'current_truth_source_tier_missing', path: consumerPath, detail: `Freshness source missing tier: ${tier}` });
        }
      }
      if (!text.includes('missing_freshness_fails_closed_as_stale_reference_only')) {
        violations.push({ kind: 'current_truth_missing_freshness_not_fail_closed', path: consumerPath, detail: 'Freshness tests must prove missing freshness fails closed.' });
      }
    }
  }
}

function validateNote(notePath: string, note: string, violations: Violation[]): void {
  for (const token of ['Freshness tiers', 'Required consumers', 'Enforcement']) {
    if (!note.includes(token)) violations.push({ kind: 'current_truth_note_token_missing', path: notePath, detail: `Missing section token: ${token}` });
  }
  for (const tier of REQUIRED_TIERS) {
    if (!note.includes(tier)) violations.push({ kind: 'current_truth_note_tier_missing', path: notePath, detail: `Missing tier token: ${tier}` });
  }
}

function markdown(payload: any): string {
  const lines = [
    '# Observability Current Truth Guard',
    '',
    `- Generated at: ${payload.generated_at}`,
    `- Revision: ${payload.revision}`,
    `- Pass: ${payload.ok}`,
    `- Contract: ${payload.contract_path}`,
    '',
    '## Summary',
  ];
  for (const [key, value] of Object.entries(payload.summary)) lines.push(`- ${key}: ${value}`);
  lines.push('', '## Violations');
  if (!payload.violations.length) lines.push('- none');
  for (const violation of payload.violations) lines.push(`- ${violation.kind}: ${violation.path} ${violation.detail}`);
  return `${lines.join('\n')}\n`;
}

async function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const contract = readJson<any>(args.contractPath);
  const note = readText(args.notePath);
  const violations: Violation[] = [];
  validateContract(contract, args.contractPath, violations);
  validateNote(args.notePath, note, violations);
  if (args.includeControlledViolation) {
    violations.push({ kind: 'controlled_current_truth_violation', path: args.contractPath, detail: 'Controlled failure proves strict mode rejects stale/current truth drift.' });
  }
  const payload = {
    ok: violations.length === 0,
    type: 'observability_current_truth_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    contract_path: args.contractPath,
    note_path: args.notePath,
    controlled_violation: args.includeControlledViolation,
    summary: {
      freshness_tiers: (contract.freshness_tiers || []).length,
      required_consumers: (contract.required_consumers || []).length,
      violations: violations.length,
    },
    violations,
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  emitStructuredResult(payload, { ok: payload.ok, outPath: args.outJson });
  if (args.strict && !payload.ok) process.exitCode = 1;
}

run().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
