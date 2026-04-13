#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { currentRevision, emitContractResult, listTrackedFiles, normalizePath, parseCommonArgs, readJson, writeJsonArtifact } from './contract_runtime_common.ts';

type Policy = {
  version?: string;
  enabled?: boolean;
  strict_default?: boolean;
  scan?: {
    include?: string[];
    ext?: string[];
    forbidden_patterns?: string[];
    allowlist?: string[];
  };
  paths: {
    latest_path: string;
    receipts_path: string;
    rewrite_inventory_path: string;
  };
};

type Finding = {
  file: string;
  line: number;
  pattern: string;
  snippet: string;
};

function isIncluded(filePath: string, include: string[]): boolean {
  return include.some((root) => filePath === root || filePath.startsWith(`${root}/`));
}

function isAllowlisted(filePath: string, allowlist: string[]): boolean {
  return allowlist.some((root) => filePath === root || filePath.startsWith(`${root}/`));
}

function isTestLike(filePath: string): boolean {
  return /(^|\/)(tests?|__tests__|fixtures?)\b/i.test(filePath) || /(test|tests|spec)\./i.test(path.basename(filePath));
}

function shouldScan(filePath: string, include: string[], exts: string[]): boolean {
  if (!isIncluded(filePath, include)) return false;
  if (isTestLike(filePath)) return false;
  if (exts.includes(path.extname(filePath))) return true;
  return include.includes(filePath);
}

function trimRustTests(source: string, filePath: string): string {
  if (!filePath.endsWith('.rs')) return source;
  const marker = source.indexOf('#[cfg(test)]');
  return marker >= 0 ? source.slice(0, marker) : source;
}

function allowedGenericPattern(line: string): boolean {
  return line.includes('/Users/*/') || line.includes('\\\\Users\\\\*\\\\');
}

function collectFindings(files: string[], policy: Policy): Finding[] {
  const include = (policy.scan?.include || []).map(normalizePath);
  const allowlist = (policy.scan?.allowlist || []).map(normalizePath);
  const exts = (policy.scan?.ext || []).map(String);
  const patterns = (policy.scan?.forbidden_patterns || []).map(String).filter(Boolean);
  const findings: Finding[] = [];

  for (const filePath of files) {
    if (!shouldScan(filePath, include, exts)) continue;
    if (isAllowlisted(filePath, allowlist)) continue;
    const abs = path.resolve(filePath);
    if (!fs.existsSync(abs)) continue;
    const source = trimRustTests(fs.readFileSync(abs, 'utf8'), filePath);
    const lines = source.split('\n');
    for (let index = 0; index < lines.length; index += 1) {
      const line = lines[index];
      if (!line || allowedGenericPattern(line)) continue;
      const match = patterns.find((pattern) => line.includes(pattern));
      if (!match) continue;
      findings.push({
        file: filePath,
        line: index + 1,
        pattern: match,
        snippet: line.trim().slice(0, 240),
      });
    }
  }

  return findings;
}

function run(argv: string[]): number {
  const args = parseCommonArgs(argv, {
    command: argv[0] || 'check',
    policy: 'client/runtime/config/relocatable_path_contract_policy.json',
  });
  const policy = readJson<Policy>(args.policy);
  const trackedFiles = listTrackedFiles();
  const findings = collectFindings(trackedFiles, policy);
  writeJsonArtifact(policy.paths.rewrite_inventory_path, {
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    findings,
  });

  const payload = {
    ok: findings.length === 0,
    type: 'relocatable_path_contract',
    generated_at: new Date().toISOString(),
    revision: currentRevision(),
    inputs: {
      command: args.command,
      policy_path: args.policy,
      strict: args.strict,
    },
    summary: {
      scanned_file_count: trackedFiles.length,
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
