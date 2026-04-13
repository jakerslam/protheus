#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { currentRevision, emitContractResult, listTrackedFiles, normalizePath, parseCommonArgs, pathExists, readJson } from './contract_runtime_common.ts';

type Policy = {
  version?: string;
  enabled?: boolean;
  runtime_roots?: string[];
  source_memory_root?: string;
  runtime_memory_roots?: string[];
  source_memory_runtime_like_ext?: string[];
  source_memory_runtime_like_allow_prefixes?: string[];
  source_memory_runtime_like_allow_files?: string[];
  required_runtime_paths?: string[];
  forbidden_source_ext_in_runtime?: string[];
  runtime_ignore_files?: string[];
  runtime_ignore_path_contains?: string[];
  paths: {
    latest_path: string;
    receipts_path: string;
  };
};

type Finding = {
  type: 'runtime_source_file' | 'source_memory_runtime_like_file' | 'missing_required_runtime_path';
  path: string;
  detail: string;
};

function inRoots(filePath: string, roots: string[]): boolean {
  return roots.some((root) => filePath === root || filePath.startsWith(`${root}/`));
}

function run(argv: string[]): number {
  const args = parseCommonArgs(argv, {
    command: argv[0] || 'check',
    policy: 'client/runtime/config/source_runtime_classifier_policy.json',
  });
  const policy = readJson<Policy>(args.policy);
  const runtimeRoots = (policy.runtime_roots || []).map(normalizePath);
  const runtimeIgnoreFiles = new Set((policy.runtime_ignore_files || []).map(String));
  const runtimeIgnoreContains = (policy.runtime_ignore_path_contains || []).map(String);
  const forbiddenRuntimeExt = new Set((policy.forbidden_source_ext_in_runtime || []).map(String));
  const sourceMemoryRoot = normalizePath(policy.source_memory_root || 'client/memory');
  const runtimeLikeExt = new Set((policy.source_memory_runtime_like_ext || []).map(String));
  const sourceAllowPrefixes = (policy.source_memory_runtime_like_allow_prefixes || []).map(normalizePath);
  const sourceAllowFiles = new Set((policy.source_memory_runtime_like_allow_files || []).map(String));
  const requiredRuntimePaths = (policy.required_runtime_paths || []).map(normalizePath);
  const trackedFiles = listTrackedFiles();
  const findings: Finding[] = [];

  for (const requiredPath of requiredRuntimePaths) {
    if (!pathExists(requiredPath)) {
      findings.push({
        type: 'missing_required_runtime_path',
        path: requiredPath,
        detail: 'missing_required_runtime_path',
      });
    }
  }

  for (const filePath of trackedFiles) {
    const ext = path.extname(filePath);
    const base = path.basename(filePath);
    const ignoredRuntimeFile =
      runtimeIgnoreFiles.has(base) || runtimeIgnoreContains.some((needle) => filePath.includes(needle));

    if (inRoots(filePath, runtimeRoots) && !ignoredRuntimeFile && forbiddenRuntimeExt.has(ext)) {
      findings.push({
        type: 'runtime_source_file',
        path: filePath,
        detail: `forbidden_runtime_source_ext:${ext}`,
      });
    }

    if (filePath === sourceMemoryRoot || filePath.startsWith(`${sourceMemoryRoot}/`)) {
      const rel = filePath.slice(sourceMemoryRoot.length).replace(/^\/+/, '');
      const allowed =
        sourceAllowFiles.has(base) || sourceAllowPrefixes.some((prefix) => rel.startsWith(prefix));
      if (!allowed && runtimeLikeExt.has(ext)) {
        findings.push({
          type: 'source_memory_runtime_like_file',
          path: filePath,
          detail: `runtime_like_source_memory_ext:${ext}`,
        });
      }
    }
  }

  const payload = {
    ok: findings.length === 0,
    type: 'source_runtime_classifier_contract',
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    inputs: {
      command: args.command,
      policy_path: args.policy,
      strict: args.strict,
    },
    summary: {
      tracked_file_count: trackedFiles.length,
      finding_count: findings.length,
      pass: findings.length === 0,
    },
    findings,
  };

  return emitContractResult(payload, {
    strict: args.strict,
    latestPath: policy.paths.latest_path,
    receiptsPath: policy.paths.receipts_path,
  });
}

process.exit(run(process.argv.slice(2)));
