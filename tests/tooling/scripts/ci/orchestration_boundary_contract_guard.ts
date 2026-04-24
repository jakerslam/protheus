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

function isCanonicalRelativePath(value: string): boolean {
  if (!value) return false;
  if (value.startsWith('/') || value.startsWith('\\')) return false;
  if (value.includes('..') || value.includes('\\') || value.includes('//')) return false;
  return /^[A-Za-z0-9._/\-]+$/.test(value);
}

function hasCaseInsensitiveSuffix(value: string, suffix: string): boolean {
  return value.toLowerCase().endsWith(suffix.toLowerCase());
}

function isCanonicalToken(value: string): boolean {
  return /^[a-z0-9][a-z0-9_]*$/.test(value);
}

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

function safeIsFile(filePath: string): boolean {
  try {
    return statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function safeIsDirectory(dirPath: string): boolean {
  try {
    return statSync(dirPath).isDirectory();
  } catch {
    return false;
  }
}

function run(): number {
  const args = parseArgs(process.argv);
  const checks: Check[] = [];
  const outJsonCanonical = isCanonicalRelativePath(args.outJson);
  const outMdCanonical = isCanonicalRelativePath(args.outMd);
  checks.push({
    id: 'orchestration_boundary_out_json_path_canonical_contract',
    ok: outJsonCanonical,
    detail: args.outJson,
  });
  checks.push({
    id: 'orchestration_boundary_out_markdown_path_canonical_contract',
    ok: outMdCanonical,
    detail: args.outMd,
  });
  checks.push({
    id: 'orchestration_boundary_out_json_current_suffix_contract',
    ok: hasCaseInsensitiveSuffix(args.outJson, '_current.json'),
    detail: args.outJson,
  });
  checks.push({
    id: 'orchestration_boundary_out_markdown_current_suffix_contract',
    ok: hasCaseInsensitiveSuffix(args.outMd, '_current.md'),
    detail: args.outMd,
  });
  checks.push({
    id: 'orchestration_boundary_output_paths_distinct_contract',
    ok: args.outJson !== args.outMd,
    detail: `${args.outJson}|${args.outMd}`,
  });

  const controlPlaneModPath = resolve('surface/orchestration/src/control_plane/mod.rs');
  const controlPlaneModPathCanonical = rel(controlPlaneModPath);
  const controlPlaneModExists = safeIsFile(controlPlaneModPath);
  const controlPlaneModSource = controlPlaneModExists
    ? readFileSync(controlPlaneModPath, 'utf8')
    : '';
  checks.push({
    id: 'orchestration_boundary_control_plane_mod_exists_contract',
    ok: controlPlaneModExists,
    detail: controlPlaneModPathCanonical,
  });
  checks.push({
    id: 'orchestration_boundary_control_plane_mod_path_canonical_contract',
    ok: isCanonicalRelativePath(controlPlaneModPathCanonical),
    detail: controlPlaneModPathCanonical,
  });

  const requiredModsUniqueCount = new Set(REQUIRED_CONTROL_PLANE_MODULES).size;
  checks.push({
    id: 'orchestration_boundary_required_modules_nonempty_unique_contract',
    ok: REQUIRED_CONTROL_PLANE_MODULES.length > 0
      && requiredModsUniqueCount === REQUIRED_CONTROL_PLANE_MODULES.length,
    detail: `count=${REQUIRED_CONTROL_PLANE_MODULES.length};unique=${requiredModsUniqueCount}`,
  });
  checks.push({
    id: 'orchestration_boundary_required_modules_token_contract',
    ok: REQUIRED_CONTROL_PLANE_MODULES.every((name) => isCanonicalToken(name)),
    detail: REQUIRED_CONTROL_PLANE_MODULES.join(','),
  });

  const forbiddenTokensUniqueCount = new Set(FORBIDDEN_AUTHORITY_TOKENS).size;
  checks.push({
    id: 'orchestration_boundary_forbidden_authority_tokens_nonempty_unique_contract',
    ok: FORBIDDEN_AUTHORITY_TOKENS.length > 0
      && forbiddenTokensUniqueCount === FORBIDDEN_AUTHORITY_TOKENS.length,
    detail: `count=${FORBIDDEN_AUTHORITY_TOKENS.length};unique=${forbiddenTokensUniqueCount}`,
  });
  checks.push({
    id: 'orchestration_boundary_forbidden_authority_tokens_token_contract',
    ok: FORBIDDEN_AUTHORITY_TOKENS.every((token) => isCanonicalToken(token)),
    detail: FORBIDDEN_AUTHORITY_TOKENS.join(','),
  });

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
  const boundaryRowCount = REQUIRED_CONTROL_PLANE_MODULES.reduce(
    (acc, name) => acc + (controlPlaneModSource.includes(`${name}::boundary()`) ? 1 : 0),
    0,
  );
  checks.push({
    id: 'orchestration_control_plane_boundary_registry_complete',
    ok: missingBoundaryRows.length === 0,
    detail:
      missingBoundaryRows.length === 0
        ? 'control-plane boundary registry includes every required subdomain'
        : `missing subdomain boundary registration rows: ${missingBoundaryRows.join(', ')}`,
  });
  checks.push({
    id: 'orchestration_boundary_registry_row_count_contract',
    ok: boundaryRowCount === REQUIRED_CONTROL_PLANE_MODULES.length,
    detail: `rows=${boundaryRowCount};required=${REQUIRED_CONTROL_PLANE_MODULES.length}`,
  });

  const surfaceRoot = resolve('surface/orchestration/src');
  const clientRoot = resolve('client');
  const adaptersRoot = resolve('adapters');
  const surfaceRootExists = safeIsDirectory(surfaceRoot);
  const clientRootExists = safeIsDirectory(clientRoot);
  const adaptersRootExists = safeIsDirectory(adaptersRoot);
  checks.push({
    id: 'orchestration_boundary_surface_source_root_exists_contract',
    ok: surfaceRootExists,
    detail: rel(surfaceRoot),
  });
  checks.push({
    id: 'orchestration_boundary_client_source_root_exists_contract',
    ok: clientRootExists,
    detail: rel(clientRoot),
  });
  checks.push({
    id: 'orchestration_boundary_adapters_source_root_exists_contract',
    ok: adaptersRootExists,
    detail: rel(adaptersRoot),
  });

  const allSurfaceRs = walkFiles(surfaceRoot).filter(
    (file) => extname(file) === '.rs',
  );
  const allSurfaceRsRel = allSurfaceRs.map((file) => rel(file));
  const allSurfaceRsSorted = allSurfaceRsRel.every(
    (file, index) => index === 0 || file.localeCompare(allSurfaceRsRel[index - 1]) >= 0,
  );
  const allSurfaceRsUnique = new Set(allSurfaceRsRel).size === allSurfaceRsRel.length;
  checks.push({
    id: 'orchestration_boundary_surface_rs_scan_nonempty_contract',
    ok: allSurfaceRsRel.length > 0,
    detail: `count=${allSurfaceRsRel.length}`,
  });
  checks.push({
    id: 'orchestration_boundary_surface_rs_scan_sorted_unique_contract',
    ok: allSurfaceRsSorted && allSurfaceRsUnique,
    detail: `count=${allSurfaceRsRel.length};unique=${new Set(allSurfaceRsRel).size}`,
  });
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
  const forbiddenAuthorityHitShapeValid = forbiddenAuthorityHits.every((row) => {
    const [filePart, tokenPart] = row.split(' -> ').map((part) => (part || '').trim());
    return isCanonicalRelativePath(filePart) && isCanonicalToken(tokenPart);
  });
  checks.push({
    id: 'orchestration_surface_forbidden_authority_tokens_absent',
    ok: forbiddenAuthorityHits.length === 0,
    detail:
      forbiddenAuthorityHits.length === 0
        ? 'no forbidden authority tokens found outside control_plane/mod.rs'
        : `forbidden authority token hits: ${forbiddenAuthorityHits.join(' | ')}`,
  });
  checks.push({
    id: 'orchestration_boundary_forbidden_authority_hit_shape_contract',
    ok: forbiddenAuthorityHitShapeValid,
    detail: `hits=${forbiddenAuthorityHits.length}`,
  });

  const clientSourceFiles = walkFiles(clientRoot).filter((file) =>
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
  const clientLeakShapeValid = clientDecompositionLeaks.every((row) => {
    const [filePart, hitPart] = row.split(' -> ').map((part) => (part || '').trim());
    if (!isCanonicalRelativePath(filePart) || !hitPart) return false;
    return hitPart.split(',').every((token) => isCanonicalToken(token.trim()));
  });
  checks.push({
    id: 'shell_surface_has_no_control_plane_subdomain_authority_tokens',
    ok: clientDecompositionLeaks.length === 0,
    detail:
      clientDecompositionLeaks.length === 0
        ? 'client shell sources do not declare control-plane subdomain authority tokens'
        : `client shell authority leakage candidates: ${clientDecompositionLeaks.join(' | ')}`,
  });
  checks.push({
    id: 'orchestration_boundary_client_leak_shape_contract',
    ok: clientLeakShapeValid,
    detail: `hits=${clientDecompositionLeaks.length}`,
  });

  const gatewayFiles = walkFiles(adaptersRoot).filter((file) =>
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
  const gatewayLeakShapeValid = gatewayAdmissionTruthHits.every((row) => {
    const [filePart, hitPart] = row.split(' -> ').map((part) => (part || '').trim());
    if (!isCanonicalRelativePath(filePart) || !hitPart) return false;
    return hitPart.split(',').every((token) => isCanonicalToken(token.trim()));
  });
  checks.push({
    id: 'gateway_surface_has_no_scheduler_or_admission_truth_tokens',
    ok: gatewayAdmissionTruthHits.length === 0,
    detail:
      gatewayAdmissionTruthHits.length === 0
        ? 'gateway sources do not own scheduler/admission truth tokens'
        : `gateway authority leakage candidates: ${gatewayAdmissionTruthHits.join(' | ')}`,
  });
  checks.push({
    id: 'orchestration_boundary_gateway_leak_shape_contract',
    ok: gatewayLeakShapeValid,
    detail: `hits=${gatewayAdmissionTruthHits.length}`,
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
