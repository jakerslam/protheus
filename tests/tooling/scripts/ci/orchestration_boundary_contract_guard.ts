#!/usr/bin/env node
/* eslint-disable no-console */
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { dirname, extname, join, resolve } from 'node:path';

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

type Args = {
  strict: boolean;
  outJson: string;
  outMd: string;
};

const DEFAULT_OUT_JSON =
  'core/local/artifacts/orchestration_boundary_contract_guard_current.json';
const DEFAULT_OUT_MD =
  'local/workspace/reports/ORCHESTRATION_BOUNDARY_CONTRACT_GUARD_CURRENT.md';

const REQUIRED_CONTROL_PLANE_MODULES = [
  'intake_normalization',
  'decomposition_planning',
  'workflow_graph_dependency',
  'recovery_escalation',
  'result_shaping_packaging',
] as const;

const FORBIDDEN_AUTHORITY_TOKENS = [
  'canonical_policy_truth',
  'execution_admission_truth',
  'deterministic_receipt_authority',
  'scheduler_truth',
  'queue_truth',
] as const;

function parseArgs(argv: string[]): Args {
  const byName = new Map<string, string>();
  for (let i = 2; i < argv.length; i += 1) {
    const token = argv[i] || '';
    if (!token.startsWith('--')) continue;
    const [name, raw] = token.split('=', 2);
    if (raw !== undefined) {
      byName.set(name.slice(2), raw);
      continue;
    }
    const next = argv[i + 1] || '';
    if (!next.startsWith('--') && next.length > 0) {
      byName.set(name.slice(2), next);
      i += 1;
    } else {
      byName.set(name.slice(2), '1');
    }
  }
  const strictValue = (byName.get('strict') || '').toLowerCase();
  const strict = strictValue === '1' || strictValue === 'true' || strictValue === 'yes';
  const outJson = (byName.get('out-json') || DEFAULT_OUT_JSON).trim();
  const outMd = (byName.get('out-md') || DEFAULT_OUT_MD).trim();
  return { strict, outJson, outMd };
}

function walkFiles(root: string): string[] {
  const out: string[] = [];
  const stack = [root];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    let entries: string[] = [];
    try {
      entries = readdirSync(current);
    } catch {
      continue;
    }
    for (const entry of entries) {
      const full = join(current, entry);
      let isDir = false;
      try {
        isDir = statSync(full).isDirectory();
      } catch {
        continue;
      }
      if (isDir) {
        stack.push(full);
      } else {
        out.push(full);
      }
    }
  }
  return out;
}

function ensureDirFor(filePath: string): void {
  const fs = require('node:fs') as typeof import('node:fs');
  fs.mkdirSync(dirname(filePath), { recursive: true });
}

function rel(path: string): string {
  return path.replace(/\\/g, '/');
}

