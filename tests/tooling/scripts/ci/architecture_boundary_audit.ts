#!/usr/bin/env node
/* eslint-disable no-console */
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

type CheckResult = {
  id: string;
  ok: boolean;
  detail: string;
};

const DEFAULT_OUT_JSON = 'core/local/artifacts/architecture_boundary_audit_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/ARCHITECTURE_BOUNDARY_AUDIT_CURRENT.md';

function argValue(argv: string[], key: string): string | null {
  const prefix = `--${key}=`;
  for (const arg of argv) {
    if (arg.startsWith(prefix)) return arg.slice(prefix.length).trim() || null;
  }
  return null;
}

function parseArgs(argv: string[]) {
  return {
    strict: argv.includes('--strict') || argv.includes('--strict=1'),
    outJson: argValue(argv, 'out-json') || DEFAULT_OUT_JSON,
    outMd: argValue(argv, 'out-md') || DEFAULT_OUT_MD,
  };
}

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

function toMarkdown(rows: CheckResult[]): string {
  const lines: string[] = [];
  lines.push('# Architecture Boundary Audit (Current)');
  lines.push('');
  lines.push(`Generated: ${new Date().toISOString()}`);
  lines.push('');
  lines.push('| Check | Result | Detail |');
  lines.push('| --- | --- | --- |');
  for (const row of rows) {
    lines.push(`| ${row.id} | ${row.ok ? 'pass' : 'fail'} | ${row.detail} |`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));

  const federation = read('core/layer0/nexus/src/federation.rs');
  const coreLib = read('core/layer0/nexus/src/lib.rs');
  const orchestrationLib = read('surface/orchestration/src/lib.rs');
  const orchestrationSeq = read('surface/orchestration/src/sequencing.rs');
  const orchestrationTransient = read('surface/orchestration/src/transient_context.rs');
  const rootSurfaceContract = JSON.parse(read('client/runtime/config/root_surface_contract.json'));
  const repoSurfacePolicy = JSON.parse(read('client/runtime/config/repo_surface_policy.json'));
  const clientSwarmWrapper = read('client/runtime/systems/autonomy/swarm_orchestration_runtime.ts');
  const surfaceSwarmRuntime = read('surface/orchestration/scripts/swarm_orchestration_runtime.ts');

  const checks: CheckResult[] = [
    {
      id: 'core_must_not_depend_on_orchestration_surface',
      ok: !coreLib.includes('infring_orchestration_surface_v1'),
      detail: 'core/layer0/nexus does not import orchestration crate',
    },
    {
      id: 'orchestration_surface_must_not_depend_on_client',
      ok: !orchestrationLib.includes('client::') && !orchestrationLib.includes('client/runtime'),
      detail: 'surface/orchestration has no client-layer dependency',
    },
    {
      id: 'client_core_direct_path_blocked_without_approved_ingress',
      ok: federation.includes('direct_client_core_path_prohibited'),
      detail: 'federation enforces explicit deny for direct client/core paths',
    },
    {
      id: 'core_orchestration_requires_strong_scrambler',
      ok: federation.includes('strong_conduit_scrambler_required'),
      detail: 'core<->orchestration routes require strong scrambler',
    },
    {
      id: 'orchestration_tool_calls_route_to_tool_broker',
      ok: orchestrationSeq.includes('CoreContractCall::ToolBrokerRequest'),
      detail: 'tool-call classification routes through Tool Broker contract',
    },
    {
      id: 'orchestration_transient_state_is_sweepable',
      ok: orchestrationTransient.includes('sweep_expired'),
      detail: 'transient orchestration context supports deterministic sweep',
    },
    {
      id: 'root_surface_contract_allowlists_surface_root',
      ok: Array.isArray(rootSurfaceContract.allowed_root_dirs) &&
        rootSurfaceContract.allowed_root_dirs.includes('surface'),
      detail: 'root surface contract explicitly allowlists surface/',
    },
    {
      id: 'repo_surface_policy_declares_surface_code_root',
      ok: Array.isArray(repoSurfacePolicy.code_roots) && repoSurfacePolicy.code_roots.includes('surface'),
      detail: 'repo surface policy treats surface/ as canonical code root',
    },
    {
      id: 'client_swarm_orchestration_is_wrapper_only',
      ok: clientSwarmWrapper.includes('TypeScript compatibility shim only.') &&
        clientSwarmWrapper.includes('surface/orchestration/scripts/swarm_orchestration_runtime.ts'),
      detail: 'client swarm orchestration entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'orchestration_runtime_lives_under_surface',
      ok: surfaceSwarmRuntime.includes('runSpawn') && surfaceSwarmRuntime.includes('swarm-runtime'),
      detail: 'swarm orchestration coordination implementation is hosted in surface/orchestration',
    },
  ];

  const failures = checks.filter((row) => !row.ok);
  const payload = {
    ok: failures.length === 0,
    type: 'architecture_boundary_audit',
    generated_at: new Date().toISOString(),
    summary: {
      checks: checks.length,
      failures: failures.length,
    },
    checks,
  };

  const outJsonAbs = resolve(args.outJson);
  const outMdAbs = resolve(args.outMd);
  mkdirSync(dirname(outJsonAbs), { recursive: true });
  mkdirSync(dirname(outMdAbs), { recursive: true });
  writeFileSync(outJsonAbs, `${JSON.stringify(payload, null, 2)}\n`);
  writeFileSync(outMdAbs, toMarkdown(checks));

  if (args.strict && failures.length > 0) {
    console.error(JSON.stringify(payload, null, 2));
    process.exit(1);
  }

  console.log(
    JSON.stringify(
      {
        ok: payload.ok,
        type: payload.type,
        out_json: args.outJson,
        out_md: args.outMd,
        summary: payload.summary,
      },
      null,
      2,
    ),
  );
}

main();
