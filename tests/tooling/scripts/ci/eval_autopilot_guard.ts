#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

const DEFAULT_MONITOR_PATH = 'artifacts/eval_agent_chat_monitor_guard_latest.json';
const DEFAULT_QUALITY_PATH = 'artifacts/eval_quality_metrics_latest.json';
const DEFAULT_SLO_PATH = 'artifacts/eval_monitor_slo_latest.json';
const DEFAULT_ADVERSARIAL_PATH = 'artifacts/eval_adversarial_guard_latest.json';
const DEFAULT_ISSUE_FILING_PATH = 'artifacts/eval_issue_filing_guard_latest.json';
const DEFAULT_ISSUE_RESOLUTION_PATH = 'artifacts/eval_issue_resolution_latest.json';
const DEFAULT_QUALITY_GATE_PATH = 'artifacts/eval_quality_gate_v1_latest.json';
const DEFAULT_REVIEWER_PATH = 'artifacts/eval_reviewer_feedback_weekly_latest.json';
const DEFAULT_JUDGE_HUMAN_PATH = 'artifacts/eval_judge_human_agreement_latest.json';
const DEFAULT_THRESHOLDS_PATH = 'tests/tooling/config/eval_quality_thresholds.json';

const DEFAULT_OUT_PATH = 'core/local/artifacts/eval_autopilot_guard_current.json';
const DEFAULT_OUT_LATEST_PATH = 'artifacts/eval_autopilot_guard_latest.json';
const DEFAULT_STATE_PATH = 'local/state/ops/eval_autopilot/latest.json';
const DEFAULT_MARKDOWN_PATH = 'local/workspace/reports/EVAL_AUTOPILOT_GUARD_CURRENT.md';

type ActionSeverity = 'critical' | 'high' | 'medium' | 'low' | 'info';

type ActionItem = {
  id: string;
  severity: ActionSeverity;
  category: string;
  summary: string;
  detail: string;
  automatable: boolean;
  recommended_commands: string[];
};

function isCanonicalRelativePath(value: string): boolean {
  const token = cleanText(value || '', 500);
  if (!token) return false;
  if (path.isAbsolute(token)) return false;
  if (token.includes('\\')) return false;
  if (token.includes('..')) return false;
  if (token.includes('//')) return false;
  if (/\s/.test(token)) return false;
  return true;
}

function hasCaseInsensitiveSuffix(value: string, suffix: string): boolean {
  return cleanText(value || '', 500).toLowerCase().endsWith(cleanText(suffix || '', 80).toLowerCase());
}

function isCanonicalToken(value: string, max = 120): boolean {
  return /^[a-z0-9_]+$/.test(cleanText(value || '', max));
}

function isAllowedSeverity(value: string): boolean {
  return ['critical', 'high', 'medium', 'low', 'info'].includes(cleanText(value || '', 40));
}

function isAllowedActionCategory(value: string): boolean {
  return [
    'monitor',
    'model-selection',
    'quality',
    'calibration',
    'slo',
    'adversarial',
    'issue-filing',
    'resolution',
    'reviewer-feedback',
    'judge-human-calibration',
    'quality-gate',
    'status',
  ].includes(cleanText(value || '', 120));
}

function isPlainObject(value: unknown): boolean {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { out: DEFAULT_OUT_PATH });
  return {
    strict: common.strict,
    monitorPath: cleanText(readFlag(argv, 'monitor') || DEFAULT_MONITOR_PATH, 500),
    qualityPath: cleanText(readFlag(argv, 'quality') || DEFAULT_QUALITY_PATH, 500),
    sloPath: cleanText(readFlag(argv, 'slo') || DEFAULT_SLO_PATH, 500),
    adversarialPath: cleanText(readFlag(argv, 'adversarial') || DEFAULT_ADVERSARIAL_PATH, 500),
    issueFilingPath: cleanText(readFlag(argv, 'issue-filing') || DEFAULT_ISSUE_FILING_PATH, 500),
    issueResolutionPath: cleanText(readFlag(argv, 'issue-resolution') || DEFAULT_ISSUE_RESOLUTION_PATH, 500),
    qualityGatePath: cleanText(readFlag(argv, 'quality-gate') || DEFAULT_QUALITY_GATE_PATH, 500),
    reviewerPath: cleanText(readFlag(argv, 'reviewer') || DEFAULT_REVIEWER_PATH, 500),
    judgeHumanPath: cleanText(readFlag(argv, 'judge-human') || DEFAULT_JUDGE_HUMAN_PATH, 500),
    thresholdsPath: cleanText(readFlag(argv, 'thresholds') || DEFAULT_THRESHOLDS_PATH, 500),
    outPath: cleanText(readFlag(argv, 'out') || common.out || DEFAULT_OUT_PATH, 500),
    outLatestPath: cleanText(readFlag(argv, 'out-latest') || DEFAULT_OUT_LATEST_PATH, 500),
    statePath: cleanText(readFlag(argv, 'state') || DEFAULT_STATE_PATH, 500),
    markdownPath: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_MARKDOWN_PATH, 500),
  };
}

