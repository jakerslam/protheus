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
const DEFAULT_BUILD = 'shell/browser-v2/browser_shell_v2_build.ts';
const DEFAULT_ARTIFACT_CSS = 'core/local/artifacts/browser_shell_v2_app/browser_shell_v2.css';
const DEFAULT_README = 'shell/browser-v2/README.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/browser_shell_v2_visual_parity_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/BROWSER_SHELL_V2_VISUAL_PARITY_GUARD_CURRENT.md';

type Violation = {
  kind: string;
  path: string;
  detail: string;
};

const REQUIRED_COMPONENT_SURFACES = [
  'app-layout',
  'global-taskbar',
  'sidebar',
  'chat-wrapper',
  'messages',
  'message-bubble',
  'chat-map',
  'input-area',
];

const REQUIRED_STYLE_TOKENS = [
  '--bg',
  '--chrome-bg',
  '--sidebar-bg',
  '--surface',
  '--border',
  '--text',
  '--accent',
  '--agent-bg',
  '--user-bg',
];

const REQUIRED_STYLE_PATTERNS = [
  'legacySurfaceCss',
  'theme.css',
  'layout.css.parts',
  'components.css.parts',
  'box-shadow',
  'color-mix',
  '.message.user .message-bubble',
  '.message.agent .message-bubble',
  '.chat-map',
  '.composer-shell',
];

const FORBIDDEN_INVENTED_SURFACES = [
  'browser-shell-v2__topbar',
  'browser-shell-v2__workspace',
  'browser-shell-v2__rail',
  'browser-shell-v2--legacy-surface',
  'Gateway Projection',
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

function validate(componentPath: string, stylesPath: string, buildPath: string, artifactCssPath: string, readmePath: string, includeControlledViolation: boolean): Violation[] {
  const violations: Violation[] = [];
  const component = `${read(componentPath)}${includeControlledViolation ? '\n<div class="legacy-shell-sidebar"></div>\n' : ''}`;
  const styles = `${read(stylesPath)}${includeControlledViolation ? '\n.browser-shell-v2 { background: white; }\n' : ''}`;
  const build = read(buildPath);
  const artifactCss = fs.existsSync(abs(artifactCssPath)) ? read(artifactCssPath) : '';
  const visualSurface = `${component}\n${styles}\n${build}\n${artifactCss}`;
  const readme = read(readmePath);
  if (!styles.includes('Intentionally empty') || styles.includes('{') || styles.includes('}')) {
    push(violations, 'v2_css_defines_visual_rules', stylesPath, 'Browser V2 must not define its own CSS rules; artifact CSS must come from the legacy dashboard bundle.');
  }
  for (const surface of REQUIRED_COMPONENT_SURFACES) {
    if (!visualSurface.includes(surface)) {
      push(violations, 'missing_legacy_visual_surface', buildPath, `Missing legacy dashboard visual surface ${surface}.`);
    }
  }
  for (const token of REQUIRED_STYLE_TOKENS) {
    if (!visualSurface.includes(`var(${token}`) && !visualSurface.includes(`${token}:`)) push(violations, 'missing_visual_token', stylesPath, `Missing legacy visual token ${token}.`);
  }
  for (const pattern of REQUIRED_STYLE_PATTERNS) {
    if (!visualSurface.includes(pattern)) push(violations, 'missing_visual_pattern', buildPath, `Missing legacy visual pattern ${pattern}.`);
  }
  for (const marker of FORBIDDEN_INVENTED_SURFACES) {
    if (build.includes(marker)) push(violations, 'invented_visual_surface', buildPath, `Browser V2 must not emit invented dashboard surface ${marker}.`);
  }
  if (!readme.includes('familiar skin') || !readme.includes('legacy dashboard skin') || !readme.includes('new substrate')) {
    push(violations, 'visual_parity_not_documented', readmePath, 'README must document familiar-skin/new-substrate visual parity strategy.');
  }
  if (component.includes('legacy-shell') || styles.includes('legacy-shell')) {
    push(violations, 'legacy_visual_selector_dependency', componentPath, 'Browser V2 visual parity must use the Shell 1.0 surface contract, not ad hoc legacy-shell placeholders.');
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
const buildPath = cleanText(readFlag(argv, 'build') || DEFAULT_BUILD, 600);
const artifactCssPath = cleanText(readFlag(argv, 'artifact-css') || DEFAULT_ARTIFACT_CSS, 600);
const readmePath = cleanText(readFlag(argv, 'readme') || DEFAULT_README, 600);
const strict = parseBool(readFlag(argv, 'strict'), true);
const includeControlledViolation = parseBool(readFlag(argv, 'include-controlled-violation'), false);
const outJson = cleanText(readFlag(argv, 'out-json') || DEFAULT_OUT_JSON, 600);
const outMarkdown = cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600);
const violations = validate(componentPath, stylesPath, buildPath, artifactCssPath, readmePath, includeControlledViolation);
const report = {
  ok: violations.length === 0,
  type: 'browser_shell_v2_visual_parity_guard',
  revision: currentRevision(ROOT),
  controlled_violation: includeControlledViolation,
  component_path: componentPath,
  styles_path: stylesPath,
  build_path: buildPath,
  artifact_css_path: artifactCssPath,
  readme_path: readmePath,
  violations,
};

writeTextArtifact(outMarkdown, markdown(report));
process.exitCode = emitStructuredResult(report, { outPath: outJson, strict, ok: report.ok });
