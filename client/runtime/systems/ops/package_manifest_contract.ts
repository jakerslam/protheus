#!/usr/bin/env node
'use strict';

import { clean, currentRevision, emitContractResult, parseCommonArgs, pathExists, readJson } from './contract_runtime_common.ts';

type Policy = {
  version?: string;
  enabled?: boolean;
  required_fields?: string[];
  paths: {
    package_json_path: string;
    latest_path: string;
    receipts_path: string;
  };
};

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

function hasValue(value: unknown): boolean {
  if (value == null) return false;
  if (typeof value === 'string') return clean(value).length > 0;
  if (Array.isArray(value)) return value.length > 0;
  if (typeof value === 'object') return Object.keys(value as Record<string, unknown>).length > 0;
  return true;
}

function run(argv: string[]): number {
  const args = parseCommonArgs(argv, {
    command: argv[0] || 'check',
    policy: 'client/runtime/config/package_manifest_contract_policy.json',
  });
  const policy = readJson<Policy>(args.policy);
  const pkg = readJson<Record<string, unknown>>(policy.paths.package_json_path);
  const fields = Array.isArray(policy.required_fields) ? policy.required_fields : [];
  const checks: CheckRow[] = [
    {
      id: 'policy_enabled',
      ok: policy.enabled !== false,
      detail: `enabled=${policy.enabled !== false}`,
    },
    {
      id: 'package_json_present',
      ok: pathExists(policy.paths.package_json_path),
      detail: policy.paths.package_json_path,
    },
    ...fields.map((field) => ({
      id: `required_field:${field}`,
      ok: hasValue(pkg[field]),
      detail: hasValue(pkg[field]) ? 'present' : 'missing_or_empty',
    })),
    {
      id: 'engines_node_present',
      ok: Boolean((pkg.engines as Record<string, unknown> | undefined)?.node),
      detail: Boolean((pkg.engines as Record<string, unknown> | undefined)?.node) ? 'present' : 'missing',
    },
    {
      id: 'package_manager_present',
      ok: typeof pkg.packageManager === 'string' && clean(pkg.packageManager).length > 0,
      detail: typeof pkg.packageManager === 'string' ? clean(pkg.packageManager) : 'missing',
    },
  ];

  const payload = {
    ok: checks.every((row) => row.ok),
    type: 'package_manifest_contract',
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    inputs: {
      command: args.command,
      policy_path: args.policy,
      strict: args.strict,
    },
    summary: {
      required_field_count: fields.length,
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
