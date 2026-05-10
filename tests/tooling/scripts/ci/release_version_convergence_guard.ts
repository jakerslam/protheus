#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/release_gates/policies/release_version_convergence_policy.json';
const violations: any[] = [];
const release = fs.readFileSync(path.join(ROOT, '.github/workflows/release.yml'), 'utf8');
const bump = fs.readFileSync(path.join(ROOT, '.github/workflows/version-bump.yml'), 'utf8');
if (!release.includes('softprops/action-gh-release')) violations.push({ kind: 'release_workflow_missing_github_release_publish' });
if (!release.includes('release-windows-prebuilt')) violations.push({ kind: 'release_workflow_missing_windows_prebuilt_dependency' });
if (!bump.includes('chore(release):')) violations.push({ kind: 'version_bump_missing_conventional_release_commit' });
if (!release.includes('release_ready')) violations.push({ kind: 'release_workflow_missing_semver_readiness' });
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'release_version_convergence_guard', generated_at: new Date().toISOString(), policy_path: policyPath, violations };
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/release_version_convergence_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
