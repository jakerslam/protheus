#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { currentRevision, emitContractResult, parseCommonArgs, pathExists, readJson } from './contract_runtime_common.ts';

type Policy = {
  version?: string;
  enabled?: boolean;
  files?: string[];
  discouraged_terms?: string[];
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

function run(argv: string[]): number {
  const args = parseCommonArgs(argv, {
    command: argv[0] || 'check',
    policy: 'client/runtime/config/legal_language_contract_policy.json',
  });
  const policy = readJson<Policy>(args.policy);
  const files = Array.isArray(policy.files) ? policy.files : [];
  const discouragedTerms = (policy.discouraged_terms || []).map((value) => String(value || '').toLowerCase()).filter(Boolean);
  const checks: CheckRow[] = [
    {
      id: 'policy_enabled',
      ok: policy.enabled !== false,
      detail: `enabled=${policy.enabled !== false}`,
    },
    ...files.map((filePath) => ({
      id: `file_present:${filePath}`,
      ok: pathExists(filePath),
      detail: filePath,
    })),
    ...files.flatMap((filePath) => {
      const abs = path.resolve(filePath);
      const source = fs.existsSync(abs) ? fs.readFileSync(abs, 'utf8').toLowerCase() : '';
      return discouragedTerms.map((term) => ({
        id: `discouraged_term:${path.basename(filePath)}:${term}`,
        ok: !source.includes(term),
        detail: source.includes(term) ? 'found' : 'absent',
      }));
    }),
  ];

  const payload = {
    ok: checks.every((row) => row.ok),
    type: 'legal_language_contract',
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    inputs: {
      command: args.command,
      policy_path: args.policy,
      strict: args.strict,
    },
    summary: {
      file_count: files.length,
      discouraged_term_count: discouragedTerms.length,
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
