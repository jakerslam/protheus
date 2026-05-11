#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (memory-runtime CodeQL security closure guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyRelPath = 'validation/conformance/contracts/memory_runtime_security_closure_policy.json';
const policyPath = path.join(root, policyRelPath);
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const artifactRelPath =
  policy.release_artifact || 'core/local/artifacts/memory_runtime_security_closure_guard_current.json';
const violations = [];
const scannedFiles = new Set();

function rel(file) {
  return path.relative(root, file).replace(/\\/g, '/');
}

function readSource(relativePath) {
  const full = path.join(root, relativePath);
  if (!fs.existsSync(full)) return '';
  scannedFiles.add(relativePath);
  return fs.readFileSync(full, 'utf8');
}

function walk(dir) {
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, ent.name);
    if (ent.isDirectory()) out.push(...walk(full));
    else if (ent.isFile()) out.push(full);
  }
  return out;
}

function makeRegExp(row) {
  return new RegExp(row.pattern, row.flags || '');
}

function baseViolation(check, reason, extra = {}) {
  return {
    id: check.id,
    path: check.path,
    reason,
    code_scanning_rule_id: check.code_scanning_rule_id || null,
    github_alert_ids: check.github_alert_ids || [],
    ...extra,
  };
}

for (const check of policy.required_checks || []) {
  const src = readSource(check.path);
  if (!src) {
    violations.push(baseViolation(check, 'missing_file'));
    continue;
  }
  if (check.must_contain && !src.includes(check.must_contain)) {
    violations.push(baseViolation(check, 'missing_required_token', { token: check.must_contain }));
  }
  for (const token of check.must_contain_all || []) {
    if (!src.includes(token)) {
      violations.push(baseViolation(check, 'missing_required_token', { token }));
    }
  }
  if (check.must_not_contain && src.includes(check.must_not_contain)) {
    violations.push(baseViolation(check, 'forbidden_token_present', { token: check.must_not_contain }));
  }
  for (const token of check.must_not_contain_all || []) {
    if (src.includes(token)) {
      violations.push(baseViolation(check, 'forbidden_token_present', { token }));
    }
  }
  for (const pattern of check.must_not_match || []) {
    if (makeRegExp(pattern).test(src)) {
      violations.push(
        baseViolation(check, 'forbidden_pattern_present', {
          pattern: pattern.pattern,
          detail: pattern.reason || null,
        }),
      );
    }
  }
  for (const pattern of check.must_match || []) {
    if (!makeRegExp(pattern).test(src)) {
      violations.push(
        baseViolation(check, 'missing_required_pattern', {
          pattern: pattern.pattern,
          detail: pattern.reason || null,
        }),
      );
    }
  }
}

for (const pattern of policy.global_forbidden_patterns || []) {
  const scanRoot = path.join(root, pattern.root);
  const extensions = pattern.include_extensions || ['.rs'];
  const re = makeRegExp(pattern);
  for (const file of walk(scanRoot)) {
    if (!extensions.includes(path.extname(file))) continue;
    const relativePath = rel(file);
    const src = readSource(relativePath);
    if (!re.test(src)) continue;
    violations.push({
      id: pattern.id,
      path: relativePath,
      reason: 'global_forbidden_pattern_present',
      pattern: pattern.pattern,
      detail: pattern.reason || null,
      code_scanning_rule_id: pattern.code_scanning_rule_id || null,
      github_alert_ids: pattern.github_alert_ids || [],
    });
  }
}

const generatedAt = new Date().toISOString();
const traceId = `validation:${generatedAt}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'memory_runtime_security_closure_guard',
  generated_at: generatedAt,
  policy_path: policyRelPath,
  code_scanning_classes: policy.code_scanning_classes || [],
  covered_code_scanning_rule_ids: Array.from(new Set((policy.code_scanning_classes || []).map((row) => row.rule_id).filter(Boolean))).sort(),
  covered_github_alert_ids: Array.from(new Set((policy.code_scanning_classes || []).flatMap((row) => row.github_alert_ids || []))).sort((a, b) => Number(a) - Number(b)),
  scanned_source_roots: policy.source_roots || [],
  scanned_file_count: scannedFiles.size,
  required_check_count: (policy.required_checks || []).length,
  global_forbidden_pattern_count: (policy.global_forbidden_patterns || []).length,
  violation_count: violations.length,
  violations,
};

fs.mkdirSync(path.join(root, path.dirname(artifactRelPath)), { recursive: true });
fs.writeFileSync(path.join(root, artifactRelPath), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (violations.length) process.exit(1);