function readJson(filePath: string): any | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function safeNumber(raw: unknown, fallback = 0): number {
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function isNonNegativeInteger(raw: unknown): boolean {
  const parsed = Number(raw);
  return Number.isInteger(parsed) && parsed >= 0;
}

function isBoolean(raw: unknown): boolean {
  return typeof raw === 'boolean';
}

function isAsciiPrintable(raw: unknown, max = 500): boolean {
  const token = cleanText(String(raw || ''), max);
  return token.length > 0 && /^[\x20-\x7E]+$/.test(token);
}

function isIsoUtcTimestamp(raw: unknown): boolean {
  const token = cleanText(String(raw || ''), 120);
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{3})?Z$/.test(token)) return false;
  const parsed = Date.parse(token);
  return Number.isFinite(parsed);
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Eval Autopilot Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report.generated_at || '', 120)}`);
  lines.push(`- ok: ${report.ok === true ? 'true' : 'false'}`);
  lines.push(`- autopilot_ready: ${Boolean(report.summary?.autopilot_ready)}`);
  lines.push(`- total_actions: ${Number(report.summary?.total_actions || 0)}`);
  lines.push(`- high_or_critical_actions: ${Number(report.summary?.high_or_critical_actions || 0)}`);
  lines.push('');
  lines.push('## Severity counts');
  const severityCounts = report.summary?.severity_counts || {};
  lines.push(`- critical: ${Number(severityCounts.critical || 0)}`);
  lines.push(`- high: ${Number(severityCounts.high || 0)}`);
  lines.push(`- medium: ${Number(severityCounts.medium || 0)}`);
  lines.push(`- low: ${Number(severityCounts.low || 0)}`);
  lines.push(`- info: ${Number(severityCounts.info || 0)}`);
  lines.push('');
  lines.push('## Actions');
  const actions: ActionItem[] = Array.isArray(report.actions) ? report.actions : [];
  if (actions.length === 0) {
    lines.push('- none');
  } else {
    for (const action of actions) {
      lines.push(`- [${action.severity}] ${action.id}: ${cleanText(action.summary || '', 180)}`);
      lines.push(`  detail=${cleanText(action.detail || '', 260)}`);
      if (Array.isArray(action.recommended_commands) && action.recommended_commands.length > 0) {
        lines.push(`  commands=${action.recommended_commands.join(' ; ')}`);
      }
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const root = process.cwd();
  const nowIso = new Date().toISOString();

  const monitorAbs = path.resolve(root, args.monitorPath);
  const qualityAbs = path.resolve(root, args.qualityPath);
  const sloAbs = path.resolve(root, args.sloPath);
  const adversarialAbs = path.resolve(root, args.adversarialPath);
  const issueFilingAbs = path.resolve(root, args.issueFilingPath);
  const issueResolutionAbs = path.resolve(root, args.issueResolutionPath);
  const qualityGateAbs = path.resolve(root, args.qualityGatePath);
  const reviewerAbs = path.resolve(root, args.reviewerPath);
  const judgeHumanAbs = path.resolve(root, args.judgeHumanPath);
  const thresholdsAbs = path.resolve(root, args.thresholdsPath);

  const monitor = readJson(monitorAbs) || {};
  const quality = readJson(qualityAbs) || {};
  const slo = readJson(sloAbs) || {};
  const adversarial = readJson(adversarialAbs) || {};
  const issueFiling = readJson(issueFilingAbs) || {};
  const issueResolution = readJson(issueResolutionAbs) || {};
  const qualityGate = readJson(qualityGateAbs) || {};
  const reviewer = readJson(reviewerAbs) || {};
  const judgeHuman = readJson(judgeHumanAbs) || {};
  const thresholds = readJson(thresholdsAbs) || {};
  const monitorPayloadObject = isPlainObject(monitor);
  const qualityPayloadObject = isPlainObject(quality);
  const sloPayloadObject = isPlainObject(slo);
  const adversarialPayloadObject = isPlainObject(adversarial);
  const issueFilingPayloadObject = isPlainObject(issueFiling);
  const issueResolutionPayloadObject = isPlainObject(issueResolution);
  const qualityGatePayloadObject = isPlainObject(qualityGate);
  const reviewerPayloadObject = isPlainObject(reviewer);
  const judgeHumanPayloadObject = isPlainObject(judgeHuman);
  const thresholdsPayloadObject = isPlainObject(thresholds);
  const allPathTokens = [
    args.monitorPath,
    args.qualityPath,
    args.sloPath,
    args.adversarialPath,
    args.issueFilingPath,
    args.issueResolutionPath,
    args.qualityGatePath,
    args.reviewerPath,
    args.judgeHumanPath,
    args.thresholdsPath,
    args.outPath,
    args.outLatestPath,
    args.statePath,
    args.markdownPath,
  ];
  const artifactInputPaths = [
    args.monitorPath,
    args.qualityPath,
    args.sloPath,
    args.adversarialPath,
    args.issueFilingPath,
    args.issueResolutionPath,
    args.qualityGatePath,
    args.reviewerPath,
    args.judgeHumanPath,
  ];
  const pathTokensTrimmed = allPathTokens.every((token) => token === token.trim() && token.length > 0);
  const pathTokensNoPlaceholder = allPathTokens.every((token) => !token.includes('${'));
  const pathTokensUnique = new Set(allPathTokens).size === allPathTokens.length;
  const artifactInputsArtifactsPrefix = artifactInputPaths.every((token) => token.startsWith('artifacts/'));
  const thresholdsTestsConfigPrefix = args.thresholdsPath.startsWith('tests/tooling/config/');
  const outputTargetsDistinct = new Set([
    args.outPath,
    args.outLatestPath,
    args.statePath,
    args.markdownPath,
  ]).size === 4;
  const nowMs = Date.now();
  const generatedAtFutureSkewMs = 5 * 60 * 1000;
  const monitorGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'generated_at')
    || isIsoUtcTimestamp(monitor?.generated_at);
  const qualityGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'generated_at')
    || isIsoUtcTimestamp(quality?.generated_at);
  const sloGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'generated_at')
    || isIsoUtcTimestamp(slo?.generated_at);
  const adversarialGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'generated_at')
    || isIsoUtcTimestamp(adversarial?.generated_at);
  const issueFilingGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'generated_at')
    || isIsoUtcTimestamp(issueFiling?.generated_at);
  const issueResolutionGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'generated_at')
    || isIsoUtcTimestamp(issueResolution?.generated_at);
  const qualityGateGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'generated_at')
    || isIsoUtcTimestamp(qualityGate?.generated_at);
  const reviewerGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'generated_at')
    || isIsoUtcTimestamp(reviewer?.generated_at);
  const judgeHumanGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'generated_at')
    || isIsoUtcTimestamp(judgeHuman?.generated_at);
  const thresholdsGeneratedAtIsoUtcOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'generated_at')
    || isIsoUtcTimestamp(thresholds?.generated_at);
  const monitorGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(monitor?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const qualityGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(quality?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const sloGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(slo?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const adversarialGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(adversarial?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const issueFilingGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(issueFiling?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const issueResolutionGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(issueResolution?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const qualityGateGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(qualityGate?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const reviewerGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(reviewer?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const judgeHumanGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(judgeHuman?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const thresholdsGeneratedAtNotFutureOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'generated_at')
    || (() => {
      const parsed = Date.parse(String(thresholds?.generated_at));
      return Number.isFinite(parsed) && parsed <= nowMs + generatedAtFutureSkewMs;
    })();
  const monitorGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'generated_at')
    || String(monitor?.generated_at) === String(monitor?.generated_at).trim();
  const qualityGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'generated_at')
    || String(quality?.generated_at) === String(quality?.generated_at).trim();
  const sloGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'generated_at')
    || String(slo?.generated_at) === String(slo?.generated_at).trim();
  const adversarialGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'generated_at')
    || String(adversarial?.generated_at) === String(adversarial?.generated_at).trim();
  const issueFilingGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'generated_at')
    || String(issueFiling?.generated_at) === String(issueFiling?.generated_at).trim();
  const issueResolutionGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'generated_at')
    || String(issueResolution?.generated_at) === String(issueResolution?.generated_at).trim();
  const qualityGateGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'generated_at')
    || String(qualityGate?.generated_at) === String(qualityGate?.generated_at).trim();
  const reviewerGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'generated_at')
    || String(reviewer?.generated_at) === String(reviewer?.generated_at).trim();
  const judgeHumanGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'generated_at')
    || String(judgeHuman?.generated_at) === String(judgeHuman?.generated_at).trim();
  const thresholdsGeneratedAtTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'generated_at')
    || String(thresholds?.generated_at) === String(thresholds?.generated_at).trim();
  const monitorGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'generated_at')
    || !String(monitor?.generated_at).includes('${');
  const qualityGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'generated_at')
    || !String(quality?.generated_at).includes('${');
  const sloGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'generated_at')
    || !String(slo?.generated_at).includes('${');
  const adversarialGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'generated_at')
    || !String(adversarial?.generated_at).includes('${');
  const issueFilingGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'generated_at')
    || !String(issueFiling?.generated_at).includes('${');
  const issueResolutionGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'generated_at')
    || !String(issueResolution?.generated_at).includes('${');
  const qualityGateGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'generated_at')
    || !String(qualityGate?.generated_at).includes('${');
  const reviewerGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'generated_at')
    || !String(reviewer?.generated_at).includes('${');
  const judgeHumanGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'generated_at')
    || !String(judgeHuman?.generated_at).includes('${');
  const thresholdsGeneratedAtNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'generated_at')
    || !String(thresholds?.generated_at).includes('${');
  const monitorGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'generated_at')
    || typeof monitor?.generated_at === 'string';
  const qualityGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'generated_at')
    || typeof quality?.generated_at === 'string';
  const sloGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'generated_at')
    || typeof slo?.generated_at === 'string';
  const adversarialGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'generated_at')
    || typeof adversarial?.generated_at === 'string';
  const issueFilingGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'generated_at')
    || typeof issueFiling?.generated_at === 'string';
  const issueResolutionGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'generated_at')
    || typeof issueResolution?.generated_at === 'string';
  const qualityGateGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'generated_at')
    || typeof qualityGate?.generated_at === 'string';
  const reviewerGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'generated_at')
    || typeof reviewer?.generated_at === 'string';
  const judgeHumanGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'generated_at')
    || typeof judgeHuman?.generated_at === 'string';
  const thresholdsGeneratedAtStringOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'generated_at')
    || typeof thresholds?.generated_at === 'string';
  const monitorGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'generated_at')
    || String(monitor?.generated_at).length <= 40;
  const qualityGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'generated_at')
    || String(quality?.generated_at).length <= 40;
  const sloGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'generated_at')
    || String(slo?.generated_at).length <= 40;
  const adversarialGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'generated_at')
    || String(adversarial?.generated_at).length <= 40;
  const issueFilingGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'generated_at')
    || String(issueFiling?.generated_at).length <= 40;
  const issueResolutionGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'generated_at')
    || String(issueResolution?.generated_at).length <= 40;
  const qualityGateGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'generated_at')
    || String(qualityGate?.generated_at).length <= 40;
  const reviewerGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'generated_at')
    || String(reviewer?.generated_at).length <= 40;
  const judgeHumanGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'generated_at')
    || String(judgeHuman?.generated_at).length <= 40;
  const thresholdsGeneratedAtLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'generated_at')
    || String(thresholds?.generated_at).length <= 40;

  const checks = [
    { id: 'monitor_artifact_present', ok: fs.existsSync(monitorAbs), detail: args.monitorPath },
    { id: 'quality_artifact_present', ok: fs.existsSync(qualityAbs), detail: args.qualityPath },
    { id: 'slo_artifact_present', ok: fs.existsSync(sloAbs), detail: args.sloPath },
    { id: 'adversarial_artifact_present', ok: fs.existsSync(adversarialAbs), detail: args.adversarialPath },
    { id: 'issue_filing_artifact_present', ok: fs.existsSync(issueFilingAbs), detail: args.issueFilingPath },
    { id: 'issue_resolution_artifact_present', ok: fs.existsSync(issueResolutionAbs), detail: args.issueResolutionPath },
    { id: 'quality_gate_artifact_present', ok: fs.existsSync(qualityGateAbs), detail: args.qualityGatePath },
    { id: 'reviewer_artifact_present', ok: fs.existsSync(reviewerAbs), detail: args.reviewerPath },
    { id: 'judge_human_artifact_present', ok: fs.existsSync(judgeHumanAbs), detail: args.judgeHumanPath },
    { id: 'thresholds_artifact_present', ok: fs.existsSync(thresholdsAbs), detail: args.thresholdsPath },
    { id: 'eval_autopilot_input_monitor_generated_at_iso_utc_or_missing_contract', ok: monitorGeneratedAtIsoUtcOrMissing, detail: String(monitor?.generated_at) },
    { id: 'eval_autopilot_input_quality_generated_at_iso_utc_or_missing_contract', ok: qualityGeneratedAtIsoUtcOrMissing, detail: String(quality?.generated_at) },
    { id: 'eval_autopilot_input_slo_generated_at_iso_utc_or_missing_contract', ok: sloGeneratedAtIsoUtcOrMissing, detail: String(slo?.generated_at) },
    { id: 'eval_autopilot_input_adversarial_generated_at_iso_utc_or_missing_contract', ok: adversarialGeneratedAtIsoUtcOrMissing, detail: String(adversarial?.generated_at) },
    { id: 'eval_autopilot_input_issue_filing_generated_at_iso_utc_or_missing_contract', ok: issueFilingGeneratedAtIsoUtcOrMissing, detail: String(issueFiling?.generated_at) },
    { id: 'eval_autopilot_input_issue_resolution_generated_at_iso_utc_or_missing_contract', ok: issueResolutionGeneratedAtIsoUtcOrMissing, detail: String(issueResolution?.generated_at) },
    { id: 'eval_autopilot_input_quality_gate_generated_at_iso_utc_or_missing_contract', ok: qualityGateGeneratedAtIsoUtcOrMissing, detail: String(qualityGate?.generated_at) },
    { id: 'eval_autopilot_input_reviewer_generated_at_iso_utc_or_missing_contract', ok: reviewerGeneratedAtIsoUtcOrMissing, detail: String(reviewer?.generated_at) },
    { id: 'eval_autopilot_input_judge_human_generated_at_iso_utc_or_missing_contract', ok: judgeHumanGeneratedAtIsoUtcOrMissing, detail: String(judgeHuman?.generated_at) },
    { id: 'eval_autopilot_input_thresholds_generated_at_iso_utc_or_missing_contract', ok: thresholdsGeneratedAtIsoUtcOrMissing, detail: String(thresholds?.generated_at) },
    { id: 'eval_autopilot_input_monitor_generated_at_not_future_or_missing_contract', ok: monitorGeneratedAtNotFutureOrMissing, detail: String(monitor?.generated_at) },
    { id: 'eval_autopilot_input_quality_generated_at_not_future_or_missing_contract', ok: qualityGeneratedAtNotFutureOrMissing, detail: String(quality?.generated_at) },
    { id: 'eval_autopilot_input_slo_generated_at_not_future_or_missing_contract', ok: sloGeneratedAtNotFutureOrMissing, detail: String(slo?.generated_at) },
    { id: 'eval_autopilot_input_adversarial_generated_at_not_future_or_missing_contract', ok: adversarialGeneratedAtNotFutureOrMissing, detail: String(adversarial?.generated_at) },
    { id: 'eval_autopilot_input_issue_filing_generated_at_not_future_or_missing_contract', ok: issueFilingGeneratedAtNotFutureOrMissing, detail: String(issueFiling?.generated_at) },
    { id: 'eval_autopilot_input_issue_resolution_generated_at_not_future_or_missing_contract', ok: issueResolutionGeneratedAtNotFutureOrMissing, detail: String(issueResolution?.generated_at) },
    { id: 'eval_autopilot_input_quality_gate_generated_at_not_future_or_missing_contract', ok: qualityGateGeneratedAtNotFutureOrMissing, detail: String(qualityGate?.generated_at) },
    { id: 'eval_autopilot_input_reviewer_generated_at_not_future_or_missing_contract', ok: reviewerGeneratedAtNotFutureOrMissing, detail: String(reviewer?.generated_at) },
    { id: 'eval_autopilot_input_judge_human_generated_at_not_future_or_missing_contract', ok: judgeHumanGeneratedAtNotFutureOrMissing, detail: String(judgeHuman?.generated_at) },
    { id: 'eval_autopilot_input_thresholds_generated_at_not_future_or_missing_contract', ok: thresholdsGeneratedAtNotFutureOrMissing, detail: String(thresholds?.generated_at) },
    { id: 'eval_autopilot_input_monitor_generated_at_trimmed_or_missing_contract', ok: monitorGeneratedAtTrimmedOrMissing, detail: String(monitor?.generated_at) },
    { id: 'eval_autopilot_input_quality_generated_at_trimmed_or_missing_contract', ok: qualityGeneratedAtTrimmedOrMissing, detail: String(quality?.generated_at) },
    { id: 'eval_autopilot_input_slo_generated_at_trimmed_or_missing_contract', ok: sloGeneratedAtTrimmedOrMissing, detail: String(slo?.generated_at) },
    { id: 'eval_autopilot_input_adversarial_generated_at_trimmed_or_missing_contract', ok: adversarialGeneratedAtTrimmedOrMissing, detail: String(adversarial?.generated_at) },
    { id: 'eval_autopilot_input_issue_filing_generated_at_trimmed_or_missing_contract', ok: issueFilingGeneratedAtTrimmedOrMissing, detail: String(issueFiling?.generated_at) },
    { id: 'eval_autopilot_input_issue_resolution_generated_at_trimmed_or_missing_contract', ok: issueResolutionGeneratedAtTrimmedOrMissing, detail: String(issueResolution?.generated_at) },
    { id: 'eval_autopilot_input_quality_gate_generated_at_trimmed_or_missing_contract', ok: qualityGateGeneratedAtTrimmedOrMissing, detail: String(qualityGate?.generated_at) },
    { id: 'eval_autopilot_input_reviewer_generated_at_trimmed_or_missing_contract', ok: reviewerGeneratedAtTrimmedOrMissing, detail: String(reviewer?.generated_at) },
    { id: 'eval_autopilot_input_judge_human_generated_at_trimmed_or_missing_contract', ok: judgeHumanGeneratedAtTrimmedOrMissing, detail: String(judgeHuman?.generated_at) },
    { id: 'eval_autopilot_input_thresholds_generated_at_trimmed_or_missing_contract', ok: thresholdsGeneratedAtTrimmedOrMissing, detail: String(thresholds?.generated_at) },
    { id: 'eval_autopilot_input_monitor_generated_at_no_placeholder_or_missing_contract', ok: monitorGeneratedAtNoPlaceholderOrMissing, detail: String(monitor?.generated_at) },
    { id: 'eval_autopilot_input_quality_generated_at_no_placeholder_or_missing_contract', ok: qualityGeneratedAtNoPlaceholderOrMissing, detail: String(quality?.generated_at) },
    { id: 'eval_autopilot_input_slo_generated_at_no_placeholder_or_missing_contract', ok: sloGeneratedAtNoPlaceholderOrMissing, detail: String(slo?.generated_at) },
    { id: 'eval_autopilot_input_adversarial_generated_at_no_placeholder_or_missing_contract', ok: adversarialGeneratedAtNoPlaceholderOrMissing, detail: String(adversarial?.generated_at) },
    { id: 'eval_autopilot_input_issue_filing_generated_at_no_placeholder_or_missing_contract', ok: issueFilingGeneratedAtNoPlaceholderOrMissing, detail: String(issueFiling?.generated_at) },
    { id: 'eval_autopilot_input_issue_resolution_generated_at_no_placeholder_or_missing_contract', ok: issueResolutionGeneratedAtNoPlaceholderOrMissing, detail: String(issueResolution?.generated_at) },
    { id: 'eval_autopilot_input_quality_gate_generated_at_no_placeholder_or_missing_contract', ok: qualityGateGeneratedAtNoPlaceholderOrMissing, detail: String(qualityGate?.generated_at) },
    { id: 'eval_autopilot_input_reviewer_generated_at_no_placeholder_or_missing_contract', ok: reviewerGeneratedAtNoPlaceholderOrMissing, detail: String(reviewer?.generated_at) },
    { id: 'eval_autopilot_input_judge_human_generated_at_no_placeholder_or_missing_contract', ok: judgeHumanGeneratedAtNoPlaceholderOrMissing, detail: String(judgeHuman?.generated_at) },
    { id: 'eval_autopilot_input_thresholds_generated_at_no_placeholder_or_missing_contract', ok: thresholdsGeneratedAtNoPlaceholderOrMissing, detail: String(thresholds?.generated_at) },
    { id: 'eval_autopilot_input_monitor_generated_at_string_or_missing_contract', ok: monitorGeneratedAtStringOrMissing, detail: String(monitor?.generated_at) },
    { id: 'eval_autopilot_input_quality_generated_at_string_or_missing_contract', ok: qualityGeneratedAtStringOrMissing, detail: String(quality?.generated_at) },
    { id: 'eval_autopilot_input_slo_generated_at_string_or_missing_contract', ok: sloGeneratedAtStringOrMissing, detail: String(slo?.generated_at) },
    { id: 'eval_autopilot_input_adversarial_generated_at_string_or_missing_contract', ok: adversarialGeneratedAtStringOrMissing, detail: String(adversarial?.generated_at) },
    { id: 'eval_autopilot_input_issue_filing_generated_at_string_or_missing_contract', ok: issueFilingGeneratedAtStringOrMissing, detail: String(issueFiling?.generated_at) },
    { id: 'eval_autopilot_input_issue_resolution_generated_at_string_or_missing_contract', ok: issueResolutionGeneratedAtStringOrMissing, detail: String(issueResolution?.generated_at) },
    { id: 'eval_autopilot_input_quality_gate_generated_at_string_or_missing_contract', ok: qualityGateGeneratedAtStringOrMissing, detail: String(qualityGate?.generated_at) },
    { id: 'eval_autopilot_input_reviewer_generated_at_string_or_missing_contract', ok: reviewerGeneratedAtStringOrMissing, detail: String(reviewer?.generated_at) },
    { id: 'eval_autopilot_input_judge_human_generated_at_string_or_missing_contract', ok: judgeHumanGeneratedAtStringOrMissing, detail: String(judgeHuman?.generated_at) },
    { id: 'eval_autopilot_input_thresholds_generated_at_string_or_missing_contract', ok: thresholdsGeneratedAtStringOrMissing, detail: String(thresholds?.generated_at) },
    { id: 'eval_autopilot_input_monitor_generated_at_length_bounded_or_missing_contract', ok: monitorGeneratedAtLengthBoundedOrMissing, detail: String(monitor?.generated_at) },
    { id: 'eval_autopilot_input_quality_generated_at_length_bounded_or_missing_contract', ok: qualityGeneratedAtLengthBoundedOrMissing, detail: String(quality?.generated_at) },
    { id: 'eval_autopilot_input_slo_generated_at_length_bounded_or_missing_contract', ok: sloGeneratedAtLengthBoundedOrMissing, detail: String(slo?.generated_at) },
    { id: 'eval_autopilot_input_adversarial_generated_at_length_bounded_or_missing_contract', ok: adversarialGeneratedAtLengthBoundedOrMissing, detail: String(adversarial?.generated_at) },
    { id: 'eval_autopilot_input_issue_filing_generated_at_length_bounded_or_missing_contract', ok: issueFilingGeneratedAtLengthBoundedOrMissing, detail: String(issueFiling?.generated_at) },
    { id: 'eval_autopilot_input_issue_resolution_generated_at_length_bounded_or_missing_contract', ok: issueResolutionGeneratedAtLengthBoundedOrMissing, detail: String(issueResolution?.generated_at) },
    { id: 'eval_autopilot_input_quality_gate_generated_at_length_bounded_or_missing_contract', ok: qualityGateGeneratedAtLengthBoundedOrMissing, detail: String(qualityGate?.generated_at) },
    { id: 'eval_autopilot_input_reviewer_generated_at_length_bounded_or_missing_contract', ok: reviewerGeneratedAtLengthBoundedOrMissing, detail: String(reviewer?.generated_at) },
    { id: 'eval_autopilot_input_judge_human_generated_at_length_bounded_or_missing_contract', ok: judgeHumanGeneratedAtLengthBoundedOrMissing, detail: String(judgeHuman?.generated_at) },
    { id: 'eval_autopilot_input_thresholds_generated_at_length_bounded_or_missing_contract', ok: thresholdsGeneratedAtLengthBoundedOrMissing, detail: String(thresholds?.generated_at) },
    { id: 'eval_autopilot_monitor_path_canonical_contract', ok: isCanonicalRelativePath(args.monitorPath), detail: args.monitorPath },
    { id: 'eval_autopilot_quality_path_canonical_contract', ok: isCanonicalRelativePath(args.qualityPath), detail: args.qualityPath },
    { id: 'eval_autopilot_slo_path_canonical_contract', ok: isCanonicalRelativePath(args.sloPath), detail: args.sloPath },
    { id: 'eval_autopilot_adversarial_path_canonical_contract', ok: isCanonicalRelativePath(args.adversarialPath), detail: args.adversarialPath },
    { id: 'eval_autopilot_issue_filing_path_canonical_contract', ok: isCanonicalRelativePath(args.issueFilingPath), detail: args.issueFilingPath },
    { id: 'eval_autopilot_issue_resolution_path_canonical_contract', ok: isCanonicalRelativePath(args.issueResolutionPath), detail: args.issueResolutionPath },
    { id: 'eval_autopilot_quality_gate_path_canonical_contract', ok: isCanonicalRelativePath(args.qualityGatePath), detail: args.qualityGatePath },
    { id: 'eval_autopilot_reviewer_path_canonical_contract', ok: isCanonicalRelativePath(args.reviewerPath), detail: args.reviewerPath },
    { id: 'eval_autopilot_judge_human_path_canonical_contract', ok: isCanonicalRelativePath(args.judgeHumanPath), detail: args.judgeHumanPath },
    { id: 'eval_autopilot_thresholds_path_canonical_contract', ok: isCanonicalRelativePath(args.thresholdsPath), detail: args.thresholdsPath },
    { id: 'eval_autopilot_out_path_canonical_contract', ok: isCanonicalRelativePath(args.outPath), detail: args.outPath },
    { id: 'eval_autopilot_out_latest_path_canonical_contract', ok: isCanonicalRelativePath(args.outLatestPath), detail: args.outLatestPath },
    { id: 'eval_autopilot_state_path_canonical_contract', ok: isCanonicalRelativePath(args.statePath), detail: args.statePath },
    { id: 'eval_autopilot_markdown_path_canonical_contract', ok: isCanonicalRelativePath(args.markdownPath), detail: args.markdownPath },
    { id: 'eval_autopilot_out_path_current_suffix_contract', ok: hasCaseInsensitiveSuffix(args.outPath, '_current.json'), detail: args.outPath },
    { id: 'eval_autopilot_out_latest_path_latest_suffix_contract', ok: hasCaseInsensitiveSuffix(args.outLatestPath, '_latest.json'), detail: args.outLatestPath },
    { id: 'eval_autopilot_markdown_path_current_suffix_contract', ok: hasCaseInsensitiveSuffix(args.markdownPath, '_CURRENT.md'), detail: args.markdownPath },
    { id: 'eval_autopilot_out_path_artifacts_prefix_contract', ok: args.outPath.startsWith('core/local/artifacts/'), detail: args.outPath },
    { id: 'eval_autopilot_state_path_local_state_prefix_contract', ok: args.statePath.startsWith('local/state/'), detail: args.statePath },
    { id: 'eval_autopilot_markdown_reports_prefix_contract', ok: args.markdownPath.startsWith('local/workspace/reports/'), detail: args.markdownPath },
    { id: 'eval_autopilot_monitor_path_default_contract', ok: args.monitorPath === DEFAULT_MONITOR_PATH, detail: args.monitorPath },
    { id: 'eval_autopilot_quality_path_default_contract', ok: args.qualityPath === DEFAULT_QUALITY_PATH, detail: args.qualityPath },
    { id: 'eval_autopilot_slo_path_default_contract', ok: args.sloPath === DEFAULT_SLO_PATH, detail: args.sloPath },
    { id: 'eval_autopilot_adversarial_path_default_contract', ok: args.adversarialPath === DEFAULT_ADVERSARIAL_PATH, detail: args.adversarialPath },
    { id: 'eval_autopilot_issue_filing_path_default_contract', ok: args.issueFilingPath === DEFAULT_ISSUE_FILING_PATH, detail: args.issueFilingPath },
    { id: 'eval_autopilot_issue_resolution_path_default_contract', ok: args.issueResolutionPath === DEFAULT_ISSUE_RESOLUTION_PATH, detail: args.issueResolutionPath },
    { id: 'eval_autopilot_quality_gate_path_default_contract', ok: args.qualityGatePath === DEFAULT_QUALITY_GATE_PATH, detail: args.qualityGatePath },
    { id: 'eval_autopilot_reviewer_path_default_contract', ok: args.reviewerPath === DEFAULT_REVIEWER_PATH, detail: args.reviewerPath },
    { id: 'eval_autopilot_judge_human_path_default_contract', ok: args.judgeHumanPath === DEFAULT_JUDGE_HUMAN_PATH, detail: args.judgeHumanPath },
    { id: 'eval_autopilot_thresholds_path_default_contract', ok: args.thresholdsPath === DEFAULT_THRESHOLDS_PATH, detail: args.thresholdsPath },
    { id: 'eval_autopilot_out_path_default_contract', ok: args.outPath === DEFAULT_OUT_PATH, detail: args.outPath },
    { id: 'eval_autopilot_out_latest_path_default_contract', ok: args.outLatestPath === DEFAULT_OUT_LATEST_PATH, detail: args.outLatestPath },
    { id: 'eval_autopilot_state_path_default_contract', ok: args.statePath === DEFAULT_STATE_PATH, detail: args.statePath },
    { id: 'eval_autopilot_markdown_path_default_contract', ok: args.markdownPath === DEFAULT_MARKDOWN_PATH, detail: args.markdownPath },
    { id: 'eval_autopilot_path_tokens_trimmed_contract', ok: pathTokensTrimmed, detail: allPathTokens.join('|') },
    { id: 'eval_autopilot_path_tokens_no_placeholder_contract', ok: pathTokensNoPlaceholder, detail: allPathTokens.join('|') },
    { id: 'eval_autopilot_path_tokens_unique_contract', ok: pathTokensUnique, detail: allPathTokens.join('|') },
    { id: 'eval_autopilot_artifact_inputs_prefix_contract', ok: artifactInputsArtifactsPrefix, detail: artifactInputPaths.join('|') },
    { id: 'eval_autopilot_thresholds_tests_config_prefix_contract', ok: thresholdsTestsConfigPrefix, detail: args.thresholdsPath },
    { id: 'eval_autopilot_output_targets_distinct_contract', ok: outputTargetsDistinct, detail: `${args.outPath}|${args.outLatestPath}|${args.statePath}|${args.markdownPath}` },
  ];
  const monitorOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'ok') || typeof monitor?.ok === 'boolean';
  const qualityOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'ok') || typeof quality?.ok === 'boolean';
  const sloOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'ok') || typeof slo?.ok === 'boolean';
  const adversarialOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'ok') || typeof adversarial?.ok === 'boolean';
  const issueFilingSummaryObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'summary') || isPlainObject(issueFiling?.summary);
  const issueResolutionSummaryObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'summary') || isPlainObject(issueResolution?.summary);
  const qualityGateSummaryObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'summary') || isPlainObject(qualityGate?.summary);
  const reviewerSummaryObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'summary') || isPlainObject(reviewer?.summary);
  const judgeHumanSummaryObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'summary') || isPlainObject(judgeHuman?.summary);
  const thresholdsGlobalObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'global') || isPlainObject(thresholds?.global);
  const monitorChecksArrayOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'checks') || Array.isArray(monitor?.checks);
  const qualityThresholdViolationsArrayOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'threshold_violations') || Array.isArray(quality?.threshold_violations);
  const qualityRegressionViolationsArrayOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'regression_violations') || Array.isArray(quality?.regression_violations);
  const sloAlertsArrayOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'alerts') || Array.isArray(slo?.alerts);
  const issueResolutionLinkedIssueStatusArrayOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'linked_issue_status')
    || Array.isArray(issueResolution?.linked_issue_status);
  const reviewerFeedbackRowsNonnegativeOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'feedback_rows')
    || safeNumber(reviewer?.summary?.feedback_rows, -1) >= 0;
  const qualityGateRemainingToUnlockNonnegativeOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'remaining_to_unlock')
    || safeNumber(qualityGate?.summary?.remaining_to_unlock, -1) >= 0;
  const judgeComparableSamplesNonnegativeOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'comparable_samples')
    || safeNumber(judgeHuman?.summary?.comparable_samples, -1) >= 0;
  const thresholdsCalibrationErrorMaxBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.global || {}, 'calibration_error_max')
    || (() => {
      const value = safeNumber(thresholds?.global?.calibration_error_max, -1);
      return value > 0 && value <= 1;
    })();
  const thresholdsJudgeHumanAgreementMinBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.global || {}, 'judge_human_agreement_min')
    || (() => {
      const value = safeNumber(thresholds?.global?.judge_human_agreement_min, -1);
      return value >= 0 && value <= 1;
    })();
  checks.push({
    id: 'eval_autopilot_input_monitor_ok_boolean_or_missing_contract',
    ok: monitorOkBooleanOrMissing,
    detail: String(monitor?.ok),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_ok_boolean_or_missing_contract',
    ok: qualityOkBooleanOrMissing,
    detail: String(quality?.ok),
  });
  checks.push({
    id: 'eval_autopilot_input_slo_ok_boolean_or_missing_contract',
    ok: sloOkBooleanOrMissing,
    detail: String(slo?.ok),
  });
  checks.push({
    id: 'eval_autopilot_input_adversarial_ok_boolean_or_missing_contract',
    ok: adversarialOkBooleanOrMissing,
    detail: String(adversarial?.ok),
  });
  checks.push({
    id: 'eval_autopilot_input_issue_filing_summary_object_or_missing_contract',
    ok: issueFilingSummaryObjectOrMissing,
    detail: String(issueFiling?.summary ? Object.keys(issueFiling.summary).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_issue_resolution_summary_object_or_missing_contract',
    ok: issueResolutionSummaryObjectOrMissing,
    detail: String(issueResolution?.summary ? Object.keys(issueResolution.summary).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_gate_summary_object_or_missing_contract',
    ok: qualityGateSummaryObjectOrMissing,
    detail: String(qualityGate?.summary ? Object.keys(qualityGate.summary).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_reviewer_summary_object_or_missing_contract',
    ok: reviewerSummaryObjectOrMissing,
    detail: String(reviewer?.summary ? Object.keys(reviewer.summary).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_judge_human_summary_object_or_missing_contract',
    ok: judgeHumanSummaryObjectOrMissing,
    detail: String(judgeHuman?.summary ? Object.keys(judgeHuman.summary).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_thresholds_global_object_or_missing_contract',
    ok: thresholdsGlobalObjectOrMissing,
    detail: String(thresholds?.global ? Object.keys(thresholds.global).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_checks_array_or_missing_contract',
    ok: monitorChecksArrayOrMissing,
    detail: String(Array.isArray(monitor?.checks)),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_threshold_violations_array_or_missing_contract',
    ok: qualityThresholdViolationsArrayOrMissing,
    detail: String(Array.isArray(quality?.threshold_violations)),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_regression_violations_array_or_missing_contract',
    ok: qualityRegressionViolationsArrayOrMissing,
    detail: String(Array.isArray(quality?.regression_violations)),
  });
  checks.push({
    id: 'eval_autopilot_input_slo_alerts_array_or_missing_contract',
    ok: sloAlertsArrayOrMissing,
    detail: String(Array.isArray(slo?.alerts)),
  });
  checks.push({
    id: 'eval_autopilot_input_issue_resolution_linked_issue_status_array_or_missing_contract',
    ok: issueResolutionLinkedIssueStatusArrayOrMissing,
    detail: String(Array.isArray(issueResolution?.linked_issue_status)),
  });
  checks.push({
    id: 'eval_autopilot_input_reviewer_feedback_rows_nonnegative_or_missing_contract',
    ok: reviewerFeedbackRowsNonnegativeOrMissing,
    detail: String(reviewer?.summary?.feedback_rows),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_gate_remaining_to_unlock_nonnegative_or_missing_contract',
    ok: qualityGateRemainingToUnlockNonnegativeOrMissing,
    detail: String(qualityGate?.summary?.remaining_to_unlock),
  });
  checks.push({
    id: 'eval_autopilot_input_judge_comparable_samples_nonnegative_or_missing_contract',
    ok: judgeComparableSamplesNonnegativeOrMissing,
    detail: String(judgeHuman?.summary?.comparable_samples),
  });
  checks.push({
    id: 'eval_autopilot_input_thresholds_calibration_error_max_bounded_or_missing_contract',
    ok: thresholdsCalibrationErrorMaxBoundedOrMissing,
    detail: String(thresholds?.global?.calibration_error_max),
  });
  checks.push({
    id: 'eval_autopilot_input_thresholds_judge_human_agreement_min_bounded_or_missing_contract',
    ok: thresholdsJudgeHumanAgreementMinBoundedOrMissing,
    detail: String(thresholds?.global?.judge_human_agreement_min),
  });
  const monitorSummaryObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'summary') || isPlainObject(monitor?.summary);
  const monitorSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor?.summary || {}, 'status')
    || isCanonicalToken(cleanText(String(monitor?.summary?.status || ''), 80), 80);
  const qualityMetricsObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'metrics') || isPlainObject(quality?.metrics);
  const qualityMetricsOverallObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(quality?.metrics || {}, 'overall')
    || isPlainObject(quality?.metrics?.overall);
  const qualityMetricsCalibrationErrorBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(quality?.metrics?.overall || {}, 'calibration_error')
    || (() => {
      const value = safeNumber(quality?.metrics?.overall?.calibration_error, -1);
      return value >= 0 && value <= 1;
    })();
  const issueFilingBlockedCountIntegerNonnegativeOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'blocked_count')
    || isNonNegativeInteger(issueFiling?.summary?.blocked_count);
  const issueFilingReadyToFileCountIntegerNonnegativeOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'ready_to_file_count')
    || isNonNegativeInteger(issueFiling?.summary?.ready_to_file_count);
  const issueResolutionFixFailedCountIntegerNonnegativeOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'fix_failed_count')
    || isNonNegativeInteger(issueResolution?.summary?.fix_failed_count);
  const issueResolutionLinkedIssueRows = Array.isArray(issueResolution?.linked_issue_status)
    ? issueResolution.linked_issue_status
    : [];
  const issueResolutionLinkedIssueRowsObjectOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'linked_issue_status')
    || issueResolutionLinkedIssueRows.every((row: any) => isPlainObject(row));
  const issueResolutionLinkedIssueIdTokenOrMissing =
    issueResolutionLinkedIssueRows.every((row: any) =>
      !Object.prototype.hasOwnProperty.call(row || {}, 'id')
      || isCanonicalToken(cleanText(String(row?.id || ''), 120), 120),
    );
  const issueResolutionLinkedIssueFixFailedBooleanOrMissing =
    issueResolutionLinkedIssueRows.every((row: any) =>
      !Object.prototype.hasOwnProperty.call(row || {}, 'fix_failed')
      || typeof row?.fix_failed === 'boolean',
    );
  const monitorCheckRows = Array.isArray(monitor?.checks) ? monitor.checks : [];
  const monitorCheckRowsObjectContract = monitorCheckRows.every((row: any) => isPlainObject(row));
  const monitorCheckRowsIdTokenOrMissingContract = monitorCheckRows.every((row: any) =>
    !Object.prototype.hasOwnProperty.call(row || {}, 'id')
    || isCanonicalToken(cleanText(String(row?.id || ''), 160), 160),
  );
  const monitorCheckRowsOkBooleanOrMissingContract = monitorCheckRows.every((row: any) =>
    !Object.prototype.hasOwnProperty.call(row || {}, 'ok')
    || typeof row?.ok === 'boolean',
  );
  const monitorCheckRowsDetailStringOrMissingContract = monitorCheckRows.every((row: any) =>
    !Object.prototype.hasOwnProperty.call(row || {}, 'detail')
    || typeof row?.detail === 'string',
  );
  const monitorCheckRowsDetailTrimmedOrMissingContract = monitorCheckRows.every((row: any) => {
    if (!Object.prototype.hasOwnProperty.call(row || {}, 'detail')) return true;
    const raw = String(row?.detail || '');
    const token = cleanText(raw, 600);
    return token.length > 0 && token === raw.trim();
  });
  const monitorCheckRowsDetailNoPlaceholderOrMissingContract = monitorCheckRows.every((row: any) =>
    !Object.prototype.hasOwnProperty.call(row || {}, 'detail')
    || !cleanText(String(row?.detail || ''), 600).includes('${'),
  );
  const monitorCheckRowsDetailAsciiOrMissingContract = monitorCheckRows.every((row: any) =>
    !Object.prototype.hasOwnProperty.call(row || {}, 'detail')
    || isAsciiPrintable(row?.detail, 600),
  );
  const qualityThresholdViolationRowsObjectOrMissingContract =
    !Object.prototype.hasOwnProperty.call(quality, 'threshold_violations')
    || (Array.isArray(quality?.threshold_violations)
      && quality.threshold_violations.every((row: any) => isPlainObject(row)));
  const qualityRegressionViolationRowsObjectOrMissingContract =
    !Object.prototype.hasOwnProperty.call(quality, 'regression_violations')
    || (Array.isArray(quality?.regression_violations)
      && quality.regression_violations.every((row: any) => isPlainObject(row)));
  checks.push({
    id: 'eval_autopilot_input_monitor_summary_object_or_missing_contract',
    ok: monitorSummaryObjectOrMissing,
    detail: String(monitor?.summary ? Object.keys(monitor.summary).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_summary_status_token_or_missing_contract',
    ok: monitorSummaryStatusTokenOrMissing,
    detail: cleanText(String(monitor?.summary?.status || ''), 80),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_metrics_object_or_missing_contract',
    ok: qualityMetricsObjectOrMissing,
    detail: String(quality?.metrics ? Object.keys(quality.metrics).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_metrics_overall_object_or_missing_contract',
    ok: qualityMetricsOverallObjectOrMissing,
    detail: String(quality?.metrics?.overall ? Object.keys(quality.metrics.overall).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_metrics_calibration_error_bounded_or_missing_contract',
    ok: qualityMetricsCalibrationErrorBoundedOrMissing,
    detail: String(quality?.metrics?.overall?.calibration_error),
  });
  checks.push({
    id: 'eval_autopilot_input_issue_filing_blocked_count_integer_nonnegative_or_missing_contract',
    ok: issueFilingBlockedCountIntegerNonnegativeOrMissing,
    detail: String(issueFiling?.summary?.blocked_count),
  });
  checks.push({
    id: 'eval_autopilot_input_issue_filing_ready_to_file_count_integer_nonnegative_or_missing_contract',
    ok: issueFilingReadyToFileCountIntegerNonnegativeOrMissing,
    detail: String(issueFiling?.summary?.ready_to_file_count),
  });
  checks.push({
    id: 'eval_autopilot_input_issue_resolution_fix_failed_count_integer_nonnegative_or_missing_contract',
    ok: issueResolutionFixFailedCountIntegerNonnegativeOrMissing,
    detail: String(issueResolution?.summary?.fix_failed_count),
  });
  checks.push({
    id: 'eval_autopilot_input_issue_resolution_linked_issue_rows_object_or_missing_contract',
    ok: issueResolutionLinkedIssueRowsObjectOrMissing,
    detail: `count=${issueResolutionLinkedIssueRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_issue_resolution_linked_issue_id_token_or_missing_contract',
    ok: issueResolutionLinkedIssueIdTokenOrMissing,
    detail: `count=${issueResolutionLinkedIssueRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_issue_resolution_linked_issue_fix_failed_boolean_or_missing_contract',
    ok: issueResolutionLinkedIssueFixFailedBooleanOrMissing,
    detail: `count=${issueResolutionLinkedIssueRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_object_contract',
    ok: monitorCheckRowsObjectContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_id_token_or_missing_contract',
    ok: monitorCheckRowsIdTokenOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_ok_boolean_or_missing_contract',
    ok: monitorCheckRowsOkBooleanOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_detail_string_or_missing_contract',
    ok: monitorCheckRowsDetailStringOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_detail_trimmed_or_missing_contract',
    ok: monitorCheckRowsDetailTrimmedOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_detail_no_placeholder_or_missing_contract',
    ok: monitorCheckRowsDetailNoPlaceholderOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_detail_ascii_or_missing_contract',
    ok: monitorCheckRowsDetailAsciiOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_quality_threshold_violation_rows_object_or_missing_contract',
    ok: qualityThresholdViolationRowsObjectOrMissingContract,
    detail: String(Array.isArray(quality?.threshold_violations) ? quality.threshold_violations.length : 0),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_regression_violation_rows_object_or_missing_contract',
    ok: qualityRegressionViolationRowsObjectOrMissingContract,
    detail: String(Array.isArray(quality?.regression_violations) ? quality.regression_violations.length : 0),
  });
  const monitorCheckIds = monitorCheckRows.map((row: any) => cleanText(String(row?.id || ''), 160));
  const monitorCheckRowsIdsUniqueOrMissingContract =
    !Object.prototype.hasOwnProperty.call(monitor, 'checks') || new Set(monitorCheckIds).size === monitorCheckIds.length;
  const monitorCheckRowsIdsCasefoldUniqueOrMissingContract =
    !Object.prototype.hasOwnProperty.call(monitor, 'checks')
    || new Set(monitorCheckIds.map((id) => id.toLowerCase())).size === monitorCheckIds.length;
  const monitorCheckRowsDetailNonemptyOrMissingContract =
    !Object.prototype.hasOwnProperty.call(monitor, 'checks')
    || monitorCheckRows.every((row: any) =>
      !Object.prototype.hasOwnProperty.call(row || {}, 'detail')
      || cleanText(String(row?.detail || ''), 600).length > 0,
    );
  const monitorCheckRowsDetailLengthBoundedOrMissingContract =
    !Object.prototype.hasOwnProperty.call(monitor, 'checks')
    || monitorCheckRows.every((row: any) =>
      !Object.prototype.hasOwnProperty.call(row || {}, 'detail')
      || cleanText(String(row?.detail || ''), 600).length <= 600,
    );
  const monitorCheckRowsCountReasonableOrMissingContract =
    !Object.prototype.hasOwnProperty.call(monitor, 'checks') || monitorCheckRows.length <= 500;
  const qualityThresholdViolationRows = Array.isArray(quality?.threshold_violations)
    ? quality.threshold_violations
    : [];
  const qualityRegressionViolationRows = Array.isArray(quality?.regression_violations)
    ? quality.regression_violations
    : [];
  const qualityThresholdViolationRowsCountReasonableOrMissingContract =
    !Object.prototype.hasOwnProperty.call(quality, 'threshold_violations')
    || qualityThresholdViolationRows.length <= 500;
  const qualityRegressionViolationRowsCountReasonableOrMissingContract =
    !Object.prototype.hasOwnProperty.call(quality, 'regression_violations')
    || qualityRegressionViolationRows.length <= 500;
  const sloAlertRows = Array.isArray(slo?.alerts) ? slo.alerts : [];
  const sloAlertRowsObjectOrMissingContract =
    !Object.prototype.hasOwnProperty.call(slo, 'alerts')
    || sloAlertRows.every((row: any) => isPlainObject(row));
  const sloAlertRowsCountReasonableOrMissingContract =
    !Object.prototype.hasOwnProperty.call(slo, 'alerts') || sloAlertRows.length <= 500;
  const issueResolutionLinkedIssueRowsCountReasonableOrMissingContract =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'linked_issue_status')
    || issueResolutionLinkedIssueRows.length <= 500;
  const issueResolutionFixFailedCountWithinLinkedRowsOrMissingContract =
    !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'fix_failed_count')
    || safeNumber(issueResolution?.summary?.fix_failed_count, 0) <= issueResolutionLinkedIssueRows.length;
  const issueFilingCountsIntegerConsistencyOrMissingContract =
    (
      !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'blocked_count')
      || isNonNegativeInteger(issueFiling?.summary?.blocked_count)
    ) && (
      !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'ready_to_file_count')
      || isNonNegativeInteger(issueFiling?.summary?.ready_to_file_count)
    );
  const qualityGateOkBooleanOrMissingContract =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'ok') || typeof qualityGate?.ok === 'boolean';
  const qualityGateAutonomousEscalationAllowedBooleanOrMissingContract =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'autonomous_escalation_allowed')
    || typeof qualityGate?.summary?.autonomous_escalation_allowed === 'boolean';
  const qualityGateRemainingToUnlockIntegerNonnegativeOrMissingContract =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'remaining_to_unlock')
    || isNonNegativeInteger(qualityGate?.summary?.remaining_to_unlock);
  const reviewerStatusTokenOrMissingContract =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'status')
    || isCanonicalToken(cleanText(String(reviewer?.summary?.status || ''), 80), 80);
  const reviewerFeedbackRowsIntegerNonnegativeOrMissingContract =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'feedback_rows')
    || isNonNegativeInteger(reviewer?.summary?.feedback_rows);
  const judgeHumanAgreementRateBoundedOrMissingContract =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'agreement_rate')
    || (() => {
      const value = safeNumber(judgeHuman?.summary?.agreement_rate, -1);
      return value >= 0 && value <= 1;
    })();
  const judgeHumanAgreementMinBoundedOrMissingContract =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'agreement_min')
    || (() => {
      const value = safeNumber(judgeHuman?.summary?.agreement_min, -1);
      return value >= 0 && value <= 1;
    })();
  const judgeHumanMinimumSamplesIntegerNonnegativeOrMissingContract =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'minimum_samples')
    || isNonNegativeInteger(judgeHuman?.summary?.minimum_samples);
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_ids_unique_or_missing_contract',
    ok: monitorCheckRowsIdsUniqueOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_ids_casefold_unique_or_missing_contract',
    ok: monitorCheckRowsIdsCasefoldUniqueOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_detail_nonempty_or_missing_contract',
    ok: monitorCheckRowsDetailNonemptyOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_detail_length_bounded_or_missing_contract',
    ok: monitorCheckRowsDetailLengthBoundedOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_monitor_check_rows_count_reasonable_or_missing_contract',
    ok: monitorCheckRowsCountReasonableOrMissingContract,
    detail: `count=${monitorCheckRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_quality_threshold_violation_rows_count_reasonable_or_missing_contract',
    ok: qualityThresholdViolationRowsCountReasonableOrMissingContract,
    detail: `count=${qualityThresholdViolationRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_quality_regression_violation_rows_count_reasonable_or_missing_contract',
    ok: qualityRegressionViolationRowsCountReasonableOrMissingContract,
    detail: `count=${qualityRegressionViolationRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_slo_alert_rows_object_or_missing_contract',
    ok: sloAlertRowsObjectOrMissingContract,
    detail: `count=${sloAlertRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_slo_alert_rows_count_reasonable_or_missing_contract',
    ok: sloAlertRowsCountReasonableOrMissingContract,
    detail: `count=${sloAlertRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_issue_resolution_linked_issue_rows_count_reasonable_or_missing_contract',
    ok: issueResolutionLinkedIssueRowsCountReasonableOrMissingContract,
    detail: `count=${issueResolutionLinkedIssueRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_issue_resolution_fix_failed_count_within_linked_rows_or_missing_contract',
    ok: issueResolutionFixFailedCountWithinLinkedRowsOrMissingContract,
    detail: `${safeNumber(issueResolution?.summary?.fix_failed_count, 0)}|${issueResolutionLinkedIssueRows.length}`,
  });
  checks.push({
    id: 'eval_autopilot_input_issue_filing_counts_integer_consistency_or_missing_contract',
    ok: issueFilingCountsIntegerConsistencyOrMissingContract,
    detail: `${String(issueFiling?.summary?.blocked_count)}|${String(issueFiling?.summary?.ready_to_file_count)}`,
  });
  checks.push({
    id: 'eval_autopilot_input_quality_gate_ok_boolean_or_missing_contract',
    ok: qualityGateOkBooleanOrMissingContract,
    detail: String(qualityGate?.ok),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_gate_autonomous_escalation_allowed_boolean_or_missing_contract',
    ok: qualityGateAutonomousEscalationAllowedBooleanOrMissingContract,
    detail: String(qualityGate?.summary?.autonomous_escalation_allowed),
  });
  checks.push({
    id: 'eval_autopilot_input_quality_gate_remaining_to_unlock_integer_nonnegative_or_missing_contract',
    ok: qualityGateRemainingToUnlockIntegerNonnegativeOrMissingContract,
    detail: String(qualityGate?.summary?.remaining_to_unlock),
  });
  checks.push({
    id: 'eval_autopilot_input_reviewer_status_token_or_missing_contract',
    ok: reviewerStatusTokenOrMissingContract,
    detail: cleanText(String(reviewer?.summary?.status || ''), 80),
  });
  checks.push({
    id: 'eval_autopilot_input_reviewer_feedback_rows_integer_nonnegative_or_missing_contract',
    ok: reviewerFeedbackRowsIntegerNonnegativeOrMissingContract,
    detail: String(reviewer?.summary?.feedback_rows),
  });
  checks.push({
    id: 'eval_autopilot_input_judge_human_agreement_rate_bounded_or_missing_contract',
    ok: judgeHumanAgreementRateBoundedOrMissingContract,
    detail: String(judgeHuman?.summary?.agreement_rate),
  });
  checks.push({
    id: 'eval_autopilot_input_judge_human_agreement_min_bounded_or_missing_contract',
    ok: judgeHumanAgreementMinBoundedOrMissingContract,
    detail: String(judgeHuman?.summary?.agreement_min),
  });
  checks.push({
    id: 'eval_autopilot_input_judge_human_minimum_samples_integer_nonnegative_or_missing_contract',
    ok: judgeHumanMinimumSamplesIntegerNonnegativeOrMissingContract,
    detail: String(judgeHuman?.summary?.minimum_samples),
  });

  const actions: ActionItem[] = [];
  const pushAction = (action: ActionItem): void => actions.push(action);

  if (monitor?.ok === false) {
    pushAction({
      id: 'eval_monitor_contract_breach',
      severity: 'high',
      category: 'monitor',
      summary: 'Eval chat-monitor guard is failing and needs remediation.',
      detail: `monitor_ok=false;check_failures=${Array.isArray(monitor?.checks) ? monitor.checks.filter((row: any) => row?.ok === false).length : 0}`,
      automatable: true,
      recommended_commands: ['npm run -s ops:eval-agent:chat-monitor:guard'],
    });
  }

  const monitorStrongModelOk = Array.isArray(monitor?.checks)
    ? monitor.checks.some((row: any) => cleanText(row?.id || '', 120) === 'strong_eval_default_model_contract' && row?.ok === true)
    : false;
  if (!monitorStrongModelOk) {
    pushAction({
      id: 'eval_model_not_strong_default',
      severity: 'high',
      category: 'model-selection',
      summary: 'Eval monitor did not confirm strong-model default contract.',
      detail: 'strong_eval_default_model_contract is missing or failed; configure eval lane to use a strong model by default.',
      automatable: true,
      recommended_commands: [
        'npm run -s ops:eval-agent:chat-monitor:guard',
        'npm run -s ops:eval:quality:metrics:guard',
      ],
    });
  }

  const thresholdViolations = Array.isArray(quality?.threshold_violations)
    ? quality.threshold_violations.length
    : 0;
  const regressionViolations = Array.isArray(quality?.regression_violations)
    ? quality.regression_violations.length
    : 0;
  if (thresholdViolations > 0 || regressionViolations > 0 || quality?.ok === false) {
    pushAction({
      id: 'eval_quality_contract_drift',
      severity: 'high',
      category: 'quality',
      summary: 'Eval quality thresholds/regression contracts are violated.',
      detail: `threshold_violations=${thresholdViolations};regression_violations=${regressionViolations};quality_ok=${Boolean(quality?.ok)}`,
      automatable: true,
      recommended_commands: [
        'npm run -s ops:eval:quality:metrics:guard',
        'npm run -s ops:eval:quality:gate:v1',
      ],
    });
  }

  const calibrationError = safeNumber(quality?.metrics?.overall?.calibration_error, -1);
  const calibrationMax = safeNumber(thresholds?.global?.calibration_error_max, 0.35);
  const reviewerRows = safeNumber(reviewer?.summary?.feedback_rows, 0);
  if (calibrationError >= 0 && calibrationError >= calibrationMax * 0.8 && reviewerRows >= 5) {
    pushAction({
      id: 'eval_calibration_tuning_recommended',
      severity: 'medium',
      category: 'calibration',
      summary: 'Calibration error is approaching threshold under active reviewer load.',
      detail: `calibration_error=${calibrationError.toFixed(3)};threshold=${calibrationMax.toFixed(3)};reviewer_rows=${reviewerRows}`,
      automatable: true,
      recommended_commands: [
        'npm run -s ops:eval:reviewer:weekly:ingest',
        'npm run -s ops:eval:quality:metrics:guard',
      ],
    });
  }

  const sloAlerts = Array.isArray(slo?.alerts) ? slo.alerts.length : 0;
  if (sloAlerts > 0 || slo?.ok === false) {
    pushAction({
      id: 'eval_monitor_slo_breach',
      severity: 'high',
      category: 'slo',
      summary: 'Eval monitor freshness/uptime SLO is in breach.',
      detail: `alerts=${sloAlerts};slo_ok=${Boolean(slo?.ok)}`,
      automatable: true,
      recommended_commands: ['npm run -s ops:eval:monitor:slo:guard'],
    });
  }

  if (adversarial?.ok === false) {
    pushAction({
      id: 'eval_adversarial_escape_detected',
      severity: 'critical',
      category: 'adversarial',
      summary: 'Adversarial eval guard reports escaping behavior.',
      detail: 'At least one adversarial class failed; block autonomous escalation until fixed.',
      automatable: true,
      recommended_commands: ['npm run -s ops:eval:adversarial:guard'],
    });
  }

  const blockedDrafts = safeNumber(issueFiling?.summary?.blocked_count, 0);
  const readyDrafts = safeNumber(issueFiling?.summary?.ready_to_file_count, 0);
  if (blockedDrafts > 0 && readyDrafts === 0) {
    pushAction({
      id: 'eval_issue_queue_blocked_by_policy',
      severity: 'medium',
      category: 'issue-filing',
      summary: 'Eval issue draft queue is blocked by policy thresholds/approvals.',
      detail: `blocked_drafts=${blockedDrafts};ready_to_file=${readyDrafts}`,
      automatable: false,
      recommended_commands: ['npm run -s ops:eval:issue:filing:guard'],
    });
  }

  const fixFailedCount = safeNumber(issueResolution?.summary?.fix_failed_count, 0);
  if (fixFailedCount > 0) {
    const failingIssueIds = Array.isArray(issueResolution?.linked_issue_status)
      ? issueResolution.linked_issue_status
          .filter((row: any) => row?.fix_failed === true)
          .map((row: any) => cleanText(row?.id || '', 120))
          .filter(Boolean)
      : [];
    pushAction({
      id: 'eval_fix_failed_issues_present',
      severity: 'high',
      category: 'resolution',
      summary: 'Issue-resolution guard reports post-patch failures still open.',
      detail: `fix_failed_count=${fixFailedCount};issue_ids=${failingIssueIds.join(',') || 'none'}`,
      automatable: true,
      recommended_commands: ['npm run -s ops:eval:issue:resolution:guard'],
    });
  }

  const reviewerStatus = cleanText(reviewer?.summary?.status || '', 80);
  if (reviewerStatus === 'awaiting_feedback') {
    pushAction({
      id: 'eval_reviewer_feedback_missing',
      severity: 'medium',
      category: 'reviewer-feedback',
      summary: 'No recent reviewer feedback ingested for calibration.',
      detail: 'Reviewer window has no rows; calibration trend confidence is low.',
      automatable: false,
      recommended_commands: ['npm run -s ops:eval:reviewer:weekly:ingest'],
    });
  }

  const judgeAgreementStatus = cleanText(judgeHuman?.summary?.status || '', 80);
  const judgeAgreementRate = safeNumber(judgeHuman?.summary?.agreement_rate, -1);
  const judgeAgreementMin = safeNumber(
    judgeHuman?.summary?.agreement_min,
    safeNumber(thresholds?.global?.judge_human_agreement_min, 0.7),
  );
  const judgeComparableSamples = safeNumber(judgeHuman?.summary?.comparable_samples, 0);
  const judgeCalibrationReady = Boolean(judgeHuman?.summary?.calibration_ready);
  if (judgeAgreementStatus === 'insufficient_signal') {
    pushAction({
      id: 'eval_judge_human_signal_insufficient',
      severity: 'medium',
      category: 'judge-human-calibration',
      summary: 'Judge-vs-human agreement has insufficient coverage to calibrate confidently.',
      detail: `comparable_samples=${judgeComparableSamples};required_min=${safeNumber(judgeHuman?.summary?.minimum_samples, 0)};agreement_rate=${judgeAgreementRate >= 0 ? judgeAgreementRate.toFixed(3) : 'n/a'}`,
      automatable: false,
      recommended_commands: [
        'npm run -s ops:eval:reviewer:weekly:ingest',
        'npm run -s ops:eval:judge-human:agreement:guard',
      ],
    });
  } else if (
    judgeHuman?.ok === false
    || !judgeCalibrationReady
    || (judgeAgreementRate >= 0 && judgeAgreementRate < judgeAgreementMin)
  ) {
    pushAction({
      id: 'eval_judge_human_agreement_breach',
      severity: 'high',
      category: 'judge-human-calibration',
      summary: 'Judge-vs-human agreement is below threshold or calibration is not ready.',
      detail: `status=${judgeAgreementStatus || 'unknown'};agreement_rate=${judgeAgreementRate >= 0 ? judgeAgreementRate.toFixed(3) : 'n/a'};agreement_min=${judgeAgreementMin.toFixed(3)};calibration_ready=${judgeCalibrationReady}`,
      automatable: true,
      recommended_commands: [
        'npm run -s ops:eval:reviewer:weekly:ingest',
        'npm run -s ops:eval:judge-human:agreement:guard',
      ],
    });
  }

  const qualityGateOk = Boolean(qualityGate?.ok);
  const autonomousEscalationAllowed = Boolean(qualityGate?.summary?.autonomous_escalation_allowed);
  const remainingToUnlock = safeNumber(qualityGate?.summary?.remaining_to_unlock, 0);
  if (!qualityGateOk || !autonomousEscalationAllowed) {
    pushAction({
      id: 'eval_quality_gate_not_ready_for_autonomous_escalation',
      severity: 'info',
      category: 'quality-gate',
      summary: 'Autonomous escalation remains locked pending consecutive clean eval passes.',
      detail: `quality_gate_ok=${qualityGateOk};autonomous_escalation_allowed=${autonomousEscalationAllowed};remaining_to_unlock=${remainingToUnlock}`,
      automatable: true,
      recommended_commands: ['npm run -s ops:eval:quality:gate:v1'],
    });
  }

  if (actions.length === 0) {
    pushAction({
      id: 'eval_autopilot_no_action_required',
      severity: 'info',
      category: 'status',
      summary: 'Eval stack is healthy under current contracts.',
      detail: 'No threshold, SLO, adversarial, filing, resolution, or quality-gate escalations are currently required.',
      automatable: true,
      recommended_commands: ['npm run -s ops:eval:autopilot:full'],
    });
  }
  const actionIds = actions.map((row) => cleanText(row.id || '', 160)).filter(Boolean);
  const actionIdsUnique = new Set(actionIds).size === actionIds.length;
  const actionIdsCanonical = actionIds.every((id) => isCanonicalToken(id, 160));
  const actionSeverityAllowed = actions.every((row) => isAllowedSeverity(String(row.severity || '')));
  const actionCategoryNonEmpty = actions.every((row) => cleanText(row.category || '', 120).length > 0);
  const actionSummaryNonEmpty = actions.every((row) => cleanText(row.summary || '', 240).length > 0);
  const actionDetailNonEmpty = actions.every((row) => cleanText(row.detail || '', 400).length > 0);
  const actionRecommendedCommandsNonEmpty = actions.every(
    (row) => Array.isArray(row.recommended_commands) && row.recommended_commands.length > 0,
  );
  const actionRecommendedCommandsTrimmed = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => {
        const token = cleanText(String(command || ''), 260);
        return token.length > 0 && token === String(command || '').trim() && !token.includes('${');
      }),
  );
  checks.push({
    id: 'eval_autopilot_monitor_payload_object_contract',
    ok: monitorPayloadObject,
    detail: args.monitorPath,
  });
  checks.push({
    id: 'eval_autopilot_quality_payload_object_contract',
    ok: qualityPayloadObject,
    detail: args.qualityPath,
  });
  checks.push({
    id: 'eval_autopilot_slo_payload_object_contract',
    ok: sloPayloadObject,
    detail: args.sloPath,
  });
  checks.push({
    id: 'eval_autopilot_adversarial_payload_object_contract',
    ok: adversarialPayloadObject,
    detail: args.adversarialPath,
  });
  checks.push({
    id: 'eval_autopilot_issue_filing_payload_object_contract',
    ok: issueFilingPayloadObject,
    detail: args.issueFilingPath,
  });
  checks.push({
    id: 'eval_autopilot_issue_resolution_payload_object_contract',
    ok: issueResolutionPayloadObject,
    detail: args.issueResolutionPath,
  });
  checks.push({
    id: 'eval_autopilot_quality_gate_payload_object_contract',
    ok: qualityGatePayloadObject,
    detail: args.qualityGatePath,
  });
  checks.push({
    id: 'eval_autopilot_reviewer_payload_object_contract',
    ok: reviewerPayloadObject,
    detail: args.reviewerPath,
  });
  checks.push({
    id: 'eval_autopilot_judge_human_payload_object_contract',
    ok: judgeHumanPayloadObject,
    detail: args.judgeHumanPath,
  });
  checks.push({
    id: 'eval_autopilot_thresholds_payload_object_contract',
    ok: thresholdsPayloadObject,
    detail: args.thresholdsPath,
  });
  checks.push({
    id: 'eval_autopilot_actions_ids_unique_contract',
    ok: actionIdsUnique,
    detail: `count=${actionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_ids_token_contract',
    ok: actionIdsCanonical,
    detail: `count=${actionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_severity_allowed_contract',
    ok: actionSeverityAllowed,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_category_nonempty_contract',
    ok: actionCategoryNonEmpty,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_summary_nonempty_contract',
    ok: actionSummaryNonEmpty,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_detail_nonempty_contract',
    ok: actionDetailNonEmpty,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_recommended_commands_nonempty_contract',
    ok: actionRecommendedCommandsNonEmpty,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_recommended_commands_trimmed_contract',
    ok: actionRecommendedCommandsTrimmed,
    detail: `count=${actions.length}`,
  });
  const checkIds = checks.map((row) => cleanText(row.id || '', 160)).filter(Boolean);
  const checkIdsUnique = new Set(checkIds).size === checkIds.length;
  const checkIdsCanonical = checkIds.every((id) => isCanonicalToken(id, 160));
  checks.push({
    id: 'eval_autopilot_checks_ids_unique_contract',
    ok: checkIdsUnique,
    detail: `count=${checkIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_ids_token_contract',
    ok: checkIdsCanonical,
    detail: `count=${checkIds.length}`,
  });

  const severityCounts = {
    critical: actions.filter((row) => row.severity === 'critical').length,
    high: actions.filter((row) => row.severity === 'high').length,
    medium: actions.filter((row) => row.severity === 'medium').length,
    low: actions.filter((row) => row.severity === 'low').length,
    info: actions.filter((row) => row.severity === 'info').length,
  };
  const highOrCriticalActions = severityCounts.critical + severityCounts.high;
  const automatableActions = actions.filter((row) => row.automatable).length;
  const checksOk = checks.every((row) => row.ok);
  const autopilotReady = checksOk && highOrCriticalActions === 0;

  const report = {
    type: 'eval_autopilot_guard',
    schema_version: 1,
    generated_at: nowIso,
    ok: autopilotReady,
    checks,
    summary: {
      autopilot_ready: autopilotReady,
      total_actions: actions.length,
      high_or_critical_actions: highOrCriticalActions,
      automatable_actions: automatableActions,
      severity_counts: severityCounts,
    },
    actions,
    sources: {
      monitor: args.monitorPath,
      quality: args.qualityPath,
      slo: args.sloPath,
      adversarial: args.adversarialPath,
      issue_filing: args.issueFilingPath,
      issue_resolution: args.issueResolutionPath,
      quality_gate: args.qualityGatePath,
      reviewer: args.reviewerPath,
      judge_human: args.judgeHumanPath,
      thresholds: args.thresholdsPath,
    },
  };
  const summarySeverityCounts = report.summary?.severity_counts || {};
  const reportSources = report.sources || {};
  const generatedAtIsoParsable = Number.isFinite(Date.parse(String(report.generated_at || '')));
  const summaryTotalActionsMatches = Number(report.summary?.total_actions || 0) === actions.length;
  const summaryHighOrCriticalMatches = Number(report.summary?.high_or_critical_actions || 0) === highOrCriticalActions;
  const summaryAutomatableMatches = Number(report.summary?.automatable_actions || 0) === automatableActions;
  const summarySeverityCriticalMatches = Number(summarySeverityCounts.critical || 0) === severityCounts.critical;
  const summarySeverityHighMatches = Number(summarySeverityCounts.high || 0) === severityCounts.high;
  const summarySeverityMediumMatches = Number(summarySeverityCounts.medium || 0) === severityCounts.medium;
  const summarySeverityLowMatches = Number(summarySeverityCounts.low || 0) === severityCounts.low;
  const summarySeverityInfoMatches = Number(summarySeverityCounts.info || 0) === severityCounts.info;
  const summaryAutopilotReadyFormulaMatches = Boolean(report.summary?.autopilot_ready) === (checksOk && highOrCriticalActions === 0);
  const reportOkMatchesSummaryReady = Boolean(report.ok) === Boolean(report.summary?.autopilot_ready);
  const sourcesReviewerPathMatches = cleanText(String(reportSources.reviewer || ''), 500) === args.reviewerPath;
  const sourcesJudgeHumanPathMatches = cleanText(String(reportSources.judge_human || ''), 500) === args.judgeHumanPath;
  const sourcesThresholdsPathMatches = cleanText(String(reportSources.thresholds || ''), 500) === args.thresholdsPath;
  const summaryTotalActionsNonNegativeInteger = isNonNegativeInteger(report.summary?.total_actions);
  const summaryHighOrCriticalNonNegativeInteger = isNonNegativeInteger(report.summary?.high_or_critical_actions);
  const summaryAutomatableNonNegativeInteger = isNonNegativeInteger(report.summary?.automatable_actions);
  const summarySeverityCriticalNonNegativeInteger = isNonNegativeInteger(summarySeverityCounts.critical);
  const summarySeverityHighNonNegativeInteger = isNonNegativeInteger(summarySeverityCounts.high);
  const summarySeverityMediumNonNegativeInteger = isNonNegativeInteger(summarySeverityCounts.medium);
  const summarySeverityLowNonNegativeInteger = isNonNegativeInteger(summarySeverityCounts.low);
  const summarySeverityInfoNonNegativeInteger = isNonNegativeInteger(summarySeverityCounts.info);
  const summarySeveritySumMatchesTotal =
    Number(summarySeverityCounts.critical || 0)
      + Number(summarySeverityCounts.high || 0)
      + Number(summarySeverityCounts.medium || 0)
      + Number(summarySeverityCounts.low || 0)
      + Number(summarySeverityCounts.info || 0)
    === Number(report.summary?.total_actions || 0);
  const summaryHighOrCriticalWithinTotal =
    Number(report.summary?.high_or_critical_actions || 0)
    <= Number(report.summary?.total_actions || 0);
  const summaryAutomatableWithinTotal =
    Number(report.summary?.automatable_actions || 0)
    <= Number(report.summary?.total_actions || 0);
  const reportActionsArray = Array.isArray(report.actions);
  const reportChecksNonEmpty = Array.isArray(report.checks) && report.checks.length > 0;
  const reportSummaryObject = isPlainObject(report.summary);
  const reportSourcesObject = isPlainObject(report.sources);
  const actionCategoryTokenContract = actions.every((row) => /^[a-z0-9_-]+$/.test(cleanText(row.category || '', 120)));
  const actionRecommendedCommandsUniquePerAction = actions.every((row) => {
    const commands = Array.isArray(row.recommended_commands) ? row.recommended_commands : [];
    const tokens = commands.map((command) => cleanText(String(command || ''), 260));
    return new Set(tokens).size === tokens.length;
  });
  const summarySeverityCountsObject = isPlainObject(summarySeverityCounts);
  const summarySeverityKeysExact =
    Object.keys(summarySeverityCounts).sort().join('|') === 'critical|high|info|low|medium';
  const summaryRequiredKeys = [
    'autopilot_ready',
    'automatable_actions',
    'high_or_critical_actions',
    'severity_counts',
    'total_actions',
  ].every((key) => Object.prototype.hasOwnProperty.call(report.summary || {}, key));
  const sourcesRequiredKeys = [
    'monitor',
    'quality',
    'slo',
    'adversarial',
    'issue_filing',
    'issue_resolution',
    'quality_gate',
    'reviewer',
    'judge_human',
    'thresholds',
  ].every((key) => Object.prototype.hasOwnProperty.call(report.sources || {}, key));
  const sourcesKeysExact =
    Object.keys(reportSources).sort().join('|')
    === 'adversarial|issue_filing|issue_resolution|judge_human|monitor|quality|quality_gate|reviewer|slo|thresholds';
  const reportOkBoolean = isBoolean(report.ok);
  const summaryAutopilotReadyBoolean = isBoolean(report.summary?.autopilot_ready);
  const checkRowsObjectContract = checks.every((row) => isPlainObject(row));
  const checkRowsIdTokenContract = checks.every((row) =>
    isCanonicalToken(cleanText(String((row as any).id || ''), 160), 160),
  );
  const checkRowsIdLowercaseContract = checks.every((row) => {
    const token = cleanText(String((row as any).id || ''), 160);
    return token === token.toLowerCase();
  });
  const checkRowsOkBooleanContract = checks.every((row) => isBoolean((row as any).ok));
  const checkRowsDetailStringContract = checks.every((row) => typeof (row as any).detail === 'string');
  const checkRowsDetailTrimmedContract = checks.every((row) => {
    const raw = String((row as any).detail || '');
    const token = cleanText(raw, 600);
    return token.length > 0 && token === raw.trim();
  });
  const checkRowsDetailNoPlaceholderContract = checks.every(
    (row) => !cleanText(String((row as any).detail || ''), 600).includes('${'),
  );
  const actionsAutomatableBooleanContract = actions.every((row) => isBoolean(row.automatable));
  const actionsSummaryTrimmedContract = actions.every((row) => {
    const raw = String(row.summary || '');
    const token = cleanText(raw, 240);
    return token.length > 0 && token === raw.trim();
  });
  const actionsDetailTrimmedContract = actions.every((row) => {
    const raw = String(row.detail || '');
    const token = cleanText(raw, 400);
    return token.length > 0 && token === raw.trim();
  });
  const actionsSummaryNoPlaceholderContract = actions.every(
    (row) => !cleanText(String(row.summary || ''), 240).includes('${'),
  );
  const actionsDetailNoPlaceholderContract = actions.every(
    (row) => !cleanText(String(row.detail || ''), 400).includes('${'),
  );
  const actionsRecommendedCommandsOpsPrefixContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) =>
        cleanText(String(command || ''), 260).startsWith('npm run -s ops:'),
      ),
  );
  const reportTypeTokenContract = isCanonicalToken(cleanText(String(report.type || ''), 80), 80);
  const reportGeneratedAtTrimmedContract =
    cleanText(String(report.generated_at || ''), 120) === String(report.generated_at || '').trim();
  const reportGeneratedAtNoPlaceholderContract =
    !cleanText(String(report.generated_at || ''), 120).includes('${');
  const reportSchemaVersionIntegerContract = Number.isInteger(Number(report.schema_version));
  const reportActionsCountMatchesContract =
    Array.isArray(report.actions) && report.actions.length === actions.length;
  const reportChecksCountMatchesContract =
    Array.isArray(report.checks) && report.checks.length === checks.length;
  const sourcePathTokens = Object.values(reportSources).map((value) => cleanText(String(value || ''), 500));
  const sourcesPathsCanonicalRelativeContract = sourcePathTokens.every((value) => isCanonicalRelativePath(value));
  const sourcesPathsTrimmedContract = Object.values(reportSources).every((value) => {
    const raw = String(value || '');
    const token = cleanText(raw, 500);
    return token.length > 0 && token === raw.trim();
  });
  const sourcesPathsNoPlaceholderContract =
    sourcePathTokens.every((value) => !value.includes('${'));
  const sourcesPathsUniqueContract = new Set(sourcePathTokens).size === sourcePathTokens.length;
  const sourcesPathsAsciiContract = sourcePathTokens.every((value) => isAsciiPrintable(value, 500));
  const actionsIdsEvalPrefixContract = actions.every((row) =>
    cleanText(String(row.id || ''), 160).startsWith('eval_'),
  );
  const actionsCommandsNoShellJoinersContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => !/[;&|]/.test(cleanText(String(command || ''), 260))),
  );
  const actionsCommandsNoNewlinesContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => !/[\r\n]/.test(String(command || ''))),
  );
  const actionsCommandsAsciiContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => isAsciiPrintable(command, 260)),
  );
  const actionsCommandsNoDestructiveTokensContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => {
        const token = cleanText(String(command || ''), 260).toLowerCase();
        return !token.includes('rm -rf') && !token.includes('git reset --hard') && !token.includes('del /f');
      }),
  );
  const actionsSummaryAsciiContract = actions.every((row) => isAsciiPrintable(row.summary, 240));
  const actionsDetailAsciiContract = actions.every((row) => isAsciiPrintable(row.detail, 400));
  const checksRowsDetailAsciiContract = checks.every((row) => isAsciiPrintable((row as any).detail, 600));
  const checksRowsIdCasefoldUniqueContract = new Set(
    checks.map((row) => cleanText(String((row as any).id || ''), 160).toLowerCase()),
  ).size === checks.length;
  checks.push({
    id: 'eval_autopilot_report_type_contract',
    ok: cleanText(report.type || '', 80) === 'eval_autopilot_guard',
    detail: cleanText(report.type || '', 80),
  });
  checks.push({
    id: 'eval_autopilot_report_schema_version_contract',
    ok: Number(report.schema_version) === 1,
    detail: String(report.schema_version),
  });
  checks.push({
    id: 'eval_autopilot_report_generated_at_iso_contract',
    ok: generatedAtIsoParsable,
    detail: cleanText(report.generated_at || '', 120),
  });
  checks.push({
    id: 'eval_autopilot_summary_total_actions_matches_actions_contract',
    ok: summaryTotalActionsMatches,
    detail: `${Number(report.summary?.total_actions || 0)}|${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_high_or_critical_matches_actions_contract',
    ok: summaryHighOrCriticalMatches,
    detail: `${Number(report.summary?.high_or_critical_actions || 0)}|${highOrCriticalActions}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_automatable_actions_matches_actions_contract',
    ok: summaryAutomatableMatches,
    detail: `${Number(report.summary?.automatable_actions || 0)}|${automatableActions}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_critical_matches_actions_contract',
    ok: summarySeverityCriticalMatches,
    detail: `${Number(summarySeverityCounts.critical || 0)}|${severityCounts.critical}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_high_matches_actions_contract',
    ok: summarySeverityHighMatches,
    detail: `${Number(summarySeverityCounts.high || 0)}|${severityCounts.high}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_medium_matches_actions_contract',
    ok: summarySeverityMediumMatches,
    detail: `${Number(summarySeverityCounts.medium || 0)}|${severityCounts.medium}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_low_matches_actions_contract',
    ok: summarySeverityLowMatches,
    detail: `${Number(summarySeverityCounts.low || 0)}|${severityCounts.low}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_info_matches_actions_contract',
    ok: summarySeverityInfoMatches,
    detail: `${Number(summarySeverityCounts.info || 0)}|${severityCounts.info}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_autopilot_ready_formula_contract',
    ok: summaryAutopilotReadyFormulaMatches,
    detail: `${Boolean(report.summary?.autopilot_ready)}|${checksOk && highOrCriticalActions === 0}`,
  });
  checks.push({
    id: 'eval_autopilot_report_ok_matches_summary_autopilot_ready_contract',
    ok: reportOkMatchesSummaryReady,
    detail: `${Boolean(report.ok)}|${Boolean(report.summary?.autopilot_ready)}`,
  });
  checks.push({
    id: 'eval_autopilot_sources_monitor_path_contract',
    ok: cleanText(String(reportSources.monitor || ''), 500) === args.monitorPath,
    detail: cleanText(String(reportSources.monitor || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_quality_path_contract',
    ok: cleanText(String(reportSources.quality || ''), 500) === args.qualityPath,
    detail: cleanText(String(reportSources.quality || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_slo_path_contract',
    ok: cleanText(String(reportSources.slo || ''), 500) === args.sloPath,
    detail: cleanText(String(reportSources.slo || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_adversarial_path_contract',
    ok: cleanText(String(reportSources.adversarial || ''), 500) === args.adversarialPath,
    detail: cleanText(String(reportSources.adversarial || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_issue_filing_path_contract',
    ok: cleanText(String(reportSources.issue_filing || ''), 500) === args.issueFilingPath,
    detail: cleanText(String(reportSources.issue_filing || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_issue_resolution_path_contract',
    ok: cleanText(String(reportSources.issue_resolution || ''), 500) === args.issueResolutionPath,
    detail: cleanText(String(reportSources.issue_resolution || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_quality_gate_path_contract',
    ok: cleanText(String(reportSources.quality_gate || ''), 500) === args.qualityGatePath,
    detail: cleanText(String(reportSources.quality_gate || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_reviewer_path_contract',
    ok: sourcesReviewerPathMatches,
    detail: cleanText(String(reportSources.reviewer || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_judge_human_path_contract',
    ok: sourcesJudgeHumanPathMatches,
    detail: cleanText(String(reportSources.judge_human || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_sources_thresholds_path_contract',
    ok: sourcesThresholdsPathMatches,
    detail: cleanText(String(reportSources.thresholds || ''), 500),
  });
  checks.push({
    id: 'eval_autopilot_summary_total_actions_nonnegative_integer_contract',
    ok: summaryTotalActionsNonNegativeInteger,
    detail: String(report.summary?.total_actions),
  });
  checks.push({
    id: 'eval_autopilot_summary_high_or_critical_nonnegative_integer_contract',
    ok: summaryHighOrCriticalNonNegativeInteger,
    detail: String(report.summary?.high_or_critical_actions),
  });
  checks.push({
    id: 'eval_autopilot_summary_automatable_nonnegative_integer_contract',
    ok: summaryAutomatableNonNegativeInteger,
    detail: String(report.summary?.automatable_actions),
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_critical_nonnegative_integer_contract',
    ok: summarySeverityCriticalNonNegativeInteger,
    detail: String(summarySeverityCounts.critical),
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_high_nonnegative_integer_contract',
    ok: summarySeverityHighNonNegativeInteger,
    detail: String(summarySeverityCounts.high),
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_medium_nonnegative_integer_contract',
    ok: summarySeverityMediumNonNegativeInteger,
    detail: String(summarySeverityCounts.medium),
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_low_nonnegative_integer_contract',
    ok: summarySeverityLowNonNegativeInteger,
    detail: String(summarySeverityCounts.low),
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_info_nonnegative_integer_contract',
    ok: summarySeverityInfoNonNegativeInteger,
    detail: String(summarySeverityCounts.info),
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_sum_matches_total_contract',
    ok: summarySeveritySumMatchesTotal,
    detail: `${Number(summarySeverityCounts.critical || 0)}+${Number(summarySeverityCounts.high || 0)}+${Number(summarySeverityCounts.medium || 0)}+${Number(summarySeverityCounts.low || 0)}+${Number(summarySeverityCounts.info || 0)}|${Number(report.summary?.total_actions || 0)}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_high_or_critical_within_total_contract',
    ok: summaryHighOrCriticalWithinTotal,
    detail: `${Number(report.summary?.high_or_critical_actions || 0)}|${Number(report.summary?.total_actions || 0)}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_automatable_within_total_contract',
    ok: summaryAutomatableWithinTotal,
    detail: `${Number(report.summary?.automatable_actions || 0)}|${Number(report.summary?.total_actions || 0)}`,
  });
  checks.push({
    id: 'eval_autopilot_report_actions_array_contract',
    ok: reportActionsArray,
    detail: String(report.actions?.length ?? 'n/a'),
  });
  checks.push({
    id: 'eval_autopilot_report_checks_nonempty_contract',
    ok: reportChecksNonEmpty,
    detail: String(report.checks?.length ?? 'n/a'),
  });
  checks.push({
    id: 'eval_autopilot_report_summary_object_contract',
    ok: reportSummaryObject,
    detail: String(report.summary ? Object.keys(report.summary).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_report_sources_object_contract',
    ok: reportSourcesObject,
    detail: String(report.sources ? Object.keys(report.sources).length : 0),
  });
  checks.push({
    id: 'eval_autopilot_actions_category_token_contract',
    ok: actionCategoryTokenContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_recommended_commands_unique_per_action_contract',
    ok: actionRecommendedCommandsUniquePerAction,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_counts_object_contract',
    ok: summarySeverityCountsObject,
    detail: String(Object.keys(summarySeverityCounts).length),
  });
  checks.push({
    id: 'eval_autopilot_summary_severity_keys_exact_contract',
    ok: summarySeverityKeysExact,
    detail: Object.keys(summarySeverityCounts).sort().join('|'),
  });
  checks.push({
    id: 'eval_autopilot_summary_required_keys_contract',
    ok: summaryRequiredKeys,
    detail: Object.keys(report.summary || {}).sort().join('|'),
  });
  checks.push({
    id: 'eval_autopilot_sources_required_keys_contract',
    ok: sourcesRequiredKeys,
    detail: Object.keys(report.sources || {}).sort().join('|'),
  });
  checks.push({
    id: 'eval_autopilot_sources_keys_exact_contract',
    ok: sourcesKeysExact,
    detail: Object.keys(reportSources || {}).sort().join('|'),
  });
  checks.push({
    id: 'eval_autopilot_report_ok_boolean_contract',
    ok: reportOkBoolean,
    detail: typeof report.ok,
  });
  checks.push({
    id: 'eval_autopilot_summary_autopilot_ready_boolean_contract',
    ok: summaryAutopilotReadyBoolean,
    detail: typeof report.summary?.autopilot_ready,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_object_contract',
    ok: checkRowsObjectContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_id_token_contract',
    ok: checkRowsIdTokenContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_id_lowercase_contract',
    ok: checkRowsIdLowercaseContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_ok_boolean_contract',
    ok: checkRowsOkBooleanContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_detail_string_contract',
    ok: checkRowsDetailStringContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_detail_trimmed_contract',
    ok: checkRowsDetailTrimmedContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_detail_no_placeholder_contract',
    ok: checkRowsDetailNoPlaceholderContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_automatable_boolean_contract',
    ok: actionsAutomatableBooleanContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_summary_trimmed_contract',
    ok: actionsSummaryTrimmedContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_detail_trimmed_contract',
    ok: actionsDetailTrimmedContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_summary_no_placeholder_contract',
    ok: actionsSummaryNoPlaceholderContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_detail_no_placeholder_contract',
    ok: actionsDetailNoPlaceholderContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_recommended_commands_ops_prefix_contract',
    ok: actionsRecommendedCommandsOpsPrefixContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_report_type_token_contract',
    ok: reportTypeTokenContract,
    detail: cleanText(String(report.type || ''), 80),
  });
  checks.push({
    id: 'eval_autopilot_report_generated_at_trimmed_contract',
    ok: reportGeneratedAtTrimmedContract,
    detail: cleanText(String(report.generated_at || ''), 120),
  });
  checks.push({
    id: 'eval_autopilot_report_generated_at_no_placeholder_contract',
    ok: reportGeneratedAtNoPlaceholderContract,
    detail: cleanText(String(report.generated_at || ''), 120),
  });
  checks.push({
    id: 'eval_autopilot_report_schema_version_integer_contract',
    ok: reportSchemaVersionIntegerContract,
    detail: String(report.schema_version),
  });
  checks.push({
    id: 'eval_autopilot_report_actions_count_matches_contract',
    ok: reportActionsCountMatchesContract,
    detail: `${Array.isArray(report.actions) ? report.actions.length : 'n/a'}|${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_report_checks_count_matches_contract',
    ok: reportChecksCountMatchesContract,
    detail: `${Array.isArray(report.checks) ? report.checks.length : 'n/a'}|${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_sources_paths_canonical_relative_contract',
    ok: sourcesPathsCanonicalRelativeContract,
    detail: sourcePathTokens.join('|'),
  });
  checks.push({
    id: 'eval_autopilot_sources_paths_trimmed_contract',
    ok: sourcesPathsTrimmedContract,
    detail: sourcePathTokens.join('|'),
  });
  checks.push({
    id: 'eval_autopilot_sources_paths_no_placeholder_contract',
    ok: sourcesPathsNoPlaceholderContract,
    detail: sourcePathTokens.join('|'),
  });
  checks.push({
    id: 'eval_autopilot_sources_paths_unique_contract',
    ok: sourcesPathsUniqueContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_sources_paths_ascii_contract',
    ok: sourcesPathsAsciiContract,
    detail: sourcePathTokens.join('|'),
  });
  checks.push({
    id: 'eval_autopilot_actions_ids_eval_prefix_contract',
    ok: actionsIdsEvalPrefixContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_commands_no_shell_joiners_contract',
    ok: actionsCommandsNoShellJoinersContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_commands_no_newlines_contract',
    ok: actionsCommandsNoNewlinesContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_commands_ascii_contract',
    ok: actionsCommandsAsciiContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_commands_no_destructive_tokens_contract',
    ok: actionsCommandsNoDestructiveTokensContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_summary_ascii_contract',
    ok: actionsSummaryAsciiContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_actions_detail_ascii_contract',
    ok: actionsDetailAsciiContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_detail_ascii_contract',
    ok: checksRowsDetailAsciiContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_checks_rows_id_casefold_unique_contract',
    ok: checksRowsIdCasefoldUniqueContract,
    detail: `count=${checks.length}`,
  });

  const finalChecksNonEmptyContract = checks.length > 0;
  const finalChecksAllRowsObjectContract = checks.every((row) => isPlainObject(row));
  const finalCheckIds = checks.map((row) => cleanText(String((row as any).id || ''), 160));
  const finalCheckIdsUniqueContract = new Set(finalCheckIds).size === finalCheckIds.length;
  const finalCheckIdsCasefoldUniqueContract = new Set(finalCheckIds.map((id) => id.toLowerCase())).size === finalCheckIds.length;
  const finalCheckIdsCanonicalContract = finalCheckIds.every((id) => isCanonicalToken(id, 160));
  const finalChecksOkBooleanContract = checks.every((row) => typeof (row as any).ok === 'boolean');
  const finalChecksDetailTrimmedContract = checks.every((row) => {
    const raw = String((row as any).detail || '');
    const token = cleanText(raw, 600);
    return token === raw.trim();
  });
  const finalChecksDetailNonEmptyContract = checks.every((row) => cleanText(String((row as any).detail || ''), 600).length > 0);
  const finalChecksDetailAsciiContract = checks.every((row) => isAsciiPrintable((row as any).detail, 600));
  const finalChecksDetailNoPlaceholderContract = checks.every(
    (row) => !cleanText(String((row as any).detail || ''), 600).includes('${'),
  );
  const finalActionIds = actions.map((row) => cleanText(String(row.id || ''), 160));
  const finalActionsIdsUniqueContract = new Set(finalActionIds).size === finalActionIds.length;
  const finalActionsIdsCasefoldUniqueContract = new Set(finalActionIds.map((id) => id.toLowerCase())).size === finalActionIds.length;
  const finalActionsIdsCanonicalContract = finalActionIds.every((id) => isCanonicalToken(id, 160));
  const finalActionsSeverityAllowedContract = actions.every((row) => isAllowedSeverity(String(row.severity || '')));
  const finalActionsCategoriesAllowedContract = actions.every((row) => isAllowedActionCategory(String(row.category || '')));
  const finalActionsRecommendedCommandsNonEmptyContract = actions.every(
    (row) => Array.isArray(row.recommended_commands) && row.recommended_commands.length > 0,
  );
  const finalActionsRecommendedCommandsAsciiContract = actions.every(
    (row) => Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => isAsciiPrintable(command, 260)),
  );
  const finalActionsRecommendedCommandsNoPlaceholderContract = actions.every(
    (row) => Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => !cleanText(String(command || ''), 260).includes('${')),
  );
  const finalActionsRecommendedCommandsOpsPrefixContract = actions.every(
    (row) => Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => cleanText(String(command || ''), 260).startsWith('npm run -s ops:')),
  );
  checks.push({
    id: 'eval_autopilot_final_checks_nonempty_contract',
    ok: finalChecksNonEmptyContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_checks_all_rows_object_contract',
    ok: finalChecksAllRowsObjectContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_unique_contract',
    ok: finalCheckIdsUniqueContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_casefold_unique_contract',
    ok: finalCheckIdsCasefoldUniqueContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_canonical_contract',
    ok: finalCheckIdsCanonicalContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_checks_ok_boolean_contract',
    ok: finalChecksOkBooleanContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_checks_detail_trimmed_contract',
    ok: finalChecksDetailTrimmedContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_checks_detail_nonempty_contract',
    ok: finalChecksDetailNonEmptyContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_checks_detail_ascii_contract',
    ok: finalChecksDetailAsciiContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_checks_detail_no_placeholder_contract',
    ok: finalChecksDetailNoPlaceholderContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_ids_unique_contract',
    ok: finalActionsIdsUniqueContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_ids_casefold_unique_contract',
    ok: finalActionsIdsCasefoldUniqueContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_ids_canonical_contract',
    ok: finalActionsIdsCanonicalContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_severity_allowed_contract',
    ok: finalActionsSeverityAllowedContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_categories_allowed_contract',
    ok: finalActionsCategoriesAllowedContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_recommended_commands_nonempty_contract',
    ok: finalActionsRecommendedCommandsNonEmptyContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_recommended_commands_ascii_contract',
    ok: finalActionsRecommendedCommandsAsciiContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_recommended_commands_no_placeholder_contract',
    ok: finalActionsRecommendedCommandsNoPlaceholderContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_recommended_commands_ops_prefix_contract',
    ok: finalActionsRecommendedCommandsOpsPrefixContract,
    detail: `count=${actions.length}`,
  });
  const finalSummarySeverityCountsObjectContract = isPlainObject(report.summary?.severity_counts);
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_object_contract',
    ok: finalSummarySeverityCountsObjectContract,
    detail: String(Object.keys(report.summary?.severity_counts || {}).length),
  });
  const finalActionCategories = actions.map((row) => cleanText(String(row.category || ''), 120));
  const finalActionsCategoryUniqueContract = new Set(finalActionCategories).size === finalActionCategories.length;
  const finalActionsCommandsPerActionMaxThreeContract = actions.every(
    (row) => Array.isArray(row.recommended_commands) && row.recommended_commands.length <= 3,
  );
  const finalActionsCommandsPerActionMinOneContract = actions.every(
    (row) => Array.isArray(row.recommended_commands) && row.recommended_commands.length >= 1,
  );
  const finalActionsCommandsNpmRunPrefixContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => cleanText(String(command || ''), 260).startsWith('npm run -s ')),
  );
  const finalActionsCommandsTrimmedContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => {
        const raw = String(command || '');
        const token = cleanText(raw, 260);
        return token.length > 0 && token === raw.trim();
      }),
  );
  const finalActionsCommandsNoDoubleSpaceContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => !cleanText(String(command || ''), 260).includes('  ')),
  );
  const finalActionsCommandsNoTabContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => !String(command || '').includes('\t')),
  );
  const finalActionsCommandsNoBackslashContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => !cleanText(String(command || ''), 260).includes('\\')),
  );
  const finalActionsCommandsNoRelativePathContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => !cleanText(String(command || ''), 260).includes('..')),
  );
  const finalActionsCommandsNoRedirectContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => !/[<>]/.test(cleanText(String(command || ''), 260))),
  );
  const finalActionsCommandsNoSubshellContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => {
        const token = cleanText(String(command || ''), 260);
        return !token.includes('$(') && !token.includes('`');
      }),
  );
  const finalActionsSummaryLengthContract = actions.every(
    (row) => cleanText(String(row.summary || ''), 240).length <= 240,
  );
  const finalActionsDetailLengthContract = actions.every(
    (row) => cleanText(String(row.detail || ''), 400).length <= 400,
  );
  const finalActionsIdLengthContract = finalActionIds.every((id) => id.length > 0 && id.length <= 120);
  const finalActionsCategoryLengthContract = finalActionCategories.every(
    (category) => category.length > 0 && category.length <= 120,
  );
  const finalActionsCountReasonableContract = actions.length <= 20;
  const finalChecksCountLowerBoundContract = checks.length >= 80;
  const finalChecksCountUpperBoundContract = checks.length <= 600;
  const finalGeneratedAtUtcSuffixContract = cleanText(String(report.generated_at || ''), 120).endsWith('Z');
  checks.push({
    id: 'eval_autopilot_final_actions_category_unique_contract',
    ok: finalActionsCategoryUniqueContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_per_action_max_three_contract',
    ok: finalActionsCommandsPerActionMaxThreeContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_per_action_min_one_contract',
    ok: finalActionsCommandsPerActionMinOneContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_npm_run_prefix_contract',
    ok: finalActionsCommandsNpmRunPrefixContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_trimmed_contract',
    ok: finalActionsCommandsTrimmedContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_no_double_space_contract',
    ok: finalActionsCommandsNoDoubleSpaceContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_no_tab_contract',
    ok: finalActionsCommandsNoTabContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_no_backslash_contract',
    ok: finalActionsCommandsNoBackslashContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_no_relative_path_contract',
    ok: finalActionsCommandsNoRelativePathContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_no_redirect_contract',
    ok: finalActionsCommandsNoRedirectContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_no_subshell_contract',
    ok: finalActionsCommandsNoSubshellContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_summary_length_contract',
    ok: finalActionsSummaryLengthContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_detail_length_contract',
    ok: finalActionsDetailLengthContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_id_length_contract',
    ok: finalActionsIdLengthContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_category_length_contract',
    ok: finalActionsCategoryLengthContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_count_reasonable_contract',
    ok: finalActionsCountReasonableContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_checks_count_lower_bound_contract',
    ok: finalChecksCountLowerBoundContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_checks_count_upper_bound_contract',
    ok: finalChecksCountUpperBoundContract,
    detail: `count=${checks.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_generated_at_utc_suffix_contract',
    ok: finalGeneratedAtUtcSuffixContract,
    detail: cleanText(String(report.generated_at || ''), 120),
  });

  const finalSeverityCounts = {
    critical: actions.filter((row) => row.severity === 'critical').length,
    high: actions.filter((row) => row.severity === 'high').length,
    medium: actions.filter((row) => row.severity === 'medium').length,
    low: actions.filter((row) => row.severity === 'low').length,
    info: actions.filter((row) => row.severity === 'info').length,
  };
  const finalHighOrCriticalActions = finalSeverityCounts.critical + finalSeverityCounts.high;
  const finalChecksOk = checks.every((row) => row.ok);
  const finalAutopilotReady = finalChecksOk && finalHighOrCriticalActions === 0;
  report.ok = finalAutopilotReady;
  report.summary.autopilot_ready = finalAutopilotReady;
  report.summary.total_actions = actions.length;
  report.summary.high_or_critical_actions = finalHighOrCriticalActions;
  report.summary.automatable_actions = actions.filter((row) => row.automatable).length;
  report.summary.severity_counts = finalSeverityCounts;
  report.actions = actions;
  report.checks = checks;

  const outLatestAbs = path.resolve(root, args.outLatestPath);
  const stateAbs = path.resolve(root, args.statePath);
  const markdownAbs = path.resolve(root, args.markdownPath);
  writeJsonArtifact(outLatestAbs, report);
  writeJsonArtifact(stateAbs, report);
  writeTextArtifact(markdownAbs, renderMarkdown(report));

  return emitStructuredResult(report, {
    outPath: path.resolve(root, args.outPath),
    strict: args.strict,
    ok: report.ok,
  });
}

process.exit(run(process.argv.slice(2)));
