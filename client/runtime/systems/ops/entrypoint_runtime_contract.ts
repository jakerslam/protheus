#!/usr/bin/env node
'use strict';

import path from 'node:path';
import { clean, currentRevision, emitContractResult, normalizePath, parseCommonArgs, pathExists, readJson } from './contract_runtime_common.ts';

type Policy = {
  version?: string;
  enabled?: boolean;
  required_bins?: Record<string, string>;
  paths: {
    package_json_path: string;
    bin_dir: string;
    bootstrap_path: string;
    latest_path: string;
    receipts_path: string;
  };
};

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

function normalizeBinTarget(value: unknown): string {
  return normalizePath(String(value || '').replace(/^\.\//, ''));
}

function run(argv: string[]): number {
  const args = parseCommonArgs(argv, {
    command: argv[0] || 'check',
    policy: 'client/runtime/config/entrypoint_runtime_contract_policy.json',
  });
  const policy = readJson<Policy>(args.policy);
  const pkg = readJson<{ bin?: Record<string, string> }>(policy.paths.package_json_path);
  const requiredBins = Object.entries(policy.required_bins || {});
  const checks: CheckRow[] = [
    {
      id: 'policy_enabled',
      ok: policy.enabled !== false,
      detail: `enabled=${policy.enabled !== false}`,
    },
    {
      id: 'bin_dir_present',
      ok: pathExists(policy.paths.bin_dir),
      detail: policy.paths.bin_dir,
    },
    {
      id: 'bootstrap_present',
      ok: pathExists(policy.paths.bootstrap_path),
      detail: policy.paths.bootstrap_path,
    },
    ...requiredBins.flatMap(([binName, expectedPath]) => {
      const packageTarget = normalizeBinTarget(pkg.bin?.[binName]);
      const normalizedExpected = normalizeBinTarget(expectedPath);
      const resolvedExpected = normalizePath(path.join(policy.paths.bin_dir, path.basename(normalizedExpected)));
      return [
        {
          id: `package_bin:${binName}`,
          ok: packageTarget === normalizedExpected,
          detail: packageTarget || 'missing',
        },
        {
          id: `bin_target_exists:${binName}`,
          ok: pathExists(normalizedExpected),
          detail: normalizedExpected,
        },
        {
          id: `bin_dir_wrapper:${binName}`,
          ok: pathExists(resolvedExpected),
          detail: resolvedExpected,
        },
      ];
    }),
  ];

  const payload = {
    ok: checks.every((row) => row.ok),
    type: 'entrypoint_runtime_contract',
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    inputs: {
      command: args.command,
      policy_path: args.policy,
      strict: args.strict,
    },
    summary: {
      required_bin_count: requiredBins.length,
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
