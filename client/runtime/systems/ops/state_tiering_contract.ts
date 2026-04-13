#!/usr/bin/env node
'use strict';

import { currentRevision, emitContractResult, parseCommonArgs, pathExists, readJson } from './contract_runtime_common.ts';

type PolicyEntry = {
  id: string;
  authority?: string;
  paths?: string[];
  read_contract?: string;
  contract?: string;
};

type Policy = {
  version?: string;
  enabled?: boolean;
  hot_runtime?: PolicyEntry[];
  audit_mirror?: PolicyEntry[];
  paths: {
    latest_path: string;
    receipts_path: string;
  };
};

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

function unique(values: string[]): boolean {
  return new Set(values).size === values.length;
}

function hotPathAllowed(value: string): boolean {
  return value.startsWith('local/state/') || value === 'core/layer0/memory' || value.startsWith('core/layer0/memory/');
}

function auditPathAllowed(value: string): boolean {
  return value.startsWith('local/state/');
}

function run(argv: string[]): number {
  const args = parseCommonArgs(argv, {
    command: argv[0] || 'check',
    policy: 'client/runtime/config/state_tier_manifest.json',
  });
  const policy = readJson<Policy>(args.policy);
  const hotRuntime = Array.isArray(policy.hot_runtime) ? policy.hot_runtime : [];
  const auditMirror = Array.isArray(policy.audit_mirror) ? policy.audit_mirror : [];
  const checks: CheckRow[] = [
    {
      id: 'policy_enabled',
      ok: policy.enabled !== false,
      detail: `enabled=${policy.enabled !== false}`,
    },
    {
      id: 'hot_runtime_nonempty',
      ok: hotRuntime.length > 0,
      detail: `count=${hotRuntime.length}`,
    },
    {
      id: 'audit_mirror_nonempty',
      ok: auditMirror.length > 0,
      detail: `count=${auditMirror.length}`,
    },
    {
      id: 'hot_runtime_ids_unique',
      ok: unique(hotRuntime.map((row) => row.id)),
      detail: `count=${hotRuntime.length}`,
    },
    {
      id: 'audit_mirror_ids_unique',
      ok: unique(auditMirror.map((row) => row.id)),
      detail: `count=${auditMirror.length}`,
    },
    ...hotRuntime.flatMap((row) => [
      {
        id: `hot_authority:${row.id}`,
        ok: typeof row.authority === 'string' && pathExists(row.authority),
        detail: row.authority || 'missing',
      },
      {
        id: `hot_contract:${row.id}`,
        ok: typeof row.read_contract === 'string' && row.read_contract.length > 0,
        detail: row.read_contract || 'missing',
      },
      {
        id: `hot_paths:${row.id}`,
        ok: Array.isArray(row.paths) && row.paths.length > 0 && row.paths.every(hotPathAllowed),
        detail: Array.isArray(row.paths) ? row.paths.join(', ') : 'missing',
      },
    ]),
    ...auditMirror.flatMap((row) => [
      {
        id: `audit_contract:${row.id}`,
        ok: typeof row.contract === 'string' && row.contract.length > 0,
        detail: row.contract || 'missing',
      },
      {
        id: `audit_paths:${row.id}`,
        ok: Array.isArray(row.paths) && row.paths.length > 0 && row.paths.every(auditPathAllowed),
        detail: Array.isArray(row.paths) ? row.paths.join(', ') : 'missing',
      },
    ]),
  ];

  const payload = {
    ok: checks.every((row) => row.ok),
    type: 'state_tiering_contract',
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    inputs: {
      command: args.command,
      policy_path: args.policy,
      strict: args.strict,
    },
    summary: {
      hot_runtime_count: hotRuntime.length,
      audit_mirror_count: auditMirror.length,
      failure_count: checks.filter((row) => !row.ok).length,
      pass: checks.every((row) => row.ok),
    },
    checks,
  };

  return emitContractResult(payload, {
    strict: args.strict,
    latestPath: policy.paths.latest_path,
    receiptsPath: policy.paths.receipts_path,
  });
}

process.exit(run(process.argv.slice(2)));
