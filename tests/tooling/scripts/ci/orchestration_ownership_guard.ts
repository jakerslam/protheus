#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_POLICY = 'client/runtime/config/orchestration_ownership_policy.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/orchestration_ownership_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/ORCHESTRATION_OWNERSHIP_GUARD_CURRENT.md';

type RequiredDoc = {
  path?: string;
  required_phrases?: string[];
};

type AllowViolationRule = {
  file?: string;
  detail_contains?: string;
  owner?: string;
  ticket?: string;
  expires_at?: string;
};

type Policy = {
  required_docs?: RequiredDoc[];
  client_wrapper_contract?: {
    include_paths?: string[];
    root?: string;
    extensions?: string[];
    required_markers?: string[];
  };
  client_runtime_wrapper_contract?: {
    scan_roots?: string[];
    extensions?: string[];
    surface_script_marker?: string;
    required_markers?: string[];
    required_delegate_markers_any?: string[];
    ignored_paths?: string[];
  };
  surface_script_import_boundary?: {
    scan_root?: string;
    extensions?: string[];
    forbidden_import_prefixes?: string[];
    allow_violations?: AllowViolationRule[];
  };
};

type Violation = {
  check_id: string;
  file: string;
  reason: string;
  detail: string;
};

type Args = {
  strict: boolean;
  policy: string;
  outJson: string;
  outMarkdown: string;
};

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function parseArgs(argv: string[]): Args {
  const strictOut = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: strictOut.strict,
    policy: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    outJson: cleanText(readFlag(argv, 'out-json') || strictOut.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function listFiles(rootPath: string, extensions: string[]): string[] {
  const out: string[] = [];
  if (!fs.existsSync(rootPath)) return out;
  const extSet = new Set(extensions.map((value) => String(value || '').toLowerCase()));
  const stack = [rootPath];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const abs = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(abs);
        continue;
      }
      if (!entry.isFile()) continue;
      const ext = path.extname(entry.name).toLowerCase();
      if (extSet.has(ext)) out.push(abs);
    }
  }
  return out.sort();
}

