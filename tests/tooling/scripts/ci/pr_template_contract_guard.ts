#!/usr/bin/env node
/* eslint-disable no-console */
import { dirname, resolve } from 'node:path';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';

type Args = {
  strict: boolean;
  outJson: string;
  outMd: string;
};

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

const DEFAULT_OUT_JSON = 'core/local/artifacts/pr_template_contract_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/PR_TEMPLATE_CONTRACT_GUARD_CURRENT.md';

function parseArgs(argv: string[]): Args {
  const map = new Map<string, string>();
  for (let i = 2; i < argv.length; i += 1) {
    const token = argv[i] || '';
    if (!token.startsWith('--')) continue;
    const [flag, value] = token.split('=', 2);
    if (value !== undefined) {
      map.set(flag.slice(2), value);
      continue;
    }
    const next = argv[i + 1] || '';
    if (next.length > 0 && !next.startsWith('--')) {
      map.set(flag.slice(2), next);
      i += 1;
    } else {
      map.set(flag.slice(2), '1');
    }
  }
  const strictRaw = (map.get('strict') || '').toLowerCase();
  const strict = strictRaw === '1' || strictRaw === 'true' || strictRaw === 'yes';
  return {
    strict,
    outJson: (map.get('out-json') || DEFAULT_OUT_JSON).trim(),
    outMd: (map.get('out-md') || DEFAULT_OUT_MD).trim(),
  };
}

function ensureDir(path: string): void {
  mkdirSync(dirname(path), { recursive: true });
}

function run(): number {
  const args = parseArgs(process.argv);
  const templatePath = resolve('.github/pull_request_template.md');
  const source = readFileSync(templatePath, 'utf8');

  const checks: Check[] = [
    {
      id: 'pr_template_contains_layer_ownership_proof_gate_question',
      ok: source.includes('Which layer owns this, and which proof/gate covers it?'),
      detail:
        'template must require explicit ownership/proof-gate declaration phrasing',
    },
    {
      id: 'pr_template_contains_capability_invariant_column',
      ok: source.includes('| Invariant |'),
      detail: 'capability proof table must include invariant column',
    },
    {
      id: 'pr_template_contains_capability_failure_mode_column',
      ok: source.includes('| Failure Mode |'),
      detail: 'capability proof table must include failure-mode column',
    },
    {
      id: 'pr_template_contains_capability_receipt_surface_column',
      ok: source.includes('| Receipt Surface |'),
      detail: 'capability proof table must include receipt-surface column',
    },
    {
      id: 'pr_template_contains_capability_recovery_behavior_column',
      ok: source.includes('| Recovery Behavior |'),
      detail: 'capability proof table must include recovery-behavior column',
    },
    {
      id: 'pr_template_enforces_truth_increase_checkbox',
      ok: source.includes(
        'No exterior capability expansion without verifiable runtime truth increase.',
      ),
      detail: 'template must include verifiable runtime truth increase assertion',
    },
    {
      id: 'pr_template_contains_runtime_closure_alignment_section',
      ok: source.includes('## Runtime Closure Feature Alignment (required for major surface features)'),
      detail: 'template must include runtime-closure alignment section for major features',
    },
    {
      id: 'pr_template_contains_runtime_closure_bucket_column',
      ok: source.includes('| Runtime Closure Bucket |'),
      detail: 'runtime-closure alignment table must include runtime closure bucket column',
    },
    {
      id: 'pr_template_contains_runtime_closure_validation_column',
      ok: source.includes('| Validation Artifact / Gate |'),
      detail: 'runtime-closure alignment table must include validation artifact/gate column',
    },
    {
      id: 'pr_template_enforces_major_feature_bucket_validation_checkbox',
      ok: source.includes(
        'If any feature scope is `major`, each major feature maps to a runtime-closure bucket and directly validates it with a linked proof artifact, replay fixture, or release gate.',
      ),
      detail: 'template must enforce major feature runtime-closure bucket validation',
    },
    {
      id: 'pr_template_enforces_visible_capability_proof_link_checkbox',
      ok: source.includes(
        'Every visible capability change links to at least one proof artifact, replay fixture, or release gate.',
      ),
      detail: 'template must enforce visible capability proof-link declaration',
    },
  ];

  const ok = checks.every((row) => row.ok);
  const payload = {
    ok,
    strict: args.strict,
    checks,
    generated_at: new Date().toISOString(),
  };
  const md = [
    '# PR TEMPLATE CONTRACT GUARD',
    '',
    `- ok: ${ok}`,
    `- strict: ${args.strict}`,
    '',
    '## Checks',
    ...checks.map(
      (row) => `- [${row.ok ? 'x' : ' '}] \`${row.id}\` — ${row.detail}`,
    ),
    '',
  ].join('\n');

  ensureDir(args.outJson);
  ensureDir(args.outMd);
  writeFileSync(args.outJson, JSON.stringify(payload, null, 2));
  writeFileSync(args.outMd, md);
  console.log(JSON.stringify(payload, null, 2));

  if (args.strict && !ok) return 1;
  return 0;
}

process.exit(run());
