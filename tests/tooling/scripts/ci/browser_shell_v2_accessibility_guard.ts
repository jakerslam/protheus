#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_COMPONENT = 'shell/browser-v2/BrowserShellV2.svelte';
const DEFAULT_OUT_JSON = 'core/local/artifacts/browser_shell_v2_accessibility_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/BROWSER_SHELL_V2_ACCESSIBILITY_GUARD_CURRENT.md';

type Violation = {
  kind: string;
  path: string;
  detail: string;
};

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function read(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function countMatches(content: string, pattern: RegExp): number {
  return Array.from(content.matchAll(pattern)).length;
}

function validate(componentPath: string, includeControlledViolation: boolean): Violation[] {
  const violations: Violation[] = [];
  const component = `${read(componentPath)}${includeControlledViolation ? '\n<button>bad</button>\n<input>\n' : ''}`;
  const requiredAriaLabels = [
    'Browser Shell V2',
    'Runtime status',
    'Selected session',
    'Agent selector',
    'Session selector',
    'Message window',
    'Lazy message detail',
    'Gateway event projection',
    'Bounded Gateway search',
    'Gateway issue evaluation request',
    'Gateway approval decision request',
    'Gateway selection requests',
    'Gateway audit receipts',
    'Shell input',
  ];
  for (const label of requiredAriaLabels) {
    if (!component.includes(`aria-label="${label}"`)) {
      push(violations, 'missing_aria_label', componentPath, `Missing aria-label "${label}".`);
    }
  }
  const buttonCount = countMatches(component, /<button\b/g);
  const typedButtonCount = countMatches(component, /<button\b[^>]*\btype="/g);
  if (typedButtonCount !== buttonCount) {
    push(violations, 'button_type_missing', componentPath, `All buttons need explicit type attributes; found ${typedButtonCount}/${buttonCount}.`);
  }
  const inputLikeCount = countMatches(component, /<(input|select)\b/g);
  const ariaInputLikeCount = countMatches(component, /<(input|select)\b[^>]*\baria-label="/g);
  if (ariaInputLikeCount !== inputLikeCount) {
    push(violations, 'input_label_missing', componentPath, `All input/select controls need aria-labels; found ${ariaInputLikeCount}/${inputLikeCount}.`);
  }
  if (!component.includes('disabled={disabled')) {
    push(violations, 'missing_disabled_state', componentPath, 'Interactive controls must reflect Shell Socket disabled/loading state.');
  }
  if (!component.includes('<main class="browser-shell-v2"')) {
    push(violations, 'missing_main_landmark', componentPath, 'Browser V2 must expose a main landmark.');
  }
  return violations;
}

function markdown(report: any): string {
  const lines = [
    '# Browser Shell V2 Accessibility Guard',
    '',
    `ok: ${report.ok}`,
    `revision: ${report.revision}`,
    '',
    '## Violations',
  ];
  if (report.violations.length === 0) lines.push('- none');
  for (const violation of report.violations as Violation[]) {
    lines.push(`- ${violation.kind}: ${violation.path} - ${violation.detail}`);
  }
  return `${lines.join('\n')}\n`;
}

const argv = process.argv.slice(2);
const componentPath = cleanText(readFlag(argv, 'component') || DEFAULT_COMPONENT, 600);
const strict = parseBool(readFlag(argv, 'strict'), true);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const outJson = cleanText(readFlag(argv, 'out-json') || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const violations = validate(componentPath, includeControlledViolation);
const report = {
  ok: violations.length === 0,
  type: 'browser_shell_v2_accessibility_guard',
  revision: currentRevision(ROOT),
  controlled_violation: includeControlledViolation,
  component_path: componentPath,
  violations,
};

writeTextArtifact(outMarkdown, markdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