function run(): number {
  const args = parseArgs(process.argv);
  const checks: Check[] = [];

  const controlPlaneModPath = resolve('surface/orchestration/src/control_plane/mod.rs');
  const controlPlaneModSource = readFileSync(controlPlaneModPath, 'utf8');

  const missingMods = REQUIRED_CONTROL_PLANE_MODULES.filter(
    (name) => !controlPlaneModSource.includes(`pub mod ${name};`),
  );
  checks.push({
    id: 'orchestration_control_plane_required_subdomains_present',
    ok: missingMods.length === 0,
    detail:
      missingMods.length === 0
        ? 'required control-plane subdomain module exports are present'
        : `missing required control-plane module exports: ${missingMods.join(', ')}`,
  });

  const missingBoundaryRows = REQUIRED_CONTROL_PLANE_MODULES.filter(
    (name) => !controlPlaneModSource.includes(`${name}::boundary()`),
  );
  checks.push({
    id: 'orchestration_control_plane_boundary_registry_complete',
    ok: missingBoundaryRows.length === 0,
    detail:
      missingBoundaryRows.length === 0
        ? 'control-plane boundary registry includes every required subdomain'
        : `missing subdomain boundary registration rows: ${missingBoundaryRows.join(', ')}`,
  });

  const allSurfaceRs = walkFiles(resolve('surface/orchestration/src')).filter(
    (file) => extname(file) === '.rs',
  );
  const forbiddenAuthorityHits: string[] = [];
  for (const file of allSurfaceRs) {
    const normalized = rel(file);
    if (normalized.endsWith('/control_plane/mod.rs')) continue;
    const source = readFileSync(file, 'utf8');
    for (const token of FORBIDDEN_AUTHORITY_TOKENS) {
      if (source.includes(token)) {
        forbiddenAuthorityHits.push(`${normalized} -> ${token}`);
      }
    }
  }
  checks.push({
    id: 'orchestration_surface_forbidden_authority_tokens_absent',
    ok: forbiddenAuthorityHits.length === 0,
    detail:
      forbiddenAuthorityHits.length === 0
        ? 'no forbidden authority tokens found outside control_plane/mod.rs'
        : `forbidden authority token hits: ${forbiddenAuthorityHits.join(' | ')}`,
  });

  const clientSourceFiles = walkFiles(resolve('client')).filter((file) =>
    ['.ts', '.tsx', '.rs'].includes(extname(file)),
  );
  const clientDecompositionLeaks: string[] = [];
  const forbiddenClientTokens = [
    'decomposition_planning',
    'workflow_graph_dependency',
    'recovery_escalation',
    'result_shaping_packaging',
  ];
  for (const file of clientSourceFiles) {
    const source = readFileSync(file, 'utf8');
    const hits = forbiddenClientTokens.filter((token) => source.includes(token));
    if (hits.length > 0) {
      clientDecompositionLeaks.push(`${rel(file)} -> ${hits.join(',')}`);
    }
  }
  checks.push({
    id: 'shell_surface_has_no_control_plane_subdomain_authority_tokens',
    ok: clientDecompositionLeaks.length === 0,
    detail:
      clientDecompositionLeaks.length === 0
        ? 'client shell sources do not declare control-plane subdomain authority tokens'
        : `client shell authority leakage candidates: ${clientDecompositionLeaks.join(' | ')}`,
  });

  const gatewayFiles = walkFiles(resolve('adapters')).filter((file) =>
    ['.ts', '.tsx', '.rs'].includes(extname(file)),
  );
  const gatewayAdmissionTruthHits: string[] = [];
  const forbiddenGatewayTokens = ['execution_admission_truth', 'scheduler_truth'];
  for (const file of gatewayFiles) {
    const source = readFileSync(file, 'utf8');
    const hits = forbiddenGatewayTokens.filter((token) => source.includes(token));
    if (hits.length > 0) {
      gatewayAdmissionTruthHits.push(`${rel(file)} -> ${hits.join(',')}`);
    }
  }
  checks.push({
    id: 'gateway_surface_has_no_scheduler_or_admission_truth_tokens',
    ok: gatewayAdmissionTruthHits.length === 0,
    detail:
      gatewayAdmissionTruthHits.length === 0
        ? 'gateway sources do not own scheduler/admission truth tokens'
        : `gateway authority leakage candidates: ${gatewayAdmissionTruthHits.join(' | ')}`,
  });

  const ok = checks.every((row) => row.ok);
  const payload = {
    ok,
    strict: args.strict,
    checks,
    generated_at: new Date().toISOString(),
  };

  const md = [
    '# ORCHESTRATION BOUNDARY CONTRACT GUARD',
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

  ensureDirFor(args.outJson);
  ensureDirFor(args.outMd);
  writeFileSync(args.outJson, JSON.stringify(payload, null, 2));
  writeFileSync(args.outMd, md);
  console.log(JSON.stringify(payload, null, 2));

  if (args.strict && !ok) return 1;
  return 0;
}

process.exit(run());