function parseImportSpecs(source: string): string[] {
  const specs: string[] = [];
  const re = /(?:import\s+[^'"]*from\s+|import\s*\(|require\s*\()\s*['"]([^'"]+)['"]/g;
  let match: RegExpExecArray | null = null;
  while ((match = re.exec(source)) != null) {
    specs.push(String(match[1] || ''));
  }
  return specs;
}

function isRuleExpired(rule: AllowViolationRule): boolean {
  if (!rule.expires_at) return false;
  const ts = Date.parse(`${rule.expires_at}T00:00:00Z`);
  if (!Number.isFinite(ts)) return true;
  return ts < Date.now();
}

function runRequiredDocCheck(policy: Policy): Violation[] {
  const violations: Violation[] = [];
  const requiredDocs = Array.isArray(policy.required_docs) ? policy.required_docs : [];
  for (const item of requiredDocs) {
    const docPath = cleanText(item.path || '', 400);
    if (!docPath) {
      violations.push({
        check_id: 'required_docs',
        file: '(policy)',
        reason: 'missing_required_doc_path',
        detail: JSON.stringify(item),
      });
      continue;
    }
    const abs = path.resolve(ROOT, docPath);
    if (!fs.existsSync(abs)) {
      violations.push({
        check_id: 'required_docs',
        file: docPath,
        reason: 'required_doc_missing',
        detail: 'file_not_found',
      });
      continue;
    }
    const source = fs.readFileSync(abs, 'utf8');
    const phrases = Array.isArray(item.required_phrases) ? item.required_phrases : [];
    for (const phrase of phrases) {
      const normalized = cleanText(phrase, 300);
      if (!normalized) continue;
      if (!source.includes(normalized)) {
        violations.push({
          check_id: 'required_docs',
          file: docPath,
          reason: 'required_doc_phrase_missing',
          detail: normalized,
        });
      }
    }
  }
  return violations;
}

function runClientCognitionWrapperCheck(policy: Policy): Violation[] {
  const violations: Violation[] = [];
  const contract = policy.client_wrapper_contract || {};
  const includePaths = Array.isArray(contract.include_paths)
    ? contract.include_paths.map((value) => cleanText(value, 400)).filter(Boolean)
    : [];
  const root = cleanText(contract.root || '', 400);
  const extensions = Array.isArray(contract.extensions) && contract.extensions.length > 0 ? contract.extensions : ['.ts'];
  const requiredMarkers = Array.isArray(contract.required_markers) ? contract.required_markers : [];
  const files = includePaths.length > 0
    ? includePaths.map((value) => path.resolve(ROOT, value)).filter((absPath) => fs.existsSync(absPath))
    : listFiles(path.resolve(ROOT, root), extensions);
  if (includePaths.length > 0) {
    for (const targetPath of includePaths) {
      const abs = path.resolve(ROOT, targetPath);
      if (!fs.existsSync(abs)) {
        violations.push({
          check_id: 'client_cognition_wrappers',
          file: targetPath,
          reason: 'required_wrapper_missing',
          detail: 'file_not_found',
        });
      }
    }
  }
  if (files.length === 0) {
    violations.push({
      check_id: 'client_cognition_wrappers',
      file: includePaths.length > 0 ? '(policy include_paths)' : root,
      reason: 'wrapper_root_empty',
      detail: 'no_files_found',
    });
    return violations;
  }
  for (const filePath of files) {
    const rp = rel(filePath);
    const source = fs.readFileSync(filePath, 'utf8');
    for (const marker of requiredMarkers) {
      const normalized = cleanText(marker, 300);
      if (!normalized) continue;
      if (!source.includes(normalized)) {
        violations.push({
          check_id: 'client_cognition_wrappers',
          file: rp,
          reason: 'missing_wrapper_marker',
          detail: normalized,
        });
      }
    }
  }
  return violations;
}

function runClientRuntimeWrapperCheck(policy: Policy): Violation[] {
  const violations: Violation[] = [];
  const contract = policy.client_runtime_wrapper_contract || {};
  const roots = Array.isArray(contract.scan_roots) ? contract.scan_roots : [];
  const extensions = Array.isArray(contract.extensions) && contract.extensions.length > 0
    ? contract.extensions
    : ['.ts'];
  const surfaceMarker = cleanText(contract.surface_script_marker || '', 200);
  const requiredMarkers = Array.isArray(contract.required_markers) ? contract.required_markers : [];
  const delegateMarkers = Array.isArray(contract.required_delegate_markers_any)
    ? contract.required_delegate_markers_any
    : [];
  const ignoredPaths = new Set(
    (Array.isArray(contract.ignored_paths) ? contract.ignored_paths : []).map((value) =>
      cleanText(value, 400).replace(/\\/g, '/'),
    ),
  );
  for (const root of roots) {
    const files = listFiles(path.resolve(ROOT, cleanText(root, 400)), extensions);
    for (const filePath of files) {
      const rp = rel(filePath);
      if (ignoredPaths.has(rp)) continue;
      const source = fs.readFileSync(filePath, 'utf8');
      if (!surfaceMarker || !source.includes(surfaceMarker)) continue;

      for (const marker of requiredMarkers) {
        const normalized = cleanText(marker, 300);
        if (!normalized) continue;
        if (!source.includes(normalized)) {
          violations.push({
            check_id: 'client_runtime_wrappers',
            file: rp,
            reason: 'missing_wrapper_marker',
            detail: normalized,
          });
        }
      }

      const hasDelegateMarker = delegateMarkers.some((marker) => source.includes(cleanText(marker, 200)));
      if (!hasDelegateMarker) {
        violations.push({
          check_id: 'client_runtime_wrappers',
          file: rp,
          reason: 'missing_delegate_marker',
          detail: delegateMarkers.join(' | '),
        });
      }
    }
  }
  return violations;
}

function runSurfaceScriptImportBoundaryCheck(policy: Policy): {
  hardViolations: Violation[];
  allowedViolations: Violation[];
  expiredAllowedViolations: Violation[];
  malformedAllowRules: Violation[];
} {
  const contract = policy.surface_script_import_boundary || {};
  const scanRoot = cleanText(contract.scan_root || '', 400);
  if (!scanRoot) {
    return {
      hardViolations: [],
      allowedViolations: [],
      expiredAllowedViolations: [],
      malformedAllowRules: [],
    };
  }
  const extensions = Array.isArray(contract.extensions) && contract.extensions.length > 0
    ? contract.extensions
    : ['.ts'];
  const forbiddenPrefixes = Array.isArray(contract.forbidden_import_prefixes)
    ? contract.forbidden_import_prefixes.map((value) => cleanText(value, 120)).filter(Boolean)
    : [];
  const allowRules = Array.isArray(contract.allow_violations) ? contract.allow_violations : [];

  const malformedAllowRules: Violation[] = [];
  for (const rule of allowRules) {
    if (!rule.file || !rule.detail_contains || !rule.owner || !rule.ticket || !rule.expires_at) {
      malformedAllowRules.push({
        check_id: 'surface_script_import_boundary',
        file: '(policy)',
        reason: 'allowlist_rule_missing_metadata',
        detail: JSON.stringify(rule),
      });
    }
  }

  const discovered: Violation[] = [];
  const files = listFiles(path.resolve(ROOT, scanRoot), extensions);
  for (const filePath of files) {
    const rp = rel(filePath);
    const source = fs.readFileSync(filePath, 'utf8');
    const specs = parseImportSpecs(source);
    for (const specRaw of specs) {
      const spec = cleanText(specRaw, 500).replace(/\\/g, '/');
      const forbidden = forbiddenPrefixes.some((prefix) => spec.includes(prefix));
      if (!forbidden) continue;
      discovered.push({
        check_id: 'surface_script_import_boundary',
        file: rp,
        reason: 'forbidden_import',
        detail: spec,
      });
    }
  }

  const allowedViolations: Violation[] = [];
  const expiredAllowedViolations: Violation[] = [];
  const hardViolations: Violation[] = [];

  for (const violation of discovered) {
    const matched = allowRules.find((rule) =>
      rule.file === violation.file && violation.detail.includes(String(rule.detail_contains || ''))
    );
    if (!matched) {
      hardViolations.push(violation);
      continue;
    }
    if (isRuleExpired(matched)) {
      expiredAllowedViolations.push(violation);
      hardViolations.push({
        check_id: 'surface_script_import_boundary',
        file: violation.file,
        reason: 'allowlist_rule_expired',
        detail: `${violation.reason}:${violation.detail}`,
      });
      continue;
    }
    allowedViolations.push(violation);
  }

  hardViolations.push(...malformedAllowRules);
  return {
    hardViolations,
    allowedViolations,
    expiredAllowedViolations,
    malformedAllowRules,
  };
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Orchestration Ownership Guard');
  lines.push('');
  lines.push(`Generated: ${payload.generated_at}`);
  lines.push(`Revision: ${payload.revision}`);
  lines.push(`Policy: ${payload.inputs.policy_path}`);
  lines.push(`Pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push('');
  lines.push(`- Required doc violations: ${payload.summary.required_doc_violation_count}`);
  lines.push(`- Client cognition wrapper violations: ${payload.summary.client_cognition_wrapper_violation_count}`);
  lines.push(`- Client runtime wrapper violations: ${payload.summary.client_runtime_wrapper_violation_count}`);
  lines.push(`- Surface import hard violations: ${payload.summary.surface_import_hard_violation_count}`);
  lines.push(`- Surface import allowlisted violations: ${payload.summary.surface_import_allowed_violation_count}`);
  lines.push(`- Surface import expired allowlist violations: ${payload.summary.surface_import_expired_allowlist_violation_count}`);
  lines.push(`- Total hard violations: ${payload.summary.hard_violation_count}`);
  lines.push('');
  lines.push('## Hard Violations');
  lines.push('');
  lines.push('| Check | File | Reason | Detail |');
  lines.push('| --- | --- | --- | --- |');
  const hardRows = Array.isArray(payload.violations) ? payload.violations : [];
  if (hardRows.length === 0) {
    lines.push('| (none) | - | - | - |');
  } else {
    for (const row of hardRows.slice(0, 120)) {
      lines.push(
        `| ${String(row.check_id || '')} | ${String(row.file || '')} | ${String(row.reason || '')} | ${String(row.detail || '').slice(0, 180)} |`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const policyPath = path.resolve(ROOT, args.policy);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Policy;

  const requiredDocViolations = runRequiredDocCheck(policy);
  const clientCognitionWrapperViolations = runClientCognitionWrapperCheck(policy);
  const clientRuntimeWrapperViolations = runClientRuntimeWrapperCheck(policy);
  const surfaceImport = runSurfaceScriptImportBoundaryCheck(policy);

  const hardViolations = [
    ...requiredDocViolations,
    ...clientCognitionWrapperViolations,
    ...clientRuntimeWrapperViolations,
    ...surfaceImport.hardViolations,
  ];

  const payload = {
    ok: hardViolations.length === 0,
    type: 'orchestration_ownership_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      strict: args.strict,
      policy_path: rel(policyPath),
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    summary: {
      pass: hardViolations.length === 0,
      required_doc_violation_count: requiredDocViolations.length,
      client_cognition_wrapper_violation_count: clientCognitionWrapperViolations.length,
      client_runtime_wrapper_violation_count: clientRuntimeWrapperViolations.length,
      surface_import_hard_violation_count: surfaceImport.hardViolations.length,
      surface_import_allowed_violation_count: surfaceImport.allowedViolations.length,
      surface_import_expired_allowlist_violation_count: surfaceImport.expiredAllowedViolations.length,
      hard_violation_count: hardViolations.length,
    },
    violations: hardViolations,
    allowed_violations: surfaceImport.allowedViolations,
    expired_allowed_violations: surfaceImport.expiredAllowedViolations,
  };

  writeTextArtifact(path.resolve(ROOT, args.outMarkdown), toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: path.resolve(ROOT, args.outJson),
    strict: args.strict,
    ok: payload.ok,
  });
}

const exitCode = main();
if (exitCode !== 0) process.exit(exitCode);
