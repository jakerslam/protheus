#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { execSync } from 'node:child_process';

const STRICT = process.argv.includes('--strict=1');
const REQUIRE_ROI_ARTIFACT = ['1', 'true', 'yes', 'on'].includes(
  String(process.env.INFRING_REQUIRE_ROI_ARTIFACT || '')
    .trim()
    .toLowerCase()
);
const ARTIFACTS_CANDIDATES = [resolve('core/local/artifacts'), resolve('artifacts')];
let FILE_LIST_CACHE = null;

function fail(msg) {
  console.error(msg);
  process.exit(1);
}

function parseJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function resolveArtifactsDir() {
  const found = ARTIFACTS_CANDIDATES.find((dir) => existsSync(dir));
  if (!found) {
    fail(
      `dod_policy_gate: missing artifacts dir (checked: ${ARTIFACTS_CANDIDATES
        .map((dir) => `"${dir}"`)
        .join(', ')})`,
    );
  }
  return found;
}

function latestRoiArtifact() {
  const artifactsDir = resolveArtifactsDir();
  const files = readdirSync(artifactsDir)
    .filter((name) => /^roi_top100_execution_\d{4}-\d{2}-\d{2}\.json$/.test(name))
    .sort();
  if (files.length === 0) {
    if (REQUIRE_ROI_ARTIFACT) {
      fail(`dod_policy_gate: missing ${artifactsDir}/roi_top100_execution_*.json`);
    }
    return null;
  }
  return resolve(artifactsDir, files[files.length - 1]);
}

function listWorkspaceFiles() {
  if (Array.isArray(FILE_LIST_CACHE)) {
    return FILE_LIST_CACHE;
  }
  const commands = ['rg --files .', 'find . -type f'];
  for (const command of commands) {
    try {
      const out = execSync(command, {
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      const files = out
        .split('\n')
        .map((line) => line.trim())
        .filter(Boolean)
        .map((line) => (line.startsWith('./') ? line.slice(2) : line));
      if (files.length > 0) {
        FILE_LIST_CACHE = files;
        return files;
      }
    } catch {
      continue;
    }
  }
  FILE_LIST_CACHE = [];
  return FILE_LIST_CACHE;
}

function globToRegex(pattern) {
  const escaped = pattern
    .replace(/[.+^${}()|[\]\\]/g, '\\$&')
    .replace(/\*/g, '.*')
    .replace(/\?/g, '.');
  return new RegExp(`^${escaped}$`);
}

function globHasMatch(pattern) {
  try {
    const out = execSync(`rg --files -g "${pattern}" . | head -n 1`, {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    }).trim();
    return out.length > 0;
  } catch {
    const normalized = String(pattern || '').trim().replace(/^\.\//, '');
    if (!normalized) return false;
    const regex = globToRegex(normalized);
    return listWorkspaceFiles().some((file) => regex.test(file));
  }
}

function evidencePathCandidates(evidence) {
  const candidates = [evidence];
  const rewriteCandidates = [evidence];
  if (evidence.startsWith('scripts/')) {
    rewriteCandidates.push(evidence.replace(/^scripts\//, 'tests/tooling/scripts/'));
  }
  for (const candidate of rewriteCandidates) {
    candidates.push(candidate);
    if (/\.(mjs|cjs|js)$/i.test(candidate)) {
      candidates.push(candidate.replace(/\.(mjs|cjs|js)$/i, '.ts'));
    }
  }
  return [...new Set(candidates)];
}

function evidenceExists(evidence) {
  if (!evidence || typeof evidence !== 'string') return false;
  const candidates = evidencePathCandidates(evidence);
  return candidates.some((candidate) => {
    if (candidate.includes('*')) return globHasMatch(candidate);
    return existsSync(resolve(candidate));
  });
}

function main() {
  const roiPath = latestRoiArtifact();
  if (!roiPath) {
    const summary = {
      ok: true,
      type: 'dod_policy_gate',
      strict: STRICT,
      source: null,
      implemented_count: 0,
      validated_count: 0,
      findings_count: 0,
      findings: [],
      skipped: true,
      skip_reason: 'roi_artifact_missing_in_checkout',
    };
    console.log(JSON.stringify(summary, null, 2));
    return;
  }
  const payload = parseJson(roiPath);
  const implemented = Array.isArray(payload.implemented) ? payload.implemented : [];
  const validated = Array.isArray(payload.validated) ? payload.validated : [];

  const findings = [];

  implemented.forEach((item, idx) => {
    if (!item?.title || typeof item.title !== 'string') {
      findings.push(`implemented[${idx}] missing title`);
    }
    if (!item?.evidence || typeof item.evidence !== 'string') {
      findings.push(`implemented[${idx}] missing evidence`);
      return;
    }
    if (!evidenceExists(item.evidence)) {
      findings.push(`implemented[${idx}] evidence_not_found: ${item.evidence}`);
    }
  });

  validated.forEach((item, idx) => {
    if (item?.result !== 'existing-coverage-validated') {
      findings.push(
        `validated[${idx}] invalid_result: expected existing-coverage-validated, got ${item?.result ?? 'null'}`,
      );
    }
    if ((item?.status ?? '').toLowerCase() === 'done') {
      findings.push(`validated[${idx}] invalid_status_done_for_regression_only: ${item?.id ?? 'unknown'}`);
    }
  });

  const summary = {
    ok: findings.length === 0,
    type: 'dod_policy_gate',
    strict: STRICT,
    source: roiPath,
    implemented_count: implemented.length,
    validated_count: validated.length,
    findings_count: findings.length,
    findings,
  };

  console.log(JSON.stringify(summary, null, 2));

  if (STRICT && findings.length > 0) {
    process.exit(2);
  }
}

main();
