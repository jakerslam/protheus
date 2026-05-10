#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_COMPONENT = 'shell/browser-v2/BrowserShellV2.svelte';
const DEFAULT_STYLES = 'shell/browser-v2/browser_shell_v2.css';
const DEFAULT_README = 'shell/browser-v2/README.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/browser_shell_v2_visual_parity_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/BROWSER_SHELL_V2_VISUAL_PARITY_GUARD_CURRENT.md';

type Violation = {
  kind: string;
  path: string;
  detail: string;
};

const REQUIRED_COMPONENT_SURFACES = [
  'browser-shell-v2__topbar',
  'browser-shell-v2__workspace',
  'browser-shell-v2__rail',
  'browser-shell-v2__messages',
  'browser-shell-v2__message',
  'browser-shell-v2__detail',
  'browser-shell-v2__events',
  'browser-shell-v2__search',
  'browser-shell-v2__issue',
  'browser-shell-v2__approval',
  'browser-shell-v2__controls',
  'browser-shell-v2__receipts',
  'browser-shell-v2__input',
];

const REQUIRED_STYLE_TOKENS = [
  '--browser-shell-v2-bg',
  '--browser-shell-v2-text',
  '--browser-shell-v2-muted',
  '--browser-shell-v2-surface',
  '--browser-shell-v2-border',
  '--browser-shell-v2-radius',
  '--browser-shell-v2-shadow',
];

const REQUIRED_STYLE_PATTERNS = [
  'backdrop-filter: blur',
  'radial-gradient',
  'linear-gradient',
  'box-shadow',
  'color-mix',
  '.browser-shell-v2__message--user',
  '.browser-shell-v2__selector-list button.active',
  '.browser-shell-v2__receipt-list code',
  '@media (max-width: 760px)',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function read(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function push(violations: Violation[], kind: string, pathRel: string, detail: string): void {
  violations.push({ kind, path: pathRel, detail });
}

function validate(componentPath: string, stylesPath: string, readmePath: string, includeControlledViolation: boolean): Violation[] {
  const violations: Violation[] = [];
  const component = `${read(componentPath)}${includeControlledViolation ? '\n<div class="legacy-shell-sidebar"></div>\n' : ''}`;
  const styles = `${read(stylesPath)}${includeControlledViolation ? '\n.browser-shell-v2 { background: white; }\n' : ''}`;
  const readme = read(readmePath);
  for (const surface of REQUIRED_COMPONENT_SURFACES) {
    if (!component.includes(surface)) {
      push(violations, 'missing_visual_surface', componentPath, `Missing V2 visual surface ${surface}.`);
    }
    if (!styles.includes(surface)) {
      push(violations, 'missing_visual_surface_style', stylesPath, `Missing style coverage for ${surface}.`);
    }
  }
  for (const token of REQUIRED_STYLE_TOKENS) {
    if (!styles.includes(token)) push(violations, 'missing_visual_token', stylesPath, `Missing reusable visual token ${token}.`);
  }
  for (const pattern of REQUIRED_STYLE_PATTERNS) {
    if (!styles.includes(pattern)) push(violations, 'missing_visual_pattern', stylesPath, `Missing visual pattern ${pattern}.`);
  }
  if (!readme.includes('familiar skin') || !readme.includes('legacy visual language') || !readme.includes('new substrate')) {
    push(violations, 'visual_parity_not_documented', readmePath, 'README must document familiar-skin/new-substrate visual parity strategy.');
  }
  if (component.includes('legacy-shell') || styles.includes('legacy-shell')) {
    push(violations, 'legacy_visual_selector_dependency', componentPath, 'Browser V2 visual parity must use clean V2 classes, not legacy selectors.');
  }
  return violations;
}

function markdown(report: any): string {
  const lines = [
    '# Browser Shell V2 Visual Parity Guard',
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
const stylesPath = cleanText(readFlag(argv, 'styles') || DEFAULT_STYLES, 600);
const readmePath = cleanText(readFlag(argv, 'readme') || DEFAULT_README, 600);
const strict = parseBool(readFlag(argv, 'strict'), true);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const outJson = cleanText(readFlag(argv, 'out-json') || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const violations = validate(componentPath, stylesPath, readmePath, includeControlledViolation);
const report = {
  ok: violations.length === 0,
  type: 'browser_shell_v2_visual_parity_guard',
  revision: currentRevision(ROOT),
  controlled_violation: includeControlledViolation,
  component_path: componentPath,
  styles_path: stylesPath,
  readme_path: readmePath,
  violations,
};

writeTextArtifact(outMarkdown, markdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
