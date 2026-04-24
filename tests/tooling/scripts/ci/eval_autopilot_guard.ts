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
  const monitorStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'status')
    || isCanonicalToken(String(monitor?.status), 120);
  const qualityStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'status')
    || isCanonicalToken(String(quality?.status), 120);
  const sloStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'status')
    || isCanonicalToken(String(slo?.status), 120);
  const adversarialStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'status')
    || isCanonicalToken(String(adversarial?.status), 120);
  const issueFilingStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'status')
    || isCanonicalToken(String(issueFiling?.status), 120);
  const issueResolutionStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'status')
    || isCanonicalToken(String(issueResolution?.status), 120);
  const qualityGateStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'status')
    || isCanonicalToken(String(qualityGate?.status), 120);
  const reviewerStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'status')
    || isCanonicalToken(String(reviewer?.status), 120);
  const judgeHumanStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'status')
    || isCanonicalToken(String(judgeHuman?.status), 120);
  const thresholdsStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'status')
    || isCanonicalToken(String(thresholds?.status), 120);
  const qualitySummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(quality?.summary || {}, 'status')
    || isCanonicalToken(String(quality?.summary?.status), 120);
  const sloSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(slo?.summary || {}, 'status')
    || isCanonicalToken(String(slo?.summary?.status), 120);
  const adversarialSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial?.summary || {}, 'status')
    || isCanonicalToken(String(adversarial?.summary?.status), 120);
  const issueFilingSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'status')
    || isCanonicalToken(String(issueFiling?.summary?.status), 120);
  const issueResolutionSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'status')
    || isCanonicalToken(String(issueResolution?.summary?.status), 120);
  const qualityGateSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'status')
    || isCanonicalToken(String(qualityGate?.summary?.status), 120);
  const reviewerSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'status')
    || isCanonicalToken(String(reviewer?.summary?.status), 120);
  const judgeHumanSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'status')
    || isCanonicalToken(String(judgeHuman?.summary?.status), 120);
  const thresholdsSummaryStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.summary || {}, 'status')
    || isCanonicalToken(String(thresholds?.summary?.status), 120);
  const thresholdsGlobalStatusTokenOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.global || {}, 'status')
    || isCanonicalToken(String(thresholds?.global?.status), 120);
  const monitorStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'status') || typeof monitor?.status === 'string';
  const qualityStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'status') || typeof quality?.status === 'string';
  const sloStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'status') || typeof slo?.status === 'string';
  const adversarialStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'status') || typeof adversarial?.status === 'string';
  const issueFilingStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'status') || typeof issueFiling?.status === 'string';
  const issueResolutionStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'status') || typeof issueResolution?.status === 'string';
  const qualityGateStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'status') || typeof qualityGate?.status === 'string';
  const reviewerStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'status') || typeof reviewer?.status === 'string';
  const judgeHumanStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'status') || typeof judgeHuman?.status === 'string';
  const thresholdsStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'status') || typeof thresholds?.status === 'string';
  const monitorSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor?.summary || {}, 'status') || typeof monitor?.summary?.status === 'string';
  const qualitySummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(quality?.summary || {}, 'status') || typeof quality?.summary?.status === 'string';
  const sloSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(slo?.summary || {}, 'status') || typeof slo?.summary?.status === 'string';
  const adversarialSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial?.summary || {}, 'status') || typeof adversarial?.summary?.status === 'string';
  const issueFilingSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'status') || typeof issueFiling?.summary?.status === 'string';
  const issueResolutionSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'status') || typeof issueResolution?.summary?.status === 'string';
  const qualityGateSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'status') || typeof qualityGate?.summary?.status === 'string';
  const reviewerSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'status') || typeof reviewer?.summary?.status === 'string';
  const judgeHumanSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'status') || typeof judgeHuman?.summary?.status === 'string';
  const thresholdsSummaryStatusStringOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.summary || {}, 'status') || typeof thresholds?.summary?.status === 'string';
  const monitorStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'status')
    || String(monitor?.status) === String(monitor?.status).trim();
  const qualityStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'status')
    || String(quality?.status) === String(quality?.status).trim();
  const sloStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'status')
    || String(slo?.status) === String(slo?.status).trim();
  const adversarialStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'status')
    || String(adversarial?.status) === String(adversarial?.status).trim();
  const issueFilingStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'status')
    || String(issueFiling?.status) === String(issueFiling?.status).trim();
  const issueResolutionStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'status')
    || String(issueResolution?.status) === String(issueResolution?.status).trim();
  const qualityGateStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'status')
    || String(qualityGate?.status) === String(qualityGate?.status).trim();
  const reviewerStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'status')
    || String(reviewer?.status) === String(reviewer?.status).trim();
  const judgeHumanStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'status')
    || String(judgeHuman?.status) === String(judgeHuman?.status).trim();
  const thresholdsStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'status')
    || String(thresholds?.status) === String(thresholds?.status).trim();
  const monitorSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor?.summary || {}, 'status')
    || String(monitor?.summary?.status) === String(monitor?.summary?.status).trim();
  const qualitySummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(quality?.summary || {}, 'status')
    || String(quality?.summary?.status) === String(quality?.summary?.status).trim();
  const sloSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(slo?.summary || {}, 'status')
    || String(slo?.summary?.status) === String(slo?.summary?.status).trim();
  const adversarialSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial?.summary || {}, 'status')
    || String(adversarial?.summary?.status) === String(adversarial?.summary?.status).trim();
  const issueFilingSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'status')
    || String(issueFiling?.summary?.status) === String(issueFiling?.summary?.status).trim();
  const issueResolutionSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'status')
    || String(issueResolution?.summary?.status) === String(issueResolution?.summary?.status).trim();
  const qualityGateSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'status')
    || String(qualityGate?.summary?.status) === String(qualityGate?.summary?.status).trim();
  const reviewerSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'status')
    || String(reviewer?.summary?.status) === String(reviewer?.summary?.status).trim();
  const judgeHumanSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'status')
    || String(judgeHuman?.summary?.status) === String(judgeHuman?.summary?.status).trim();
  const thresholdsSummaryStatusTrimmedOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.summary || {}, 'status')
    || String(thresholds?.summary?.status) === String(thresholds?.summary?.status).trim();
  const monitorStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'status')
    || !String(monitor?.status).includes('${');
  const qualityStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'status')
    || !String(quality?.status).includes('${');
  const sloStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'status')
    || !String(slo?.status).includes('${');
  const adversarialStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'status')
    || !String(adversarial?.status).includes('${');
  const issueFilingStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'status')
    || !String(issueFiling?.status).includes('${');
  const issueResolutionStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'status')
    || !String(issueResolution?.status).includes('${');
  const qualityGateStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'status')
    || !String(qualityGate?.status).includes('${');
  const reviewerStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'status')
    || !String(reviewer?.status).includes('${');
  const judgeHumanStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'status')
    || !String(judgeHuman?.status).includes('${');
  const thresholdsStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'status')
    || !String(thresholds?.status).includes('${');
  const monitorSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor?.summary || {}, 'status')
    || !String(monitor?.summary?.status).includes('${');
  const qualitySummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(quality?.summary || {}, 'status')
    || !String(quality?.summary?.status).includes('${');
  const sloSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(slo?.summary || {}, 'status')
    || !String(slo?.summary?.status).includes('${');
  const adversarialSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial?.summary || {}, 'status')
    || !String(adversarial?.summary?.status).includes('${');
  const issueFilingSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'status')
    || !String(issueFiling?.summary?.status).includes('${');
  const issueResolutionSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'status')
    || !String(issueResolution?.summary?.status).includes('${');
  const qualityGateSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'status')
    || !String(qualityGate?.summary?.status).includes('${');
  const reviewerSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'status')
    || !String(reviewer?.summary?.status).includes('${');
  const judgeHumanSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'status')
    || !String(judgeHuman?.summary?.status).includes('${');
  const thresholdsSummaryStatusNoPlaceholderOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.summary || {}, 'status')
    || !String(thresholds?.summary?.status).includes('${');
  const monitorStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'status')
    || String(monitor?.status).length <= 120;
  const qualityStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'status')
    || String(quality?.status).length <= 120;
  const sloStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'status')
    || String(slo?.status).length <= 120;
  const adversarialStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'status')
    || String(adversarial?.status).length <= 120;
  const issueFilingStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'status')
    || String(issueFiling?.status).length <= 120;
  const issueResolutionStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'status')
    || String(issueResolution?.status).length <= 120;
  const qualityGateStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'status')
    || String(qualityGate?.status).length <= 120;
  const reviewerStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'status')
    || String(reviewer?.status).length <= 120;
  const judgeHumanStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'status')
    || String(judgeHuman?.status).length <= 120;
  const thresholdsStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'status')
    || String(thresholds?.status).length <= 120;
  const monitorSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor?.summary || {}, 'status')
    || String(monitor?.summary?.status).length <= 120;
  const qualitySummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(quality?.summary || {}, 'status')
    || String(quality?.summary?.status).length <= 120;
  const sloSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(slo?.summary || {}, 'status')
    || String(slo?.summary?.status).length <= 120;
  const adversarialSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial?.summary || {}, 'status')
    || String(adversarial?.summary?.status).length <= 120;
  const issueFilingSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'status')
    || String(issueFiling?.summary?.status).length <= 120;
  const issueResolutionSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'status')
    || String(issueResolution?.summary?.status).length <= 120;
  const qualityGateSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'status')
    || String(qualityGate?.summary?.status).length <= 120;
  const reviewerSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'status')
    || String(reviewer?.summary?.status).length <= 120;
  const judgeHumanSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'status')
    || String(judgeHuman?.summary?.status).length <= 120;
  const thresholdsSummaryStatusLengthBoundedOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.summary || {}, 'status')
    || String(thresholds?.summary?.status).length <= 120;
  const monitorSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(monitor, 'status') || !Object.prototype.hasOwnProperty.call(monitor?.summary || {}, 'status'))
    || String(monitor?.status) === String(monitor?.summary?.status);
  const qualitySummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(quality, 'status') || !Object.prototype.hasOwnProperty.call(quality?.summary || {}, 'status'))
    || String(quality?.status) === String(quality?.summary?.status);
  const sloSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(slo, 'status') || !Object.prototype.hasOwnProperty.call(slo?.summary || {}, 'status'))
    || String(slo?.status) === String(slo?.summary?.status);
  const adversarialSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(adversarial, 'status') || !Object.prototype.hasOwnProperty.call(adversarial?.summary || {}, 'status'))
    || String(adversarial?.status) === String(adversarial?.summary?.status);
  const issueFilingSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(issueFiling, 'status') || !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'status'))
    || String(issueFiling?.status) === String(issueFiling?.summary?.status);
  const issueResolutionSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(issueResolution, 'status') || !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'status'))
    || String(issueResolution?.status) === String(issueResolution?.summary?.status);
  const qualityGateSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(qualityGate, 'status') || !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'status'))
    || String(qualityGate?.status) === String(qualityGate?.summary?.status);
  const reviewerSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(reviewer, 'status') || !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'status'))
    || String(reviewer?.status) === String(reviewer?.summary?.status);
  const judgeHumanSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(judgeHuman, 'status') || !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'status'))
    || String(judgeHuman?.status) === String(judgeHuman?.summary?.status);
  const thresholdsSummaryStatusConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(thresholds, 'status') || !Object.prototype.hasOwnProperty.call(thresholds?.summary || {}, 'status'))
    || String(thresholds?.status) === String(thresholds?.summary?.status);
  const monitorStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor, 'status') || cleanText(String(monitor?.status), 120).length > 0;
  const qualityStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(quality, 'status') || cleanText(String(quality?.status), 120).length > 0;
  const sloStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(slo, 'status') || cleanText(String(slo?.status), 120).length > 0;
  const adversarialStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial, 'status') || cleanText(String(adversarial?.status), 120).length > 0;
  const issueFilingStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'status') || cleanText(String(issueFiling?.status), 120).length > 0;
  const issueResolutionStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'status') || cleanText(String(issueResolution?.status), 120).length > 0;
  const qualityGateStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate, 'status') || cleanText(String(qualityGate?.status), 120).length > 0;
  const reviewerStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'status') || cleanText(String(reviewer?.status), 120).length > 0;
  const judgeHumanStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'status') || cleanText(String(judgeHuman?.status), 120).length > 0;
  const thresholdsStatusNotEmptyOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'status') || cleanText(String(thresholds?.status), 120).length > 0;
  const issueFilingOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling, 'ok') || typeof issueFiling?.ok === 'boolean';
  const issueResolutionOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution, 'ok') || typeof issueResolution?.ok === 'boolean';
  const reviewerOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer, 'ok') || typeof reviewer?.ok === 'boolean';
  const judgeHumanOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman, 'ok') || typeof judgeHuman?.ok === 'boolean';
  const thresholdsOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds, 'ok') || typeof thresholds?.ok === 'boolean';
  const monitorSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(monitor?.summary || {}, 'ok') || typeof monitor?.summary?.ok === 'boolean';
  const qualitySummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(quality?.summary || {}, 'ok') || typeof quality?.summary?.ok === 'boolean';
  const sloSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(slo?.summary || {}, 'ok') || typeof slo?.summary?.ok === 'boolean';
  const adversarialSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(adversarial?.summary || {}, 'ok') || typeof adversarial?.summary?.ok === 'boolean';
  const issueFilingSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(issueFiling?.summary || {}, 'ok') || typeof issueFiling?.summary?.ok === 'boolean';
  const issueResolutionSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(issueResolution?.summary || {}, 'ok') || typeof issueResolution?.summary?.ok === 'boolean';
  const qualityGateSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(qualityGate?.summary || {}, 'ok') || typeof qualityGate?.summary?.ok === 'boolean';
  const reviewerSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(reviewer?.summary || {}, 'ok') || typeof reviewer?.summary?.ok === 'boolean';
  const judgeHumanSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(judgeHuman?.summary || {}, 'ok') || typeof judgeHuman?.summary?.ok === 'boolean';
  const thresholdsSummaryOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.summary || {}, 'ok') || typeof thresholds?.summary?.ok === 'boolean';
  const thresholdsGlobalOkBooleanOrMissing =
    !Object.prototype.hasOwnProperty.call(thresholds?.global || {}, 'ok') || typeof thresholds?.global?.ok === 'boolean';
  const monitorSummaryOkConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(monitor, 'ok') || !Object.prototype.hasOwnProperty.call(monitor?.summary || {}, 'ok'))
    || monitor?.ok === monitor?.summary?.ok;
  const qualitySummaryOkConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(quality, 'ok') || !Object.prototype.hasOwnProperty.call(quality?.summary || {}, 'ok'))
    || quality?.ok === quality?.summary?.ok;
  const sloSummaryOkConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(slo, 'ok') || !Object.prototype.hasOwnProperty.call(slo?.summary || {}, 'ok'))
    || slo?.ok === slo?.summary?.ok;
  const adversarialSummaryOkConsistentWithRootOrMissing =
    (!Object.prototype.hasOwnProperty.call(adversarial, 'ok') || !Object.prototype.hasOwnProperty.call(adversarial?.summary || {}, 'ok'))
    || adversarial?.ok === adversarial?.summary?.ok;

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
    { id: 'eval_autopilot_input_monitor_status_token_or_missing_contract', ok: monitorStatusTokenOrMissing, detail: String(monitor?.status) },
    { id: 'eval_autopilot_input_quality_status_token_or_missing_contract', ok: qualityStatusTokenOrMissing, detail: String(quality?.status) },
    { id: 'eval_autopilot_input_slo_status_token_or_missing_contract', ok: sloStatusTokenOrMissing, detail: String(slo?.status) },
    { id: 'eval_autopilot_input_adversarial_status_token_or_missing_contract', ok: adversarialStatusTokenOrMissing, detail: String(adversarial?.status) },
    { id: 'eval_autopilot_input_issue_filing_status_token_or_missing_contract', ok: issueFilingStatusTokenOrMissing, detail: String(issueFiling?.status) },
    { id: 'eval_autopilot_input_issue_resolution_status_token_or_missing_contract', ok: issueResolutionStatusTokenOrMissing, detail: String(issueResolution?.status) },
    { id: 'eval_autopilot_input_quality_gate_status_token_or_missing_contract', ok: qualityGateStatusTokenOrMissing, detail: String(qualityGate?.status) },
    { id: 'eval_autopilot_input_reviewer_status_root_token_or_missing_contract', ok: reviewerStatusTokenOrMissing, detail: String(reviewer?.status) },
    { id: 'eval_autopilot_input_judge_human_status_token_or_missing_contract', ok: judgeHumanStatusTokenOrMissing, detail: String(judgeHuman?.status) },
    { id: 'eval_autopilot_input_thresholds_status_token_or_missing_contract', ok: thresholdsStatusTokenOrMissing, detail: String(thresholds?.status) },
    { id: 'eval_autopilot_input_quality_summary_status_token_or_missing_contract', ok: qualitySummaryStatusTokenOrMissing, detail: String(quality?.summary?.status) },
    { id: 'eval_autopilot_input_slo_summary_status_token_or_missing_contract', ok: sloSummaryStatusTokenOrMissing, detail: String(slo?.summary?.status) },
    { id: 'eval_autopilot_input_adversarial_summary_status_token_or_missing_contract', ok: adversarialSummaryStatusTokenOrMissing, detail: String(adversarial?.summary?.status) },
    { id: 'eval_autopilot_input_issue_filing_summary_status_token_or_missing_contract', ok: issueFilingSummaryStatusTokenOrMissing, detail: String(issueFiling?.summary?.status) },
    { id: 'eval_autopilot_input_issue_resolution_summary_status_token_or_missing_contract', ok: issueResolutionSummaryStatusTokenOrMissing, detail: String(issueResolution?.summary?.status) },
    { id: 'eval_autopilot_input_quality_gate_summary_status_token_or_missing_contract', ok: qualityGateSummaryStatusTokenOrMissing, detail: String(qualityGate?.summary?.status) },
    { id: 'eval_autopilot_input_reviewer_summary_status_token_or_missing_contract', ok: reviewerSummaryStatusTokenOrMissing, detail: String(reviewer?.summary?.status) },
    { id: 'eval_autopilot_input_judge_human_summary_status_token_or_missing_contract', ok: judgeHumanSummaryStatusTokenOrMissing, detail: String(judgeHuman?.summary?.status) },
    { id: 'eval_autopilot_input_thresholds_summary_status_token_or_missing_contract', ok: thresholdsSummaryStatusTokenOrMissing, detail: String(thresholds?.summary?.status) },
    { id: 'eval_autopilot_input_thresholds_global_status_token_or_missing_contract', ok: thresholdsGlobalStatusTokenOrMissing, detail: String(thresholds?.global?.status) },
    { id: 'eval_autopilot_input_monitor_status_string_or_missing_contract', ok: monitorStatusStringOrMissing, detail: String(monitor?.status) },
    { id: 'eval_autopilot_input_quality_status_string_or_missing_contract', ok: qualityStatusStringOrMissing, detail: String(quality?.status) },
    { id: 'eval_autopilot_input_slo_status_string_or_missing_contract', ok: sloStatusStringOrMissing, detail: String(slo?.status) },
    { id: 'eval_autopilot_input_adversarial_status_string_or_missing_contract', ok: adversarialStatusStringOrMissing, detail: String(adversarial?.status) },
    { id: 'eval_autopilot_input_issue_filing_status_string_or_missing_contract', ok: issueFilingStatusStringOrMissing, detail: String(issueFiling?.status) },
    { id: 'eval_autopilot_input_issue_resolution_status_string_or_missing_contract', ok: issueResolutionStatusStringOrMissing, detail: String(issueResolution?.status) },
    { id: 'eval_autopilot_input_quality_gate_status_string_or_missing_contract', ok: qualityGateStatusStringOrMissing, detail: String(qualityGate?.status) },
    { id: 'eval_autopilot_input_reviewer_status_string_or_missing_contract', ok: reviewerStatusStringOrMissing, detail: String(reviewer?.status) },
    { id: 'eval_autopilot_input_judge_human_status_string_or_missing_contract', ok: judgeHumanStatusStringOrMissing, detail: String(judgeHuman?.status) },
    { id: 'eval_autopilot_input_thresholds_status_string_or_missing_contract', ok: thresholdsStatusStringOrMissing, detail: String(thresholds?.status) },
    { id: 'eval_autopilot_input_monitor_summary_status_string_or_missing_contract', ok: monitorSummaryStatusStringOrMissing, detail: String(monitor?.summary?.status) },
    { id: 'eval_autopilot_input_quality_summary_status_string_or_missing_contract', ok: qualitySummaryStatusStringOrMissing, detail: String(quality?.summary?.status) },
    { id: 'eval_autopilot_input_slo_summary_status_string_or_missing_contract', ok: sloSummaryStatusStringOrMissing, detail: String(slo?.summary?.status) },
    { id: 'eval_autopilot_input_adversarial_summary_status_string_or_missing_contract', ok: adversarialSummaryStatusStringOrMissing, detail: String(adversarial?.summary?.status) },
    { id: 'eval_autopilot_input_issue_filing_summary_status_string_or_missing_contract', ok: issueFilingSummaryStatusStringOrMissing, detail: String(issueFiling?.summary?.status) },
    { id: 'eval_autopilot_input_issue_resolution_summary_status_string_or_missing_contract', ok: issueResolutionSummaryStatusStringOrMissing, detail: String(issueResolution?.summary?.status) },
    { id: 'eval_autopilot_input_quality_gate_summary_status_string_or_missing_contract', ok: qualityGateSummaryStatusStringOrMissing, detail: String(qualityGate?.summary?.status) },
    { id: 'eval_autopilot_input_reviewer_summary_status_string_or_missing_contract', ok: reviewerSummaryStatusStringOrMissing, detail: String(reviewer?.summary?.status) },
    { id: 'eval_autopilot_input_judge_human_summary_status_string_or_missing_contract', ok: judgeHumanSummaryStatusStringOrMissing, detail: String(judgeHuman?.summary?.status) },
    { id: 'eval_autopilot_input_thresholds_summary_status_string_or_missing_contract', ok: thresholdsSummaryStatusStringOrMissing, detail: String(thresholds?.summary?.status) },
    { id: 'eval_autopilot_input_monitor_status_trimmed_or_missing_contract', ok: monitorStatusTrimmedOrMissing, detail: String(monitor?.status) },
    { id: 'eval_autopilot_input_quality_status_trimmed_or_missing_contract', ok: qualityStatusTrimmedOrMissing, detail: String(quality?.status) },
    { id: 'eval_autopilot_input_slo_status_trimmed_or_missing_contract', ok: sloStatusTrimmedOrMissing, detail: String(slo?.status) },
    { id: 'eval_autopilot_input_adversarial_status_trimmed_or_missing_contract', ok: adversarialStatusTrimmedOrMissing, detail: String(adversarial?.status) },
    { id: 'eval_autopilot_input_issue_filing_status_trimmed_or_missing_contract', ok: issueFilingStatusTrimmedOrMissing, detail: String(issueFiling?.status) },
    { id: 'eval_autopilot_input_issue_resolution_status_trimmed_or_missing_contract', ok: issueResolutionStatusTrimmedOrMissing, detail: String(issueResolution?.status) },
    { id: 'eval_autopilot_input_quality_gate_status_trimmed_or_missing_contract', ok: qualityGateStatusTrimmedOrMissing, detail: String(qualityGate?.status) },
    { id: 'eval_autopilot_input_reviewer_status_trimmed_or_missing_contract', ok: reviewerStatusTrimmedOrMissing, detail: String(reviewer?.status) },
    { id: 'eval_autopilot_input_judge_human_status_trimmed_or_missing_contract', ok: judgeHumanStatusTrimmedOrMissing, detail: String(judgeHuman?.status) },
    { id: 'eval_autopilot_input_thresholds_status_trimmed_or_missing_contract', ok: thresholdsStatusTrimmedOrMissing, detail: String(thresholds?.status) },
    { id: 'eval_autopilot_input_monitor_summary_status_trimmed_or_missing_contract', ok: monitorSummaryStatusTrimmedOrMissing, detail: String(monitor?.summary?.status) },
    { id: 'eval_autopilot_input_quality_summary_status_trimmed_or_missing_contract', ok: qualitySummaryStatusTrimmedOrMissing, detail: String(quality?.summary?.status) },
    { id: 'eval_autopilot_input_slo_summary_status_trimmed_or_missing_contract', ok: sloSummaryStatusTrimmedOrMissing, detail: String(slo?.summary?.status) },
    { id: 'eval_autopilot_input_adversarial_summary_status_trimmed_or_missing_contract', ok: adversarialSummaryStatusTrimmedOrMissing, detail: String(adversarial?.summary?.status) },
    { id: 'eval_autopilot_input_issue_filing_summary_status_trimmed_or_missing_contract', ok: issueFilingSummaryStatusTrimmedOrMissing, detail: String(issueFiling?.summary?.status) },
    { id: 'eval_autopilot_input_issue_resolution_summary_status_trimmed_or_missing_contract', ok: issueResolutionSummaryStatusTrimmedOrMissing, detail: String(issueResolution?.summary?.status) },
    { id: 'eval_autopilot_input_quality_gate_summary_status_trimmed_or_missing_contract', ok: qualityGateSummaryStatusTrimmedOrMissing, detail: String(qualityGate?.summary?.status) },
    { id: 'eval_autopilot_input_reviewer_summary_status_trimmed_or_missing_contract', ok: reviewerSummaryStatusTrimmedOrMissing, detail: String(reviewer?.summary?.status) },
    { id: 'eval_autopilot_input_judge_human_summary_status_trimmed_or_missing_contract', ok: judgeHumanSummaryStatusTrimmedOrMissing, detail: String(judgeHuman?.summary?.status) },
    { id: 'eval_autopilot_input_thresholds_summary_status_trimmed_or_missing_contract', ok: thresholdsSummaryStatusTrimmedOrMissing, detail: String(thresholds?.summary?.status) },
    { id: 'eval_autopilot_input_monitor_status_no_placeholder_or_missing_contract', ok: monitorStatusNoPlaceholderOrMissing, detail: String(monitor?.status) },
    { id: 'eval_autopilot_input_quality_status_no_placeholder_or_missing_contract', ok: qualityStatusNoPlaceholderOrMissing, detail: String(quality?.status) },
    { id: 'eval_autopilot_input_slo_status_no_placeholder_or_missing_contract', ok: sloStatusNoPlaceholderOrMissing, detail: String(slo?.status) },
    { id: 'eval_autopilot_input_adversarial_status_no_placeholder_or_missing_contract', ok: adversarialStatusNoPlaceholderOrMissing, detail: String(adversarial?.status) },
    { id: 'eval_autopilot_input_issue_filing_status_no_placeholder_or_missing_contract', ok: issueFilingStatusNoPlaceholderOrMissing, detail: String(issueFiling?.status) },
    { id: 'eval_autopilot_input_issue_resolution_status_no_placeholder_or_missing_contract', ok: issueResolutionStatusNoPlaceholderOrMissing, detail: String(issueResolution?.status) },
    { id: 'eval_autopilot_input_quality_gate_status_no_placeholder_or_missing_contract', ok: qualityGateStatusNoPlaceholderOrMissing, detail: String(qualityGate?.status) },
    { id: 'eval_autopilot_input_reviewer_status_no_placeholder_or_missing_contract', ok: reviewerStatusNoPlaceholderOrMissing, detail: String(reviewer?.status) },
    { id: 'eval_autopilot_input_judge_human_status_no_placeholder_or_missing_contract', ok: judgeHumanStatusNoPlaceholderOrMissing, detail: String(judgeHuman?.status) },
    { id: 'eval_autopilot_input_thresholds_status_no_placeholder_or_missing_contract', ok: thresholdsStatusNoPlaceholderOrMissing, detail: String(thresholds?.status) },
    { id: 'eval_autopilot_input_monitor_summary_status_no_placeholder_or_missing_contract', ok: monitorSummaryStatusNoPlaceholderOrMissing, detail: String(monitor?.summary?.status) },
    { id: 'eval_autopilot_input_quality_summary_status_no_placeholder_or_missing_contract', ok: qualitySummaryStatusNoPlaceholderOrMissing, detail: String(quality?.summary?.status) },
    { id: 'eval_autopilot_input_slo_summary_status_no_placeholder_or_missing_contract', ok: sloSummaryStatusNoPlaceholderOrMissing, detail: String(slo?.summary?.status) },
    { id: 'eval_autopilot_input_adversarial_summary_status_no_placeholder_or_missing_contract', ok: adversarialSummaryStatusNoPlaceholderOrMissing, detail: String(adversarial?.summary?.status) },
    { id: 'eval_autopilot_input_issue_filing_summary_status_no_placeholder_or_missing_contract', ok: issueFilingSummaryStatusNoPlaceholderOrMissing, detail: String(issueFiling?.summary?.status) },
    { id: 'eval_autopilot_input_issue_resolution_summary_status_no_placeholder_or_missing_contract', ok: issueResolutionSummaryStatusNoPlaceholderOrMissing, detail: String(issueResolution?.summary?.status) },
    { id: 'eval_autopilot_input_quality_gate_summary_status_no_placeholder_or_missing_contract', ok: qualityGateSummaryStatusNoPlaceholderOrMissing, detail: String(qualityGate?.summary?.status) },
    { id: 'eval_autopilot_input_reviewer_summary_status_no_placeholder_or_missing_contract', ok: reviewerSummaryStatusNoPlaceholderOrMissing, detail: String(reviewer?.summary?.status) },
    { id: 'eval_autopilot_input_judge_human_summary_status_no_placeholder_or_missing_contract', ok: judgeHumanSummaryStatusNoPlaceholderOrMissing, detail: String(judgeHuman?.summary?.status) },
    { id: 'eval_autopilot_input_thresholds_summary_status_no_placeholder_or_missing_contract', ok: thresholdsSummaryStatusNoPlaceholderOrMissing, detail: String(thresholds?.summary?.status) },
    { id: 'eval_autopilot_input_monitor_status_length_bounded_or_missing_contract', ok: monitorStatusLengthBoundedOrMissing, detail: String(monitor?.status) },
    { id: 'eval_autopilot_input_quality_status_length_bounded_or_missing_contract', ok: qualityStatusLengthBoundedOrMissing, detail: String(quality?.status) },
    { id: 'eval_autopilot_input_slo_status_length_bounded_or_missing_contract', ok: sloStatusLengthBoundedOrMissing, detail: String(slo?.status) },
    { id: 'eval_autopilot_input_adversarial_status_length_bounded_or_missing_contract', ok: adversarialStatusLengthBoundedOrMissing, detail: String(adversarial?.status) },
    { id: 'eval_autopilot_input_issue_filing_status_length_bounded_or_missing_contract', ok: issueFilingStatusLengthBoundedOrMissing, detail: String(issueFiling?.status) },
    { id: 'eval_autopilot_input_issue_resolution_status_length_bounded_or_missing_contract', ok: issueResolutionStatusLengthBoundedOrMissing, detail: String(issueResolution?.status) },
    { id: 'eval_autopilot_input_quality_gate_status_length_bounded_or_missing_contract', ok: qualityGateStatusLengthBoundedOrMissing, detail: String(qualityGate?.status) },
    { id: 'eval_autopilot_input_reviewer_status_length_bounded_or_missing_contract', ok: reviewerStatusLengthBoundedOrMissing, detail: String(reviewer?.status) },
    { id: 'eval_autopilot_input_judge_human_status_length_bounded_or_missing_contract', ok: judgeHumanStatusLengthBoundedOrMissing, detail: String(judgeHuman?.status) },
    { id: 'eval_autopilot_input_thresholds_status_length_bounded_or_missing_contract', ok: thresholdsStatusLengthBoundedOrMissing, detail: String(thresholds?.status) },
    { id: 'eval_autopilot_input_monitor_summary_status_length_bounded_or_missing_contract', ok: monitorSummaryStatusLengthBoundedOrMissing, detail: String(monitor?.summary?.status) },
    { id: 'eval_autopilot_input_quality_summary_status_length_bounded_or_missing_contract', ok: qualitySummaryStatusLengthBoundedOrMissing, detail: String(quality?.summary?.status) },
    { id: 'eval_autopilot_input_slo_summary_status_length_bounded_or_missing_contract', ok: sloSummaryStatusLengthBoundedOrMissing, detail: String(slo?.summary?.status) },
    { id: 'eval_autopilot_input_adversarial_summary_status_length_bounded_or_missing_contract', ok: adversarialSummaryStatusLengthBoundedOrMissing, detail: String(adversarial?.summary?.status) },
    { id: 'eval_autopilot_input_issue_filing_summary_status_length_bounded_or_missing_contract', ok: issueFilingSummaryStatusLengthBoundedOrMissing, detail: String(issueFiling?.summary?.status) },
    { id: 'eval_autopilot_input_issue_resolution_summary_status_length_bounded_or_missing_contract', ok: issueResolutionSummaryStatusLengthBoundedOrMissing, detail: String(issueResolution?.summary?.status) },
    { id: 'eval_autopilot_input_quality_gate_summary_status_length_bounded_or_missing_contract', ok: qualityGateSummaryStatusLengthBoundedOrMissing, detail: String(qualityGate?.summary?.status) },
    { id: 'eval_autopilot_input_reviewer_summary_status_length_bounded_or_missing_contract', ok: reviewerSummaryStatusLengthBoundedOrMissing, detail: String(reviewer?.summary?.status) },
    { id: 'eval_autopilot_input_judge_human_summary_status_length_bounded_or_missing_contract', ok: judgeHumanSummaryStatusLengthBoundedOrMissing, detail: String(judgeHuman?.summary?.status) },
    { id: 'eval_autopilot_input_thresholds_summary_status_length_bounded_or_missing_contract', ok: thresholdsSummaryStatusLengthBoundedOrMissing, detail: String(thresholds?.summary?.status) },
    { id: 'eval_autopilot_input_monitor_summary_status_consistent_with_root_or_missing_contract', ok: monitorSummaryStatusConsistentWithRootOrMissing, detail: `${String(monitor?.status)}|${String(monitor?.summary?.status)}` },
    { id: 'eval_autopilot_input_quality_summary_status_consistent_with_root_or_missing_contract', ok: qualitySummaryStatusConsistentWithRootOrMissing, detail: `${String(quality?.status)}|${String(quality?.summary?.status)}` },
    { id: 'eval_autopilot_input_slo_summary_status_consistent_with_root_or_missing_contract', ok: sloSummaryStatusConsistentWithRootOrMissing, detail: `${String(slo?.status)}|${String(slo?.summary?.status)}` },
    { id: 'eval_autopilot_input_adversarial_summary_status_consistent_with_root_or_missing_contract', ok: adversarialSummaryStatusConsistentWithRootOrMissing, detail: `${String(adversarial?.status)}|${String(adversarial?.summary?.status)}` },
    { id: 'eval_autopilot_input_issue_filing_summary_status_consistent_with_root_or_missing_contract', ok: issueFilingSummaryStatusConsistentWithRootOrMissing, detail: `${String(issueFiling?.status)}|${String(issueFiling?.summary?.status)}` },
    { id: 'eval_autopilot_input_issue_resolution_summary_status_consistent_with_root_or_missing_contract', ok: issueResolutionSummaryStatusConsistentWithRootOrMissing, detail: `${String(issueResolution?.status)}|${String(issueResolution?.summary?.status)}` },
    { id: 'eval_autopilot_input_quality_gate_summary_status_consistent_with_root_or_missing_contract', ok: qualityGateSummaryStatusConsistentWithRootOrMissing, detail: `${String(qualityGate?.status)}|${String(qualityGate?.summary?.status)}` },
    { id: 'eval_autopilot_input_reviewer_summary_status_consistent_with_root_or_missing_contract', ok: reviewerSummaryStatusConsistentWithRootOrMissing, detail: `${String(reviewer?.status)}|${String(reviewer?.summary?.status)}` },
    { id: 'eval_autopilot_input_judge_human_summary_status_consistent_with_root_or_missing_contract', ok: judgeHumanSummaryStatusConsistentWithRootOrMissing, detail: `${String(judgeHuman?.status)}|${String(judgeHuman?.summary?.status)}` },
    { id: 'eval_autopilot_input_thresholds_summary_status_consistent_with_root_or_missing_contract', ok: thresholdsSummaryStatusConsistentWithRootOrMissing, detail: `${String(thresholds?.status)}|${String(thresholds?.summary?.status)}` },
    { id: 'eval_autopilot_input_monitor_status_not_empty_or_missing_contract', ok: monitorStatusNotEmptyOrMissing, detail: String(monitor?.status) },
    { id: 'eval_autopilot_input_quality_status_not_empty_or_missing_contract', ok: qualityStatusNotEmptyOrMissing, detail: String(quality?.status) },
    { id: 'eval_autopilot_input_slo_status_not_empty_or_missing_contract', ok: sloStatusNotEmptyOrMissing, detail: String(slo?.status) },
    { id: 'eval_autopilot_input_adversarial_status_not_empty_or_missing_contract', ok: adversarialStatusNotEmptyOrMissing, detail: String(adversarial?.status) },
    { id: 'eval_autopilot_input_issue_filing_status_not_empty_or_missing_contract', ok: issueFilingStatusNotEmptyOrMissing, detail: String(issueFiling?.status) },
    { id: 'eval_autopilot_input_issue_resolution_status_not_empty_or_missing_contract', ok: issueResolutionStatusNotEmptyOrMissing, detail: String(issueResolution?.status) },
    { id: 'eval_autopilot_input_quality_gate_status_not_empty_or_missing_contract', ok: qualityGateStatusNotEmptyOrMissing, detail: String(qualityGate?.status) },
    { id: 'eval_autopilot_input_reviewer_status_not_empty_or_missing_contract', ok: reviewerStatusNotEmptyOrMissing, detail: String(reviewer?.status) },
    { id: 'eval_autopilot_input_judge_human_status_not_empty_or_missing_contract', ok: judgeHumanStatusNotEmptyOrMissing, detail: String(judgeHuman?.status) },
    { id: 'eval_autopilot_input_thresholds_status_not_empty_or_missing_contract', ok: thresholdsStatusNotEmptyOrMissing, detail: String(thresholds?.status) },
    { id: 'eval_autopilot_input_issue_filing_ok_boolean_or_missing_contract', ok: issueFilingOkBooleanOrMissing, detail: String(issueFiling?.ok) },
    { id: 'eval_autopilot_input_issue_resolution_ok_boolean_or_missing_contract', ok: issueResolutionOkBooleanOrMissing, detail: String(issueResolution?.ok) },
    { id: 'eval_autopilot_input_reviewer_ok_boolean_or_missing_contract', ok: reviewerOkBooleanOrMissing, detail: String(reviewer?.ok) },
    { id: 'eval_autopilot_input_judge_human_ok_boolean_or_missing_contract', ok: judgeHumanOkBooleanOrMissing, detail: String(judgeHuman?.ok) },
    { id: 'eval_autopilot_input_thresholds_ok_boolean_or_missing_contract', ok: thresholdsOkBooleanOrMissing, detail: String(thresholds?.ok) },
    { id: 'eval_autopilot_input_monitor_summary_ok_boolean_or_missing_contract', ok: monitorSummaryOkBooleanOrMissing, detail: String(monitor?.summary?.ok) },
    { id: 'eval_autopilot_input_quality_summary_ok_boolean_or_missing_contract', ok: qualitySummaryOkBooleanOrMissing, detail: String(quality?.summary?.ok) },
    { id: 'eval_autopilot_input_slo_summary_ok_boolean_or_missing_contract', ok: sloSummaryOkBooleanOrMissing, detail: String(slo?.summary?.ok) },
    { id: 'eval_autopilot_input_adversarial_summary_ok_boolean_or_missing_contract', ok: adversarialSummaryOkBooleanOrMissing, detail: String(adversarial?.summary?.ok) },
    { id: 'eval_autopilot_input_issue_filing_summary_ok_boolean_or_missing_contract', ok: issueFilingSummaryOkBooleanOrMissing, detail: String(issueFiling?.summary?.ok) },
    { id: 'eval_autopilot_input_issue_resolution_summary_ok_boolean_or_missing_contract', ok: issueResolutionSummaryOkBooleanOrMissing, detail: String(issueResolution?.summary?.ok) },
    { id: 'eval_autopilot_input_quality_gate_summary_ok_boolean_or_missing_contract', ok: qualityGateSummaryOkBooleanOrMissing, detail: String(qualityGate?.summary?.ok) },
    { id: 'eval_autopilot_input_reviewer_summary_ok_boolean_or_missing_contract', ok: reviewerSummaryOkBooleanOrMissing, detail: String(reviewer?.summary?.ok) },
    { id: 'eval_autopilot_input_judge_human_summary_ok_boolean_or_missing_contract', ok: judgeHumanSummaryOkBooleanOrMissing, detail: String(judgeHuman?.summary?.ok) },
    { id: 'eval_autopilot_input_thresholds_summary_ok_boolean_or_missing_contract', ok: thresholdsSummaryOkBooleanOrMissing, detail: String(thresholds?.summary?.ok) },
    { id: 'eval_autopilot_input_thresholds_global_ok_boolean_or_missing_contract', ok: thresholdsGlobalOkBooleanOrMissing, detail: String(thresholds?.global?.ok) },
    { id: 'eval_autopilot_input_monitor_summary_ok_consistent_with_root_or_missing_contract', ok: monitorSummaryOkConsistentWithRootOrMissing, detail: `${String(monitor?.ok)}|${String(monitor?.summary?.ok)}` },
    { id: 'eval_autopilot_input_quality_summary_ok_consistent_with_root_or_missing_contract', ok: qualitySummaryOkConsistentWithRootOrMissing, detail: `${String(quality?.ok)}|${String(quality?.summary?.ok)}` },
    { id: 'eval_autopilot_input_slo_summary_ok_consistent_with_root_or_missing_contract', ok: sloSummaryOkConsistentWithRootOrMissing, detail: `${String(slo?.ok)}|${String(slo?.summary?.ok)}` },
    { id: 'eval_autopilot_input_adversarial_summary_ok_consistent_with_root_or_missing_contract', ok: adversarialSummaryOkConsistentWithRootOrMissing, detail: `${String(adversarial?.ok)}|${String(adversarial?.summary?.ok)}` },
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

  const finalCheckIdsEvalPrefixContract = finalCheckIds.every((id) => id.startsWith('eval_autopilot_'));
  const finalCheckIdsContractSuffixContract = finalCheckIds.every((id) => id.endsWith('_contract'));
  const finalCheckIdsLengthBoundedContract = finalCheckIds.every((id) => id.length <= 160);
  const finalCheckIdsNoPlaceholderContract = finalCheckIds.every((id) => !id.includes('${'));
  const finalCheckIdsAsciiContract = finalCheckIds.every((id) => isAsciiPrintable(id, 160));
  const finalActionIdsNoPlaceholderContract = finalActionIds.every((id) => !id.includes('${'));
  const finalActionIdsAsciiContract = finalActionIds.every((id) => isAsciiPrintable(id, 160));
  const finalActionIdsNoDoubleSpaceContract = finalActionIds.every((id) => !id.includes('  '));
  const finalActionIdsNonEmptyContract = finalActionIds.every((id) => id.length > 0);
  const finalActionsSummaryNonEmptyContract = actions.every((row) => cleanText(String(row.summary || ''), 240).length > 0);
  const finalActionsDetailNonEmptyContract = actions.every((row) => cleanText(String(row.detail || ''), 400).length > 0);
  const finalActionsSummaryNoNewlineContract = actions.every((row) => !String(row.summary || '').includes('\n'));
  const finalActionsDetailNoNewlineContract = actions.every((row) => !String(row.detail || '').includes('\n'));
  const finalActionsSummaryNoTabContract = actions.every((row) => !String(row.summary || '').includes('\t'));
  const finalActionsDetailNoTabContract = actions.every((row) => !String(row.detail || '').includes('\t'));
  const finalActionsSummaryNoDoubleSpaceContract = actions.every(
    (row) => !cleanText(String(row.summary || ''), 240).includes('  '),
  );
  const finalActionsDetailNoDoubleSpaceContract = actions.every(
    (row) => !cleanText(String(row.detail || ''), 400).includes('  '),
  );
  const finalActionsCommandsUniquePerActionContract = actions.every((row) => {
    const commands = Array.isArray(row.recommended_commands) ? row.recommended_commands : [];
    return new Set(commands).size === commands.length;
  });
  const finalActionsCommandsCasefoldUniquePerActionContract = actions.every((row) => {
    const commands = Array.isArray(row.recommended_commands)
      ? row.recommended_commands.map((command) => cleanText(String(command || ''), 260).toLowerCase())
      : [];
    return new Set(commands).size === commands.length;
  });
  const finalActionsCommandsEachNonEmptyTrimmedContract = actions.every(
    (row) =>
      Array.isArray(row.recommended_commands)
      && row.recommended_commands.every((command) => {
        const raw = String(command || '');
        const token = cleanText(raw, 260);
        return token.length > 0 && token === raw.trim();
      }),
  );
  checks.push({
    id: 'eval_autopilot_final_check_ids_eval_prefix_contract',
    ok: finalCheckIdsEvalPrefixContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_contract_suffix_contract',
    ok: finalCheckIdsContractSuffixContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_length_bounded_contract',
    ok: finalCheckIdsLengthBoundedContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_placeholder_contract',
    ok: finalCheckIdsNoPlaceholderContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_ascii_contract',
    ok: finalCheckIdsAsciiContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_placeholder_contract',
    ok: finalActionIdsNoPlaceholderContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_ascii_contract',
    ok: finalActionIdsAsciiContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_double_space_contract',
    ok: finalActionIdsNoDoubleSpaceContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_nonempty_contract',
    ok: finalActionIdsNonEmptyContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_summary_nonempty_contract',
    ok: finalActionsSummaryNonEmptyContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_detail_nonempty_contract',
    ok: finalActionsDetailNonEmptyContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_summary_no_newline_contract',
    ok: finalActionsSummaryNoNewlineContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_detail_no_newline_contract',
    ok: finalActionsDetailNoNewlineContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_summary_no_tab_contract',
    ok: finalActionsSummaryNoTabContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_detail_no_tab_contract',
    ok: finalActionsDetailNoTabContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_summary_no_double_space_contract',
    ok: finalActionsSummaryNoDoubleSpaceContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_detail_no_double_space_contract',
    ok: finalActionsDetailNoDoubleSpaceContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_unique_per_action_contract',
    ok: finalActionsCommandsUniquePerActionContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_casefold_unique_per_action_contract',
    ok: finalActionsCommandsCasefoldUniquePerActionContract,
    detail: `count=${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_actions_commands_each_nonempty_trimmed_contract',
    ok: finalActionsCommandsEachNonEmptyTrimmedContract,
    detail: `count=${actions.length}`,
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
  const finalSummaryTotalActionsIntegerContract = Number.isInteger(report.summary.total_actions)
    && report.summary.total_actions >= 0;
  const finalSummaryTotalActionsMatchesActionsContract = report.summary.total_actions === actions.length;
  const finalSummaryHighOrCriticalIntegerContract = Number.isInteger(report.summary.high_or_critical_actions)
    && report.summary.high_or_critical_actions >= 0;
  const finalSummaryHighOrCriticalMatchesActionsContract = report.summary.high_or_critical_actions === finalHighOrCriticalActions;
  const finalSummaryHighOrCriticalBoundedContract = report.summary.high_or_critical_actions <= report.summary.total_actions;
  const finalSummaryAutomatableIntegerContract = Number.isInteger(report.summary.automatable_actions)
    && report.summary.automatable_actions >= 0;
  const finalSummaryAutomatableMatchesActionsContract = report.summary.automatable_actions === actions.filter((row) => row.automatable).length;
  const finalSummaryAutomatableBoundedContract = report.summary.automatable_actions <= report.summary.total_actions;
  const finalSummaryAutopilotReadyMatchesReportOkContract = report.summary.autopilot_ready === report.ok;
  const finalSummarySeverityCountsKeysExactContract = Object.keys(report.summary.severity_counts || {}).sort().join('|')
    === ['critical', 'high', 'info', 'low', 'medium'].join('|');
  const finalSummarySeverityCountsValuesIntegerContract = ['critical', 'high', 'medium', 'low', 'info']
    .every((key) => Number.isInteger((report.summary.severity_counts as any)[key]));
  const finalSummarySeverityCountsValuesNonNegativeContract = ['critical', 'high', 'medium', 'low', 'info']
    .every((key) => Number((report.summary.severity_counts as any)[key]) >= 0);
  const finalSummarySeverityCountsTotalMatchesTotalActionsContract = Object.values(report.summary.severity_counts || {})
    .reduce((sum, value) => sum + Number(value || 0), 0) === report.summary.total_actions;
  const finalSummarySeverityCountsTotalMatchesActionsContract = Object.values(report.summary.severity_counts || {})
    .reduce((sum, value) => sum + Number(value || 0), 0) === actions.length;
  const finalSummarySeverityCriticalMatchesContract = Number((report.summary.severity_counts as any).critical || 0) === finalSeverityCounts.critical;
  const finalSummarySeverityHighMatchesContract = Number((report.summary.severity_counts as any).high || 0) === finalSeverityCounts.high;
  const finalSummarySeverityMediumMatchesContract = Number((report.summary.severity_counts as any).medium || 0) === finalSeverityCounts.medium;
  const finalSummarySeverityLowMatchesContract = Number((report.summary.severity_counts as any).low || 0) === finalSeverityCounts.low;
  const finalSummarySeverityInfoMatchesContract = Number((report.summary.severity_counts as any).info || 0) === finalSeverityCounts.info;
  const finalSummarySeverityHighOrCriticalMatchesSummaryContract = (
    Number((report.summary.severity_counts as any).critical || 0)
    + Number((report.summary.severity_counts as any).high || 0)
  ) === report.summary.high_or_critical_actions;
  checks.push({
    id: 'eval_autopilot_final_summary_total_actions_integer_contract',
    ok: finalSummaryTotalActionsIntegerContract,
    detail: String(report.summary.total_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_total_actions_matches_actions_contract',
    ok: finalSummaryTotalActionsMatchesActionsContract,
    detail: `${report.summary.total_actions}|${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_high_or_critical_integer_contract',
    ok: finalSummaryHighOrCriticalIntegerContract,
    detail: String(report.summary.high_or_critical_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_high_or_critical_matches_actions_contract',
    ok: finalSummaryHighOrCriticalMatchesActionsContract,
    detail: `${report.summary.high_or_critical_actions}|${finalHighOrCriticalActions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_high_or_critical_bounded_contract',
    ok: finalSummaryHighOrCriticalBoundedContract,
    detail: `${report.summary.high_or_critical_actions}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_automatable_integer_contract',
    ok: finalSummaryAutomatableIntegerContract,
    detail: String(report.summary.automatable_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_automatable_matches_actions_contract',
    ok: finalSummaryAutomatableMatchesActionsContract,
    detail: `${report.summary.automatable_actions}|${actions.filter((row) => row.automatable).length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_automatable_bounded_contract',
    ok: finalSummaryAutomatableBoundedContract,
    detail: `${report.summary.automatable_actions}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_autopilot_ready_matches_report_ok_contract',
    ok: finalSummaryAutopilotReadyMatchesReportOkContract,
    detail: `${report.summary.autopilot_ready}|${report.ok}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_keys_exact_contract',
    ok: finalSummarySeverityCountsKeysExactContract,
    detail: Object.keys(report.summary.severity_counts || {}).sort().join('|'),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_values_integer_contract',
    ok: finalSummarySeverityCountsValuesIntegerContract,
    detail: JSON.stringify(report.summary.severity_counts || {}),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_values_nonnegative_contract',
    ok: finalSummarySeverityCountsValuesNonNegativeContract,
    detail: JSON.stringify(report.summary.severity_counts || {}),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_total_matches_total_actions_contract',
    ok: finalSummarySeverityCountsTotalMatchesTotalActionsContract,
    detail: `${Object.values(report.summary.severity_counts || {}).reduce((sum, value) => sum + Number(value || 0), 0)}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_total_matches_actions_contract',
    ok: finalSummarySeverityCountsTotalMatchesActionsContract,
    detail: `${Object.values(report.summary.severity_counts || {}).reduce((sum, value) => sum + Number(value || 0), 0)}|${actions.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_critical_matches_contract',
    ok: finalSummarySeverityCriticalMatchesContract,
    detail: `${Number((report.summary.severity_counts as any).critical || 0)}|${finalSeverityCounts.critical}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_high_matches_contract',
    ok: finalSummarySeverityHighMatchesContract,
    detail: `${Number((report.summary.severity_counts as any).high || 0)}|${finalSeverityCounts.high}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_medium_matches_contract',
    ok: finalSummarySeverityMediumMatchesContract,
    detail: `${Number((report.summary.severity_counts as any).medium || 0)}|${finalSeverityCounts.medium}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_low_matches_contract',
    ok: finalSummarySeverityLowMatchesContract,
    detail: `${Number((report.summary.severity_counts as any).low || 0)}|${finalSeverityCounts.low}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_info_matches_contract',
    ok: finalSummarySeverityInfoMatchesContract,
    detail: `${Number((report.summary.severity_counts as any).info || 0)}|${finalSeverityCounts.info}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_high_or_critical_matches_summary_contract',
    ok: finalSummarySeverityHighOrCriticalMatchesSummaryContract,
    detail: `${Number((report.summary.severity_counts as any).critical || 0) + Number((report.summary.severity_counts as any).high || 0)}|${report.summary.high_or_critical_actions}`,
  });
  const finalSummaryTotalActionsSafeIntegerContract = Number.isSafeInteger(report.summary.total_actions);
  const finalSummaryHighOrCriticalSafeIntegerContract = Number.isSafeInteger(report.summary.high_or_critical_actions);
  const finalSummaryAutomatableSafeIntegerContract = Number.isSafeInteger(report.summary.automatable_actions);
  const finalSummarySeverityCriticalSafeIntegerContract = Number.isSafeInteger(Number((report.summary.severity_counts as any).critical));
  const finalSummarySeverityHighSafeIntegerContract = Number.isSafeInteger(Number((report.summary.severity_counts as any).high));
  const finalSummarySeverityMediumSafeIntegerContract = Number.isSafeInteger(Number((report.summary.severity_counts as any).medium));
  const finalSummarySeverityLowSafeIntegerContract = Number.isSafeInteger(Number((report.summary.severity_counts as any).low));
  const finalSummarySeverityInfoSafeIntegerContract = Number.isSafeInteger(Number((report.summary.severity_counts as any).info));
  const finalSummarySeverityCriticalBoundedByTotalContract = Number((report.summary.severity_counts as any).critical) <= report.summary.total_actions;
  const finalSummarySeverityHighBoundedByTotalContract = Number((report.summary.severity_counts as any).high) <= report.summary.total_actions;
  const finalSummarySeverityMediumBoundedByTotalContract = Number((report.summary.severity_counts as any).medium) <= report.summary.total_actions;
  const finalSummarySeverityLowBoundedByTotalContract = Number((report.summary.severity_counts as any).low) <= report.summary.total_actions;
  const finalSummarySeverityInfoBoundedByTotalContract = Number((report.summary.severity_counts as any).info) <= report.summary.total_actions;
  const finalSummaryTotalActionsMaxTwentyContract = report.summary.total_actions <= 20;
  const finalSummaryAutomatableMaxTwentyContract = report.summary.automatable_actions <= 20;
  const finalSummaryHighOrCriticalMaxTwentyContract = report.summary.high_or_critical_actions <= 20;
  const finalSummaryHighOrCriticalGeCriticalContract = report.summary.high_or_critical_actions >= Number((report.summary.severity_counts as any).critical);
  const finalSummaryHighOrCriticalGeHighContract = report.summary.high_or_critical_actions >= Number((report.summary.severity_counts as any).high);
  const finalSummarySeverityCountsJsonToken = cleanText(JSON.stringify(report.summary.severity_counts || {}), 400);
  const finalSummarySeverityCountsJsonAsciiContract = isAsciiPrintable(finalSummarySeverityCountsJsonToken, 400);
  const finalSummarySeverityCountsJsonNoPlaceholderContract = !finalSummarySeverityCountsJsonToken.includes('${');
  checks.push({
    id: 'eval_autopilot_final_summary_total_actions_safe_integer_contract',
    ok: finalSummaryTotalActionsSafeIntegerContract,
    detail: String(report.summary.total_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_high_or_critical_safe_integer_contract',
    ok: finalSummaryHighOrCriticalSafeIntegerContract,
    detail: String(report.summary.high_or_critical_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_automatable_safe_integer_contract',
    ok: finalSummaryAutomatableSafeIntegerContract,
    detail: String(report.summary.automatable_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_critical_safe_integer_contract',
    ok: finalSummarySeverityCriticalSafeIntegerContract,
    detail: String(Number((report.summary.severity_counts as any).critical)),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_high_safe_integer_contract',
    ok: finalSummarySeverityHighSafeIntegerContract,
    detail: String(Number((report.summary.severity_counts as any).high)),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_medium_safe_integer_contract',
    ok: finalSummarySeverityMediumSafeIntegerContract,
    detail: String(Number((report.summary.severity_counts as any).medium)),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_low_safe_integer_contract',
    ok: finalSummarySeverityLowSafeIntegerContract,
    detail: String(Number((report.summary.severity_counts as any).low)),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_info_safe_integer_contract',
    ok: finalSummarySeverityInfoSafeIntegerContract,
    detail: String(Number((report.summary.severity_counts as any).info)),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_critical_bounded_by_total_contract',
    ok: finalSummarySeverityCriticalBoundedByTotalContract,
    detail: `${Number((report.summary.severity_counts as any).critical)}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_high_bounded_by_total_contract',
    ok: finalSummarySeverityHighBoundedByTotalContract,
    detail: `${Number((report.summary.severity_counts as any).high)}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_medium_bounded_by_total_contract',
    ok: finalSummarySeverityMediumBoundedByTotalContract,
    detail: `${Number((report.summary.severity_counts as any).medium)}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_low_bounded_by_total_contract',
    ok: finalSummarySeverityLowBoundedByTotalContract,
    detail: `${Number((report.summary.severity_counts as any).low)}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_info_bounded_by_total_contract',
    ok: finalSummarySeverityInfoBoundedByTotalContract,
    detail: `${Number((report.summary.severity_counts as any).info)}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_total_actions_max_twenty_contract',
    ok: finalSummaryTotalActionsMaxTwentyContract,
    detail: String(report.summary.total_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_automatable_max_twenty_contract',
    ok: finalSummaryAutomatableMaxTwentyContract,
    detail: String(report.summary.automatable_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_high_or_critical_max_twenty_contract',
    ok: finalSummaryHighOrCriticalMaxTwentyContract,
    detail: String(report.summary.high_or_critical_actions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_high_or_critical_ge_critical_contract',
    ok: finalSummaryHighOrCriticalGeCriticalContract,
    detail: `${report.summary.high_or_critical_actions}|${Number((report.summary.severity_counts as any).critical)}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_high_or_critical_ge_high_contract',
    ok: finalSummaryHighOrCriticalGeHighContract,
    detail: `${report.summary.high_or_critical_actions}|${Number((report.summary.severity_counts as any).high)}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_json_ascii_contract',
    ok: finalSummarySeverityCountsJsonAsciiContract,
    detail: finalSummarySeverityCountsJsonToken,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_json_no_placeholder_contract',
    ok: finalSummarySeverityCountsJsonNoPlaceholderContract,
    detail: finalSummarySeverityCountsJsonToken,
  });
  const finalSummarySeverityCountsJsonTrimmedContract = finalSummarySeverityCountsJsonToken === finalSummarySeverityCountsJsonToken.trim();
  const finalSummarySeverityCountsJsonLengthBoundedContract = finalSummarySeverityCountsJsonToken.length <= 400;
  const finalSummarySeverityCountsJsonNoNewlineContract = !finalSummarySeverityCountsJsonToken.includes('\n');
  const finalSummarySeverityCountsJsonNoTabContract = !finalSummarySeverityCountsJsonToken.includes('\t');
  const finalSummarySeverityCountsJsonNoDoubleSpaceContract = !finalSummarySeverityCountsJsonToken.includes('  ');
  const finalSummaryNonAutomatableActions = report.summary.total_actions - report.summary.automatable_actions;
  const finalSummaryNonAutomatableIntegerContract = Number.isInteger(finalSummaryNonAutomatableActions);
  const finalSummaryNonAutomatableSafeIntegerContract = Number.isSafeInteger(finalSummaryNonAutomatableActions);
  const finalSummaryNonAutomatableNonNegativeContract = finalSummaryNonAutomatableActions >= 0;
  const finalSummaryNonAutomatableBoundedContract = finalSummaryNonAutomatableActions <= report.summary.total_actions;
  const finalSummaryNonAutomatableMatchesActionsContract = finalSummaryNonAutomatableActions
    === actions.filter((row) => !row.automatable).length;
  const finalSummaryTotalEqualsAutomatablePlusNonAutomatableContract = report.summary.total_actions
    === report.summary.automatable_actions + finalSummaryNonAutomatableActions;
  const finalSummaryAutopilotReadyFormulaContract = report.summary.autopilot_ready
    === (report.ok && report.summary.high_or_critical_actions === 0);
  const finalSummaryAutopilotReadyFalseWhenHighOrCriticalPositiveContract = !(
    report.summary.high_or_critical_actions > 0 && report.summary.autopilot_ready
  );
  const finalSummaryAutopilotReadyTrueRequiresZeroHighOrCriticalContract = !report.summary.autopilot_ready
    || report.summary.high_or_critical_actions === 0;
  const finalSummaryAutopilotReadyTrueRequiresReportOkContract = !report.summary.autopilot_ready || report.ok;
  const finalSummaryReportOkFalseWhenHighOrCriticalPositiveContract = !(
    report.summary.high_or_critical_actions > 0 && report.ok
  );
  const finalSummaryTotalZeroImpliesHighOrCriticalZeroContract = report.summary.total_actions !== 0
    || report.summary.high_or_critical_actions === 0;
  const finalSummaryTotalZeroImpliesAutomatableZeroContract = report.summary.total_actions !== 0
    || report.summary.automatable_actions === 0;
  const finalSummaryTotalZeroImpliesSeverityAllZeroContract = report.summary.total_actions !== 0
    || ['critical', 'high', 'medium', 'low', 'info'].every(
      (key) => Number((report.summary.severity_counts as any)[key] || 0) === 0,
    );
  const finalSummaryAutomatablePlusHighOrCriticalBoundedContract = (
    report.summary.automatable_actions + report.summary.high_or_critical_actions
  ) <= report.summary.total_actions;
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_json_trimmed_contract',
    ok: finalSummarySeverityCountsJsonTrimmedContract,
    detail: finalSummarySeverityCountsJsonToken,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_json_length_bounded_contract',
    ok: finalSummarySeverityCountsJsonLengthBoundedContract,
    detail: String(finalSummarySeverityCountsJsonToken.length),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_json_no_newline_contract',
    ok: finalSummarySeverityCountsJsonNoNewlineContract,
    detail: finalSummarySeverityCountsJsonToken,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_json_no_tab_contract',
    ok: finalSummarySeverityCountsJsonNoTabContract,
    detail: finalSummarySeverityCountsJsonToken,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_severity_counts_json_no_double_space_contract',
    ok: finalSummarySeverityCountsJsonNoDoubleSpaceContract,
    detail: finalSummarySeverityCountsJsonToken,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_non_automatable_integer_contract',
    ok: finalSummaryNonAutomatableIntegerContract,
    detail: String(finalSummaryNonAutomatableActions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_non_automatable_safe_integer_contract',
    ok: finalSummaryNonAutomatableSafeIntegerContract,
    detail: String(finalSummaryNonAutomatableActions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_non_automatable_nonnegative_contract',
    ok: finalSummaryNonAutomatableNonNegativeContract,
    detail: String(finalSummaryNonAutomatableActions),
  });
  checks.push({
    id: 'eval_autopilot_final_summary_non_automatable_bounded_contract',
    ok: finalSummaryNonAutomatableBoundedContract,
    detail: `${finalSummaryNonAutomatableActions}|${report.summary.total_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_non_automatable_matches_actions_contract',
    ok: finalSummaryNonAutomatableMatchesActionsContract,
    detail: `${finalSummaryNonAutomatableActions}|${actions.filter((row) => !row.automatable).length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_total_equals_automatable_plus_non_automatable_contract',
    ok: finalSummaryTotalEqualsAutomatablePlusNonAutomatableContract,
    detail: `${report.summary.total_actions}|${report.summary.automatable_actions}|${finalSummaryNonAutomatableActions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_autopilot_ready_formula_contract',
    ok: finalSummaryAutopilotReadyFormulaContract,
    detail: `${report.summary.autopilot_ready}|${report.ok}|${report.summary.high_or_critical_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_autopilot_ready_false_when_high_or_critical_positive_contract',
    ok: finalSummaryAutopilotReadyFalseWhenHighOrCriticalPositiveContract,
    detail: `${report.summary.autopilot_ready}|${report.summary.high_or_critical_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_autopilot_ready_true_requires_zero_high_or_critical_contract',
    ok: finalSummaryAutopilotReadyTrueRequiresZeroHighOrCriticalContract,
    detail: `${report.summary.autopilot_ready}|${report.summary.high_or_critical_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_autopilot_ready_true_requires_report_ok_contract',
    ok: finalSummaryAutopilotReadyTrueRequiresReportOkContract,
    detail: `${report.summary.autopilot_ready}|${report.ok}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_report_ok_false_when_high_or_critical_positive_contract',
    ok: finalSummaryReportOkFalseWhenHighOrCriticalPositiveContract,
    detail: `${report.ok}|${report.summary.high_or_critical_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_total_zero_implies_high_or_critical_zero_contract',
    ok: finalSummaryTotalZeroImpliesHighOrCriticalZeroContract,
    detail: `${report.summary.total_actions}|${report.summary.high_or_critical_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_total_zero_implies_automatable_zero_contract',
    ok: finalSummaryTotalZeroImpliesAutomatableZeroContract,
    detail: `${report.summary.total_actions}|${report.summary.automatable_actions}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_total_zero_implies_severity_all_zero_contract',
    ok: finalSummaryTotalZeroImpliesSeverityAllZeroContract,
    detail: `${report.summary.total_actions}|${finalSummarySeverityCountsJsonToken}`,
  });
  checks.push({
    id: 'eval_autopilot_final_summary_automatable_plus_high_or_critical_bounded_contract',
    ok: finalSummaryAutomatablePlusHighOrCriticalBoundedContract,
    detail: `${report.summary.automatable_actions}|${report.summary.high_or_critical_actions}|${report.summary.total_actions}`,
  });
  const finalCheckIdsNoNewlineContract = finalCheckIds.every((id) => !id.includes('\n'));
  const finalCheckIdsNoTabContract = finalCheckIds.every((id) => !id.includes('\t'));
  const finalCheckIdsNoDoubleSpaceContract = finalCheckIds.every((id) => !id.includes('  '));
  const finalCheckIdsNoSlashContract = finalCheckIds.every((id) => !id.includes('/'));
  const finalCheckIdsNoBackslashContract = finalCheckIds.every((id) => !id.includes('\\'));
  const finalCheckIdsNoDotDotContract = finalCheckIds.every((id) => !id.includes('..'));
  const finalCheckIdsNoLeadingUnderscoreContract = finalCheckIds.every((id) => !id.startsWith('_'));
  const finalCheckIdsNoTrailingUnderscoreContract = finalCheckIds.every((id) => !id.endsWith('_'));
  const finalCheckIdsMinLengthContract = finalCheckIds.every((id) => id.length >= 12);
  const finalCheckIdsUnderscoreCountMinTwoContract = finalCheckIds.every(
    (id) => (id.match(/_/g) || []).length >= 2,
  );
  const finalActionIdsNoNewlineContract = finalActionIds.every((id) => !id.includes('\n'));
  const finalActionIdsNoTabContract = finalActionIds.every((id) => !id.includes('\t'));
  const finalActionIdsNoSlashContract = finalActionIds.every((id) => !id.includes('/'));
  const finalActionIdsNoBackslashContract = finalActionIds.every((id) => !id.includes('\\'));
  const finalActionIdsNoDotDotContract = finalActionIds.every((id) => !id.includes('..'));
  const finalActionIdsNoLeadingUnderscoreContract = finalActionIds.every((id) => !id.startsWith('_'));
  const finalActionIdsNoTrailingUnderscoreContract = finalActionIds.every((id) => !id.endsWith('_'));
  const finalActionIdsNoWhitespaceContract = finalActionIds.every((id) => !/\s/.test(id));
  const finalActionIdsUnderscoreCountMinOneContract = finalActionIds.every(
    (id) => (id.match(/_/g) || []).length >= 1,
  );
  const finalActionCategoriesNoWhitespaceContract = finalActionCategories.every(
    (category) => !/\s/.test(category),
  );
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_newline_contract',
    ok: finalCheckIdsNoNewlineContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_tab_contract',
    ok: finalCheckIdsNoTabContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_double_space_contract',
    ok: finalCheckIdsNoDoubleSpaceContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_slash_contract',
    ok: finalCheckIdsNoSlashContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_backslash_contract',
    ok: finalCheckIdsNoBackslashContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_dotdot_contract',
    ok: finalCheckIdsNoDotDotContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_leading_underscore_contract',
    ok: finalCheckIdsNoLeadingUnderscoreContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_trailing_underscore_contract',
    ok: finalCheckIdsNoTrailingUnderscoreContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_min_length_contract',
    ok: finalCheckIdsMinLengthContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_underscore_count_min_two_contract',
    ok: finalCheckIdsUnderscoreCountMinTwoContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_newline_contract',
    ok: finalActionIdsNoNewlineContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_tab_contract',
    ok: finalActionIdsNoTabContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_slash_contract',
    ok: finalActionIdsNoSlashContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_backslash_contract',
    ok: finalActionIdsNoBackslashContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_dotdot_contract',
    ok: finalActionIdsNoDotDotContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_leading_underscore_contract',
    ok: finalActionIdsNoLeadingUnderscoreContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_trailing_underscore_contract',
    ok: finalActionIdsNoTrailingUnderscoreContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_whitespace_contract',
    ok: finalActionIdsNoWhitespaceContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_underscore_count_min_one_contract',
    ok: finalActionIdsUnderscoreCountMinOneContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_whitespace_contract',
    ok: finalActionCategoriesNoWhitespaceContract,
    detail: `count=${finalActionCategories.length}`,
  });
  const finalActionCategoriesNoNewlineContract = finalActionCategories.every((category) => !category.includes('\n'));
  const finalActionCategoriesNoTabContract = finalActionCategories.every((category) => !category.includes('\t'));
  const finalActionCategoriesNoDoubleSpaceContract = finalActionCategories.every((category) => !category.includes('  '));
  const finalActionCategoriesNoSlashContract = finalActionCategories.every((category) => !category.includes('/'));
  const finalActionCategoriesNoBackslashContract = finalActionCategories.every((category) => !category.includes('\\'));
  const finalActionCategoriesNoDotDotContract = finalActionCategories.every((category) => !category.includes('..'));
  const finalActionCategoriesNoLeadingUnderscoreContract = finalActionCategories.every(
    (category) => !category.startsWith('_'),
  );
  const finalActionCategoriesNoTrailingUnderscoreContract = finalActionCategories.every(
    (category) => !category.endsWith('_'),
  );
  const finalActionCategoriesMinLengthThreeContract = finalActionCategories.every((category) => category.length >= 3);
  const finalActionCategoriesMaxLengthEightyContract = finalActionCategories.every((category) => category.length <= 80);
  const finalActionCategoriesLowercaseContract = finalActionCategories.every((category) => category === category.toLowerCase());
  const finalActionCategoriesCanonicalTokenContract = finalActionCategories.every(
    (category) => isCanonicalToken(category, 120),
  );
  const finalActionCategoriesAsciiContract = finalActionCategories.every(
    (category) => isAsciiPrintable(category, 120),
  );
  const finalActionCategoriesNoPlaceholderContract = finalActionCategories.every((category) => !category.includes('${'));
  const finalActionCategoriesCasefoldUniqueContract = new Set(finalActionCategories.map((category) => category.toLowerCase())).size
    === finalActionCategories.length;
  const finalActionCategoriesUnderscoreCountMinOneContract = finalActionCategories.every(
    (category) => (category.match(/_/g) || []).length >= 1,
  );
  const finalActionCategoriesNoDashContract = finalActionCategories.every((category) => !category.includes('-'));
  const finalActionCategoriesNoColonContract = finalActionCategories.every((category) => !category.includes(':'));
  const finalActionCategoriesNoSemicolonContract = finalActionCategories.every((category) => !category.includes(';'));
  const finalActionCategoriesNoCommaContract = finalActionCategories.every((category) => !category.includes(','));
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_newline_contract',
    ok: finalActionCategoriesNoNewlineContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_tab_contract',
    ok: finalActionCategoriesNoTabContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_double_space_contract',
    ok: finalActionCategoriesNoDoubleSpaceContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_slash_contract',
    ok: finalActionCategoriesNoSlashContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_backslash_contract',
    ok: finalActionCategoriesNoBackslashContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_dotdot_contract',
    ok: finalActionCategoriesNoDotDotContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_leading_underscore_contract',
    ok: finalActionCategoriesNoLeadingUnderscoreContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_trailing_underscore_contract',
    ok: finalActionCategoriesNoTrailingUnderscoreContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_min_length_three_contract',
    ok: finalActionCategoriesMinLengthThreeContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_max_length_eighty_contract',
    ok: finalActionCategoriesMaxLengthEightyContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_lowercase_contract',
    ok: finalActionCategoriesLowercaseContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_canonical_token_contract',
    ok: finalActionCategoriesCanonicalTokenContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_ascii_contract',
    ok: finalActionCategoriesAsciiContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_placeholder_contract',
    ok: finalActionCategoriesNoPlaceholderContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_casefold_unique_contract',
    ok: finalActionCategoriesCasefoldUniqueContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_underscore_count_min_one_contract',
    ok: finalActionCategoriesUnderscoreCountMinOneContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_dash_contract',
    ok: finalActionCategoriesNoDashContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_colon_contract',
    ok: finalActionCategoriesNoColonContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_semicolon_contract',
    ok: finalActionCategoriesNoSemicolonContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_comma_contract',
    ok: finalActionCategoriesNoCommaContract,
    detail: `count=${finalActionCategories.length}`,
  });
  const finalCheckIdsSnakeCaseContract = finalCheckIds.every((id) => /^[a-z0-9_]+$/.test(id));
  const finalCheckIdsNoConsecutiveUnderscoreContract = finalCheckIds.every((id) => !id.includes('__'));
  const finalCheckIdsNoLeadingDigitContract = finalCheckIds.every((id) => !/^[0-9]/.test(id));
  const finalCheckIdsNoTrailingDigitContract = finalCheckIds.every((id) => !/[0-9]$/.test(id));
  const finalCheckIdsContractSuffixOnceContract = finalCheckIds.every(
    (id) => (id.match(/_contract/g) || []).length === 1,
  );
  const finalCheckIdsContainsAutopilotTokenContract = finalCheckIds.every((id) => id.includes('autopilot'));
  const finalActionIdsSnakeCaseContract = finalActionIds.every((id) => /^[a-z0-9_]+$/.test(id));
  const finalActionIdsNoConsecutiveUnderscoreContract = finalActionIds.every((id) => !id.includes('__'));
  const finalActionIdsNoLeadingDigitContract = finalActionIds.every((id) => !/^[0-9]/.test(id));
  const finalActionIdsNoTrailingDigitContract = finalActionIds.every((id) => !/[0-9]$/.test(id));
  const finalActionIdsContainsEvalTokenContract = finalActionIds.every((id) => id.includes('eval'));
  const finalActionIdsMinLengthTenContract = finalActionIds.every((id) => id.length >= 10);
  const finalActionIdsMaxLengthEightyContract = finalActionIds.every((id) => id.length <= 80);
  const finalActionCategoriesSnakeCaseContract = finalActionCategories.every(
    (category) => /^[a-z0-9_]+$/.test(category),
  );
  const finalActionCategoriesNoConsecutiveUnderscoreContract = finalActionCategories.every(
    (category) => !category.includes('__'),
  );
  const finalActionCategoriesNoLeadingDigitContract = finalActionCategories.every(
    (category) => !/^[0-9]/.test(category),
  );
  const finalActionCategoriesNoTrailingDigitContract = finalActionCategories.every(
    (category) => !/[0-9]$/.test(category),
  );
  const finalSourcePathsNoBackslashContract = sourcePathTokens.every((token) => !token.includes('\\'));
  const finalSourcePathsNoDotDotContract = sourcePathTokens.every((token) => !token.includes('..'));
  const finalSourcePathsNoTabContract = sourcePathTokens.every((token) => !token.includes('\t'));
  checks.push({
    id: 'eval_autopilot_final_check_ids_snake_case_contract',
    ok: finalCheckIdsSnakeCaseContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_consecutive_underscore_contract',
    ok: finalCheckIdsNoConsecutiveUnderscoreContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_leading_digit_contract',
    ok: finalCheckIdsNoLeadingDigitContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_no_trailing_digit_contract',
    ok: finalCheckIdsNoTrailingDigitContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_contract_suffix_once_contract',
    ok: finalCheckIdsContractSuffixOnceContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_check_ids_contains_autopilot_token_contract',
    ok: finalCheckIdsContainsAutopilotTokenContract,
    detail: `count=${finalCheckIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_snake_case_contract',
    ok: finalActionIdsSnakeCaseContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_consecutive_underscore_contract',
    ok: finalActionIdsNoConsecutiveUnderscoreContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_leading_digit_contract',
    ok: finalActionIdsNoLeadingDigitContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_no_trailing_digit_contract',
    ok: finalActionIdsNoTrailingDigitContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_contains_eval_token_contract',
    ok: finalActionIdsContainsEvalTokenContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_min_length_ten_contract',
    ok: finalActionIdsMinLengthTenContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_ids_max_length_eighty_contract',
    ok: finalActionIdsMaxLengthEightyContract,
    detail: `count=${finalActionIds.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_snake_case_contract',
    ok: finalActionCategoriesSnakeCaseContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_consecutive_underscore_contract',
    ok: finalActionCategoriesNoConsecutiveUnderscoreContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_leading_digit_contract',
    ok: finalActionCategoriesNoLeadingDigitContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_action_categories_no_trailing_digit_contract',
    ok: finalActionCategoriesNoTrailingDigitContract,
    detail: `count=${finalActionCategories.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_no_backslash_contract',
    ok: finalSourcePathsNoBackslashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_no_dotdot_contract',
    ok: finalSourcePathsNoDotDotContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_no_tab_contract',
    ok: finalSourcePathsNoTabContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  const finalSourcePathsTailNoNewlineContract = sourcePathTokens.every((token) => !token.includes('\n'));
  const finalSourcePathsTailNoDoubleSpaceContract = sourcePathTokens.every((token) => !token.includes('  '));
  const finalSourcePathsTailNoLeadingSlashContract = sourcePathTokens.every((token) => !token.startsWith('/'));
  const finalSourcePathsTailNoTrailingSlashContract = sourcePathTokens.every((token) => !token.endsWith('/'));
  const finalSourcePathsTailNoWildcardContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return !value.includes('*') && !value.includes('?');
  });
  const finalSourcePathsTailNoShellJoinersContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return !value.includes('&&') && !value.includes('||') && !value.includes(';');
  });
  const finalSourcePathsTailNoPercentEnvContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('%'));
  const finalSourcePathsTailNoWindowsDrivePrefixContract = sourcePathTokens.every(
    (token) => !/^[A-Za-z]:/.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailLengthBoundedContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.length > 0 && value.length <= 240;
  });
  const finalSourcePathsTailPathSegmentsNonEmptyContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => segment.length > 0);
  });
  const finalReportGeneratedAtTailToken = cleanText(String(report.generated_at || ''), 120);
  const finalReportGeneratedAtTailNoNewlineContract = !finalReportGeneratedAtTailToken.includes('\n');
  const finalReportGeneratedAtTailNoTabContract = !finalReportGeneratedAtTailToken.includes('\t');
  const finalReportGeneratedAtTailNoSpaceContract = !finalReportGeneratedAtTailToken.includes(' ');
  const finalReportGeneratedAtTailContainsTContract = finalReportGeneratedAtTailToken.includes('T');
  const finalReportGeneratedAtTailMinLengthTwentyContract = finalReportGeneratedAtTailToken.length >= 20;
  const finalReportTypeTailToken = cleanText(String(report.type || ''), 120);
  const finalReportTypeTailLowercaseContract = finalReportTypeTailToken === finalReportTypeTailToken.toLowerCase();
  const finalReportTypeTailNoWhitespaceContract = !/\s/.test(finalReportTypeTailToken);
  const finalReportTypeTailNoPlaceholderContract = !finalReportTypeTailToken.includes('${');
  const finalReportTypeTailAsciiContract = isAsciiPrintable(finalReportTypeTailToken, 120);
  const finalReportTypeTailLengthBoundedContract = finalReportTypeTailToken.length >= 3
    && finalReportTypeTailToken.length <= 80;
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_newline_contract',
    ok: finalSourcePathsTailNoNewlineContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_double_space_contract',
    ok: finalSourcePathsTailNoDoubleSpaceContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_leading_slash_contract',
    ok: finalSourcePathsTailNoLeadingSlashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_trailing_slash_contract',
    ok: finalSourcePathsTailNoTrailingSlashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_wildcard_contract',
    ok: finalSourcePathsTailNoWildcardContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_shell_joiners_contract',
    ok: finalSourcePathsTailNoShellJoinersContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_percent_env_contract',
    ok: finalSourcePathsTailNoPercentEnvContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_windows_drive_prefix_contract',
    ok: finalSourcePathsTailNoWindowsDrivePrefixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_length_bounded_contract',
    ok: finalSourcePathsTailLengthBoundedContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_path_segments_nonempty_contract',
    ok: finalSourcePathsTailPathSegmentsNonEmptyContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_newline_contract',
    ok: finalReportGeneratedAtTailNoNewlineContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_tab_contract',
    ok: finalReportGeneratedAtTailNoTabContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_space_contract',
    ok: finalReportGeneratedAtTailNoSpaceContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_contains_t_contract',
    ok: finalReportGeneratedAtTailContainsTContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_min_length_twenty_contract',
    ok: finalReportGeneratedAtTailMinLengthTwentyContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_lowercase_contract',
    ok: finalReportTypeTailLowercaseContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_whitespace_contract',
    ok: finalReportTypeTailNoWhitespaceContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_placeholder_contract',
    ok: finalReportTypeTailNoPlaceholderContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_ascii_contract',
    ok: finalReportTypeTailAsciiContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_length_bounded_contract',
    ok: finalReportTypeTailLengthBoundedContract,
    detail: finalReportTypeTailToken,
  });
  const finalSourcePathsTailNoPlaceholderContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('${'),
  );
  const finalSourcePathsTailNoBacktickContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('`'));
  const finalSourcePathsTailNoDollarContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('$'));
  const finalSourcePathsTailNoDoubleQuoteContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('"'));
  const finalSourcePathsTailNoSingleQuoteContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('\''));
  const finalSourcePathsTailNoAngleBracketContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return !value.includes('<') && !value.includes('>');
  });
  const finalSourcePathsTailNoPipeContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('|'));
  const finalSourcePathsTailNoAmpersandContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('&'));
  const finalSourcePathsTailNoCarriageReturnContract = sourcePathTokens.every((token) => !token.includes('\r'));
  const finalSourcePathsTailNoHashContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('#'));
  const finalSourcePathsTailNoExclamationContract = sourcePathTokens.every((token) => !cleanText(token, 260).includes('!'));
  const finalReportGeneratedAtTailNoPlaceholderContract = !finalReportGeneratedAtTailToken.includes('${');
  const finalReportGeneratedAtTailAsciiContract = isAsciiPrintable(finalReportGeneratedAtTailToken, 120);
  const finalReportGeneratedAtTailEndsWithZContract = finalReportGeneratedAtTailToken.endsWith('Z');
  const finalReportGeneratedAtTailMaxLengthFortyContract = finalReportGeneratedAtTailToken.length <= 40;
  const finalReportGeneratedAtTailContainsDashContract = finalReportGeneratedAtTailToken.includes('-');
  const finalReportGeneratedAtTailContainsColonContract = finalReportGeneratedAtTailToken.includes(':');
  const finalReportGeneratedAtTailNoSlashContract = !finalReportGeneratedAtTailToken.includes('/');
  const finalReportGeneratedAtTailNoBackslashContract = !finalReportGeneratedAtTailToken.includes('\\');
  const finalReportGeneratedAtTailNoCommaContract = !finalReportGeneratedAtTailToken.includes(',');
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_placeholder_contract',
    ok: finalSourcePathsTailNoPlaceholderContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_backtick_contract',
    ok: finalSourcePathsTailNoBacktickContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_dollar_contract',
    ok: finalSourcePathsTailNoDollarContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_double_quote_contract',
    ok: finalSourcePathsTailNoDoubleQuoteContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_single_quote_contract',
    ok: finalSourcePathsTailNoSingleQuoteContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_angle_bracket_contract',
    ok: finalSourcePathsTailNoAngleBracketContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_pipe_contract',
    ok: finalSourcePathsTailNoPipeContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_ampersand_contract',
    ok: finalSourcePathsTailNoAmpersandContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_carriage_return_contract',
    ok: finalSourcePathsTailNoCarriageReturnContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_hash_contract',
    ok: finalSourcePathsTailNoHashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_exclamation_contract',
    ok: finalSourcePathsTailNoExclamationContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_placeholder_contract',
    ok: finalReportGeneratedAtTailNoPlaceholderContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_ascii_contract',
    ok: finalReportGeneratedAtTailAsciiContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_ends_with_z_contract',
    ok: finalReportGeneratedAtTailEndsWithZContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_max_length_forty_contract',
    ok: finalReportGeneratedAtTailMaxLengthFortyContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_contains_dash_contract',
    ok: finalReportGeneratedAtTailContainsDashContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_contains_colon_contract',
    ok: finalReportGeneratedAtTailContainsColonContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_slash_contract',
    ok: finalReportGeneratedAtTailNoSlashContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_backslash_contract',
    ok: finalReportGeneratedAtTailNoBackslashContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_comma_contract',
    ok: finalReportGeneratedAtTailNoCommaContract,
    detail: finalReportGeneratedAtTailToken,
  });
  const finalReportTypeTailStartsWithEvalAutopilotContract = finalReportTypeTailToken.startsWith('eval_autopilot_');
  const finalReportTypeTailEndsWithReportContract = finalReportTypeTailToken.endsWith('_report');
  const finalReportTypeTailSnakeCaseContract = /^[a-z0-9_]+$/.test(finalReportTypeTailToken);
  const finalReportTypeTailNoConsecutiveUnderscoreContract = !finalReportTypeTailToken.includes('__');
  const finalReportTypeTailNoLeadingDigitContract = !/^[0-9]/.test(finalReportTypeTailToken);
  const finalReportTypeTailNoTrailingDigitContract = !/[0-9]$/.test(finalReportTypeTailToken);
  const finalReportTypeTailNoLeadingUnderscoreContract = !finalReportTypeTailToken.startsWith('_');
  const finalReportTypeTailNoTrailingUnderscoreContract = !finalReportTypeTailToken.endsWith('_');
  const finalReportTypeTailNoDashContract = !finalReportTypeTailToken.includes('-');
  const finalReportTypeTailNoColonContract = !finalReportTypeTailToken.includes(':');
  const finalReportTypeTailNoSemicolonContract = !finalReportTypeTailToken.includes(';');
  const finalReportTypeTailNoCommaContract = !finalReportTypeTailToken.includes(',');
  const finalReportTypeTailNoSlashContract = !finalReportTypeTailToken.includes('/');
  const finalReportTypeTailNoBackslashContract = !finalReportTypeTailToken.includes('\\');
  const finalReportTypeTailNoDoubleSpaceContract = !finalReportTypeTailToken.includes('  ');
  const finalReportTypeTailUnderscoreCountMinTwoContract = (finalReportTypeTailToken.match(/_/g) || []).length >= 2;
  const finalReportTypeTailNoBacktickContract = !finalReportTypeTailToken.includes('`');
  const finalReportTypeTailNoDollarContract = !finalReportTypeTailToken.includes('$');
  const finalReportTypeTailNoDoubleQuoteContract = !finalReportTypeTailToken.includes('"');
  const finalReportTypeTailNoSingleQuoteContract = !finalReportTypeTailToken.includes('\'');
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_starts_with_eval_autopilot_contract',
    ok: finalReportTypeTailStartsWithEvalAutopilotContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_ends_with_report_contract',
    ok: finalReportTypeTailEndsWithReportContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_snake_case_contract',
    ok: finalReportTypeTailSnakeCaseContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_consecutive_underscore_contract',
    ok: finalReportTypeTailNoConsecutiveUnderscoreContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_leading_digit_contract',
    ok: finalReportTypeTailNoLeadingDigitContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_trailing_digit_contract',
    ok: finalReportTypeTailNoTrailingDigitContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_leading_underscore_contract',
    ok: finalReportTypeTailNoLeadingUnderscoreContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_trailing_underscore_contract',
    ok: finalReportTypeTailNoTrailingUnderscoreContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_dash_contract',
    ok: finalReportTypeTailNoDashContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_colon_contract',
    ok: finalReportTypeTailNoColonContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_semicolon_contract',
    ok: finalReportTypeTailNoSemicolonContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_comma_contract',
    ok: finalReportTypeTailNoCommaContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_slash_contract',
    ok: finalReportTypeTailNoSlashContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_backslash_contract',
    ok: finalReportTypeTailNoBackslashContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_double_space_contract',
    ok: finalReportTypeTailNoDoubleSpaceContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_underscore_count_min_two_contract',
    ok: finalReportTypeTailUnderscoreCountMinTwoContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_backtick_contract',
    ok: finalReportTypeTailNoBacktickContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_dollar_contract',
    ok: finalReportTypeTailNoDollarContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_double_quote_contract',
    ok: finalReportTypeTailNoDoubleQuoteContract,
    detail: finalReportTypeTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_type_tail_no_single_quote_contract',
    ok: finalReportTypeTailNoSingleQuoteContract,
    detail: finalReportTypeTailToken,
  });
  const finalReportGeneratedAtTailIsoMatch = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{3}))?Z$/.exec(
    finalReportGeneratedAtTailToken,
  );
  const finalReportGeneratedAtTailIsoUtcShapeContract = finalReportGeneratedAtTailIsoMatch !== null;
  const finalReportGeneratedAtTailSingleTContract = (finalReportGeneratedAtTailToken.match(/T/g) || []).length === 1;
  const finalReportGeneratedAtTailSingleZContract = (finalReportGeneratedAtTailToken.match(/Z/g) || []).length === 1;
  const finalReportGeneratedAtTailNoLowercaseZContract = !finalReportGeneratedAtTailToken.includes('z');
  const finalReportGeneratedAtTailNoPlusContract = !finalReportGeneratedAtTailToken.includes('+');
  const finalReportGeneratedAtTailNoMinusAfterTContract = !(
    finalReportGeneratedAtTailToken.includes('T')
    && finalReportGeneratedAtTailToken.split('T')[1].includes('-')
  );
  const finalReportGeneratedAtTailNoUnderscoreContract = !finalReportGeneratedAtTailToken.includes('_');
  const finalReportGeneratedAtTailNoDotDotContract = !finalReportGeneratedAtTailToken.includes('..');
  const finalReportGeneratedAtTailYear = finalReportGeneratedAtTailIsoMatch ? Number(finalReportGeneratedAtTailIsoMatch[1]) : NaN;
  const finalReportGeneratedAtTailMonth = finalReportGeneratedAtTailIsoMatch ? Number(finalReportGeneratedAtTailIsoMatch[2]) : NaN;
  const finalReportGeneratedAtTailDay = finalReportGeneratedAtTailIsoMatch ? Number(finalReportGeneratedAtTailIsoMatch[3]) : NaN;
  const finalReportGeneratedAtTailHour = finalReportGeneratedAtTailIsoMatch ? Number(finalReportGeneratedAtTailIsoMatch[4]) : NaN;
  const finalReportGeneratedAtTailMinute = finalReportGeneratedAtTailIsoMatch ? Number(finalReportGeneratedAtTailIsoMatch[5]) : NaN;
  const finalReportGeneratedAtTailSecond = finalReportGeneratedAtTailIsoMatch ? Number(finalReportGeneratedAtTailIsoMatch[6]) : NaN;
  const finalReportGeneratedAtTailYearRangeContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && finalReportGeneratedAtTailYear >= 2000
    && finalReportGeneratedAtTailYear <= 2200;
  const finalReportGeneratedAtTailMonthRangeContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && finalReportGeneratedAtTailMonth >= 1
    && finalReportGeneratedAtTailMonth <= 12;
  const finalReportGeneratedAtTailDayRangeContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && finalReportGeneratedAtTailDay >= 1
    && finalReportGeneratedAtTailDay <= 31;
  const finalReportGeneratedAtTailHourRangeContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && finalReportGeneratedAtTailHour >= 0
    && finalReportGeneratedAtTailHour <= 23;
  const finalReportGeneratedAtTailMinuteRangeContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && finalReportGeneratedAtTailMinute >= 0
    && finalReportGeneratedAtTailMinute <= 59;
  const finalReportGeneratedAtTailSecondRangeContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && finalReportGeneratedAtTailSecond >= 0
    && finalReportGeneratedAtTailSecond <= 59;
  const finalSourcePathsTailCasefoldUniqueContract = new Set(
    sourcePathTokens.map((token) => cleanText(token, 260).toLowerCase()),
  ).size === sourcePathTokens.length;
  const finalSourcePathsTailNoLeadingDotSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !segment.startsWith('.'));
  });
  const finalSourcePathsTailNoTrailingDotSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !segment.endsWith('.'));
  });
  const finalSourcePathsTailSegmentsNoSpaceContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !segment.includes(' '));
  });
  const finalSourcePathsTailSegmentsLengthBoundedContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => segment.length > 0 && segment.length <= 120);
  });
  const finalSourcePathsTailNoEmptyAfterTrimContract = sourcePathTokens.every(
    (token) => cleanText(token, 260).trim().length > 0,
  );
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_iso_utc_shape_contract',
    ok: finalReportGeneratedAtTailIsoUtcShapeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_single_t_contract',
    ok: finalReportGeneratedAtTailSingleTContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_single_z_contract',
    ok: finalReportGeneratedAtTailSingleZContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_lowercase_z_contract',
    ok: finalReportGeneratedAtTailNoLowercaseZContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_plus_contract',
    ok: finalReportGeneratedAtTailNoPlusContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_minus_after_t_contract',
    ok: finalReportGeneratedAtTailNoMinusAfterTContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_underscore_contract',
    ok: finalReportGeneratedAtTailNoUnderscoreContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_dotdot_contract',
    ok: finalReportGeneratedAtTailNoDotDotContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_year_range_contract',
    ok: finalReportGeneratedAtTailYearRangeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_month_range_contract',
    ok: finalReportGeneratedAtTailMonthRangeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_day_range_contract',
    ok: finalReportGeneratedAtTailDayRangeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_hour_range_contract',
    ok: finalReportGeneratedAtTailHourRangeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_minute_range_contract',
    ok: finalReportGeneratedAtTailMinuteRangeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_second_range_contract',
    ok: finalReportGeneratedAtTailSecondRangeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_casefold_unique_contract',
    ok: finalSourcePathsTailCasefoldUniqueContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_leading_dot_segment_contract',
    ok: finalSourcePathsTailNoLeadingDotSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_trailing_dot_segment_contract',
    ok: finalSourcePathsTailNoTrailingDotSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_segments_no_space_contract',
    ok: finalSourcePathsTailSegmentsNoSpaceContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_segments_length_bounded_contract',
    ok: finalSourcePathsTailSegmentsLengthBoundedContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_empty_after_trim_contract',
    ok: finalSourcePathsTailNoEmptyAfterTrimContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  const finalReportGeneratedAtTailTrimmedContract = finalReportGeneratedAtTailToken === finalReportGeneratedAtTailToken.trim();
  const finalReportGeneratedAtTailCharsetWhitelistContract = /^[0-9T:.\-Z]+$/.test(finalReportGeneratedAtTailToken);
  const finalReportGeneratedAtTailDateParseValidContract = !Number.isNaN(Date.parse(finalReportGeneratedAtTailToken));
  const finalReportGeneratedAtTailMillisFormatOptionalThreeDigitsContract = !finalReportGeneratedAtTailToken.includes('.')
    || /\.\d{3}Z$/.test(finalReportGeneratedAtTailToken);
  const finalReportGeneratedAtTailDotCountBoundedContract = (finalReportGeneratedAtTailToken.match(/\./g) || []).length <= 1;
  const finalReportGeneratedAtTailNoDoubleColonContract = !finalReportGeneratedAtTailToken.includes('::');
  const finalReportGeneratedAtTailSeparatorPositionsContract = finalReportGeneratedAtTailToken.length >= 20
    && finalReportGeneratedAtTailToken[4] === '-'
    && finalReportGeneratedAtTailToken[7] === '-'
    && finalReportGeneratedAtTailToken[10] === 'T'
    && finalReportGeneratedAtTailToken[13] === ':'
    && finalReportGeneratedAtTailToken[16] === ':'
    && finalReportGeneratedAtTailToken.endsWith('Z');
  const finalReportGeneratedAtTailYearTokenLengthFourContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && String(finalReportGeneratedAtTailYear).length === 4;
  const finalReportGeneratedAtTailMonthTokenLengthTwoContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && String(finalReportGeneratedAtTailIsoMatch?.[2] || '').length === 2;
  const finalReportGeneratedAtTailDayTokenLengthTwoContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && String(finalReportGeneratedAtTailIsoMatch?.[3] || '').length === 2;
  const finalReportGeneratedAtTailHourTokenLengthTwoContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && String(finalReportGeneratedAtTailIsoMatch?.[4] || '').length === 2;
  const finalReportGeneratedAtTailMinuteTokenLengthTwoContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && String(finalReportGeneratedAtTailIsoMatch?.[5] || '').length === 2;
  const finalReportGeneratedAtTailSecondTokenLengthTwoContract = finalReportGeneratedAtTailIsoUtcShapeContract
    && String(finalReportGeneratedAtTailIsoMatch?.[6] || '').length === 2;
  const finalReportGeneratedAtTailNoAlphaExceptTzContract = finalReportGeneratedAtTailToken
    .replace(/[0-9:\-\.]/g, '')
    .split('')
    .every((char) => char === 'T' || char === 'Z');
  const finalReportGeneratedAtTailNoParenthesesContract = !finalReportGeneratedAtTailToken.includes('(')
    && !finalReportGeneratedAtTailToken.includes(')');
  const finalReportGeneratedAtTailNoBracketsContract = !finalReportGeneratedAtTailToken.includes('[')
    && !finalReportGeneratedAtTailToken.includes(']');
  const finalReportGeneratedAtTailNoBracesContract = !finalReportGeneratedAtTailToken.includes('{')
    && !finalReportGeneratedAtTailToken.includes('}');
  const finalReportGeneratedAtTailNoAtSignContract = !finalReportGeneratedAtTailToken.includes('@');
  const finalReportGeneratedAtTailNoQuestionContract = !finalReportGeneratedAtTailToken.includes('?');
  const finalReportGeneratedAtTailNoAmpersandContract = !finalReportGeneratedAtTailToken.includes('&');
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_trimmed_contract',
    ok: finalReportGeneratedAtTailTrimmedContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_charset_whitelist_contract',
    ok: finalReportGeneratedAtTailCharsetWhitelistContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_date_parse_valid_contract',
    ok: finalReportGeneratedAtTailDateParseValidContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_millis_format_optional_three_digits_contract',
    ok: finalReportGeneratedAtTailMillisFormatOptionalThreeDigitsContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_dot_count_bounded_contract',
    ok: finalReportGeneratedAtTailDotCountBoundedContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_double_colon_contract',
    ok: finalReportGeneratedAtTailNoDoubleColonContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_separator_positions_contract',
    ok: finalReportGeneratedAtTailSeparatorPositionsContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_year_token_length_four_contract',
    ok: finalReportGeneratedAtTailYearTokenLengthFourContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_month_token_length_two_contract',
    ok: finalReportGeneratedAtTailMonthTokenLengthTwoContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_day_token_length_two_contract',
    ok: finalReportGeneratedAtTailDayTokenLengthTwoContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_hour_token_length_two_contract',
    ok: finalReportGeneratedAtTailHourTokenLengthTwoContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_minute_token_length_two_contract',
    ok: finalReportGeneratedAtTailMinuteTokenLengthTwoContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_second_token_length_two_contract',
    ok: finalReportGeneratedAtTailSecondTokenLengthTwoContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_alpha_except_tz_contract',
    ok: finalReportGeneratedAtTailNoAlphaExceptTzContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_parentheses_contract',
    ok: finalReportGeneratedAtTailNoParenthesesContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_brackets_contract',
    ok: finalReportGeneratedAtTailNoBracketsContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_braces_contract',
    ok: finalReportGeneratedAtTailNoBracesContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_at_sign_contract',
    ok: finalReportGeneratedAtTailNoAtSignContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_question_contract',
    ok: finalReportGeneratedAtTailNoQuestionContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_ampersand_contract',
    ok: finalReportGeneratedAtTailNoAmpersandContract,
    detail: finalReportGeneratedAtTailToken,
  });
  const finalSourcePathsTailNoColonContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes(':'),
  );
  const finalSourcePathsTailNoSemicolonContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes(';'),
  );
  const finalSourcePathsTailNoCommaContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes(','),
  );
  const finalSourcePathsTailNoPlusContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('+'),
  );
  const finalSourcePathsTailNoEqualsContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('='),
  );
  const finalSourcePathsTailNoTildeContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('~'),
  );
  const finalSourcePathsTailNoParenthesesContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return !value.includes('(') && !value.includes(')');
  });
  const finalSourcePathsTailNoBracketsContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return !value.includes('[') && !value.includes(']');
  });
  const finalSourcePathsTailNoBracesContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return !value.includes('{') && !value.includes('}');
  });
  const finalSourcePathsTailNoAtSignContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('@'),
  );
  const finalSourcePathsTailNoQuestionContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('?'),
  );
  const finalSourcePathsTailCharsetWhitelistContract = sourcePathTokens.every((token) =>
    /^[A-Za-z0-9._/-]+$/.test(cleanText(token, 260)),
  );
  const finalReportGeneratedAtTailNoTildeContract = !finalReportGeneratedAtTailToken.includes('~');
  const finalReportGeneratedAtTailNoPipeContract = !finalReportGeneratedAtTailToken.includes('|');
  const finalReportGeneratedAtTailNoHashContract = !finalReportGeneratedAtTailToken.includes('#');
  const finalReportGeneratedAtTailNoExclamationContract = !finalReportGeneratedAtTailToken.includes('!');
  const finalReportGeneratedAtTailNoEqualsContract = !finalReportGeneratedAtTailToken.includes('=');
  const finalReportGeneratedAtTailNoPercentContract = !finalReportGeneratedAtTailToken.includes('%');
  const finalReportGeneratedAtTailNoBacktickContract = !finalReportGeneratedAtTailToken.includes('`');
  const finalReportGeneratedAtTailNoDoubleSpaceContract = !finalReportGeneratedAtTailToken.includes('  ');
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_colon_contract',
    ok: finalSourcePathsTailNoColonContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_semicolon_contract',
    ok: finalSourcePathsTailNoSemicolonContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_comma_contract',
    ok: finalSourcePathsTailNoCommaContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_plus_contract',
    ok: finalSourcePathsTailNoPlusContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_equals_contract',
    ok: finalSourcePathsTailNoEqualsContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_tilde_contract',
    ok: finalSourcePathsTailNoTildeContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_parentheses_contract',
    ok: finalSourcePathsTailNoParenthesesContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_brackets_contract',
    ok: finalSourcePathsTailNoBracketsContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_braces_contract',
    ok: finalSourcePathsTailNoBracesContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_at_sign_contract',
    ok: finalSourcePathsTailNoAtSignContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_question_contract',
    ok: finalSourcePathsTailNoQuestionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_charset_whitelist_contract',
    ok: finalSourcePathsTailCharsetWhitelistContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_tilde_contract',
    ok: finalReportGeneratedAtTailNoTildeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_pipe_contract',
    ok: finalReportGeneratedAtTailNoPipeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_hash_contract',
    ok: finalReportGeneratedAtTailNoHashContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_exclamation_contract',
    ok: finalReportGeneratedAtTailNoExclamationContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_equals_contract',
    ok: finalReportGeneratedAtTailNoEqualsContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_percent_contract',
    ok: finalReportGeneratedAtTailNoPercentContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_backtick_contract',
    ok: finalReportGeneratedAtTailNoBacktickContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_double_space_contract',
    ok: finalReportGeneratedAtTailNoDoubleSpaceContract,
    detail: finalReportGeneratedAtTailToken,
  });
  const finalSourcePathsTailNoPercentContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('%'),
  );
  const finalSourcePathsTailNoCaretContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('^'),
  );
  const finalSourcePathsTailNoAsteriskContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('*'),
  );
  const finalSourcePathsTailNoControlCharsContract = sourcePathTokens.every(
    (token) => !/[\x00-\x1F\x7F]/.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoConsecutiveSlashContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('//'),
  );
  const finalSourcePathsTailNoLeadingSlashContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).startsWith('/'),
  );
  const finalSourcePathsTailNoTrailingSlashContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).endsWith('/'),
  );
  const finalSourcePathsTailNoWindowsDrivePrefixContract = sourcePathTokens.every(
    (token) => !/^[A-Za-z]:/.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailSegmentsNoLeadingDashContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !segment.startsWith('-'));
  });
  const finalSourcePathsTailSegmentsNoTrailingDashContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !segment.endsWith('-'));
  });
  const finalSourcePathsTailDepthMinTwoContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').length >= 2;
  });
  const finalSourcePathsTailDepthMaxTwelveContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').length <= 12;
  });
  const finalReportGeneratedAtTailNoSemicolonContract = !finalReportGeneratedAtTailToken.includes(';');
  const finalReportGeneratedAtTailNoWhitespaceContract = !/\s/.test(finalReportGeneratedAtTailToken);
  const finalReportGeneratedAtTailNoNewlineContract = !finalReportGeneratedAtTailToken.includes('\n');
  const finalReportGeneratedAtTailNoTabContract = !finalReportGeneratedAtTailToken.includes('\t');
  const finalReportGeneratedAtTailNoCarriageReturnContract = !finalReportGeneratedAtTailToken.includes('\r');
  const finalReportGeneratedAtTailDashCountExactTwoContract = (finalReportGeneratedAtTailToken.match(/-/g) || []).length === 2;
  const finalReportGeneratedAtTailColonCountExactTwoContract = (finalReportGeneratedAtTailToken.match(/:/g) || []).length === 2;
  const finalReportGeneratedAtTailLengthTwentyOrTwentyFourContract = finalReportGeneratedAtTailToken.length === 20
    || finalReportGeneratedAtTailToken.length === 24;
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_percent_contract',
    ok: finalSourcePathsTailNoPercentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_caret_contract',
    ok: finalSourcePathsTailNoCaretContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_asterisk_contract',
    ok: finalSourcePathsTailNoAsteriskContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_control_chars_contract',
    ok: finalSourcePathsTailNoControlCharsContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_consecutive_slash_contract',
    ok: finalSourcePathsTailNoConsecutiveSlashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_leading_slash_contract',
    ok: finalSourcePathsTailNoLeadingSlashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_trailing_slash_contract',
    ok: finalSourcePathsTailNoTrailingSlashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_windows_drive_prefix_contract',
    ok: finalSourcePathsTailNoWindowsDrivePrefixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_segments_no_leading_dash_contract',
    ok: finalSourcePathsTailSegmentsNoLeadingDashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_segments_no_trailing_dash_contract',
    ok: finalSourcePathsTailSegmentsNoTrailingDashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_depth_min_two_contract',
    ok: finalSourcePathsTailDepthMinTwoContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_depth_max_twelve_contract',
    ok: finalSourcePathsTailDepthMaxTwelveContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_semicolon_contract',
    ok: finalReportGeneratedAtTailNoSemicolonContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_whitespace_contract',
    ok: finalReportGeneratedAtTailNoWhitespaceContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_newline_contract',
    ok: finalReportGeneratedAtTailNoNewlineContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_tab_contract',
    ok: finalReportGeneratedAtTailNoTabContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_carriage_return_contract',
    ok: finalReportGeneratedAtTailNoCarriageReturnContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_dash_count_exact_two_contract',
    ok: finalReportGeneratedAtTailDashCountExactTwoContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_colon_count_exact_two_contract',
    ok: finalReportGeneratedAtTailColonCountExactTwoContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_length_twenty_or_twenty_four_contract',
    ok: finalReportGeneratedAtTailLengthTwentyOrTwentyFourContract,
    detail: finalReportGeneratedAtTailToken,
  });
  const finalSourcePathsTailNoUrlSchemeContract = sourcePathTokens.every(
    (token) => !cleanText(token, 260).includes('://'),
  );
  const finalSourcePathsTailNoEncodedSlashContract = sourcePathTokens.every(
    (token) => !/%2f/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoEncodedBackslashContract = sourcePathTokens.every(
    (token) => !/%5c/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoEncodedDotDotContract = sourcePathTokens.every(
    (token) => !/%2e%2e/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoCurrentDirSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => segment !== '.');
  });
  const finalSourcePathsTailNoParentTraversalSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => segment !== '..');
  });
  const finalSourcePathsTailNoEmptySegmentsContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => segment.length > 0);
  });
  const finalSourcePathsTailSegmentCharsetWhitelistContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => /^[A-Za-z0-9._-]+$/.test(segment));
  });
  const finalSourcePathsTailNoSegmentDoubleDashContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !segment.includes('--'));
  });
  const finalSourcePathsTailNoSegmentDoubleUnderscoreContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !segment.includes('__'));
  });
  const finalSourcePathsTailPosixNormalizeIdempotentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return path.posix.normalize(value) === value;
  });
  const finalSourcePathsTailNormalizeNotParentPrefixedContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    const normalized = path.posix.normalize(value);
    return normalized !== '..' && !normalized.startsWith('../');
  });
  const finalReportGeneratedAtTailDatePartIsoShapeContract = /^\d{4}-\d{2}-\d{2}T/.test(
    finalReportGeneratedAtTailToken,
  );
  const finalReportGeneratedAtTailTimePartIsoShapeContract = /T\d{2}:\d{2}:\d{2}(\.\d{3})?Z$/.test(
    finalReportGeneratedAtTailToken,
  );
  const finalReportGeneratedAtTailTIndexTenContract = finalReportGeneratedAtTailToken.indexOf('T') === 10;
  const finalReportGeneratedAtTailNoLeadingSpaceContract = !finalReportGeneratedAtTailToken.startsWith(' ');
  const finalReportGeneratedAtTailNoTrailingSpaceContract = !finalReportGeneratedAtTailToken.endsWith(' ');
  const finalReportGeneratedAtTailNoTrailingDotBeforeZContract = !finalReportGeneratedAtTailToken.endsWith('.Z');
  const finalReportGeneratedAtTailNoEpochSecondsShapeContract = !/^\d{10}$/.test(finalReportGeneratedAtTailToken);
  const finalReportGeneratedAtTailNoEpochMillisShapeContract = !/^\d{13}$/.test(finalReportGeneratedAtTailToken);
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_url_scheme_contract',
    ok: finalSourcePathsTailNoUrlSchemeContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_encoded_slash_contract',
    ok: finalSourcePathsTailNoEncodedSlashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_encoded_backslash_contract',
    ok: finalSourcePathsTailNoEncodedBackslashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_encoded_dotdot_contract',
    ok: finalSourcePathsTailNoEncodedDotDotContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_current_dir_segment_contract',
    ok: finalSourcePathsTailNoCurrentDirSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_parent_traversal_segment_contract',
    ok: finalSourcePathsTailNoParentTraversalSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_empty_segments_contract',
    ok: finalSourcePathsTailNoEmptySegmentsContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_segment_charset_whitelist_contract',
    ok: finalSourcePathsTailSegmentCharsetWhitelistContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_segment_double_dash_contract',
    ok: finalSourcePathsTailNoSegmentDoubleDashContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_segment_double_underscore_contract',
    ok: finalSourcePathsTailNoSegmentDoubleUnderscoreContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_posix_normalize_idempotent_contract',
    ok: finalSourcePathsTailPosixNormalizeIdempotentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_normalize_not_parent_prefixed_contract',
    ok: finalSourcePathsTailNormalizeNotParentPrefixedContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_date_part_iso_shape_contract',
    ok: finalReportGeneratedAtTailDatePartIsoShapeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_time_part_iso_shape_contract',
    ok: finalReportGeneratedAtTailTimePartIsoShapeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_t_index_ten_contract',
    ok: finalReportGeneratedAtTailTIndexTenContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_leading_space_contract',
    ok: finalReportGeneratedAtTailNoLeadingSpaceContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_trailing_space_contract',
    ok: finalReportGeneratedAtTailNoTrailingSpaceContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_trailing_dot_before_z_contract',
    ok: finalReportGeneratedAtTailNoTrailingDotBeforeZContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_epoch_seconds_shape_contract',
    ok: finalReportGeneratedAtTailNoEpochSecondsShapeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_epoch_millis_shape_contract',
    ok: finalReportGeneratedAtTailNoEpochMillisShapeContract,
    detail: finalReportGeneratedAtTailToken,
  });
  const finalSourcePathsTailNoWindowsReservedDeviceNamesContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !/^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i.test(segment));
  });
  const finalSourcePathsTailNoSegmentTrailingDotOrSpaceContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !/[. ]$/.test(segment));
  });
  const finalSourcePathsTailBasenameNonEmptyContract = sourcePathTokens.every(
    (token) => path.posix.basename(cleanText(token, 260)).length > 0,
  );
  const finalSourcePathsTailDirnameNonEmptyContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    const dirname = path.posix.dirname(value);
    return dirname !== '.' && dirname.length > 0;
  });
  const finalSourcePathsTailNoTempSuffixContract = sourcePathTokens.every(
    (token) => !/\.(tmp|temp)$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoBackupSuffixContract = sourcePathTokens.every(
    (token) => !/\.(bak|backup|old)$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoRejectSuffixContract = sourcePathTokens.every(
    (token) => !/\.(rej|orig)$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoSwapSuffixContract = sourcePathTokens.every(
    (token) => !/\.sw[opx]$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoBidiControlsContract = sourcePathTokens.every(
    (token) => !/[\u202A-\u202E\u2066-\u2069]/.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoSegmentDoubleDotPatternContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260);
    return value.split('/').every((segment) => !segment.includes('..'));
  });
  const finalSourcePathsTailNoBinaryArtifactExtensionContract = sourcePathTokens.every(
    (token) => !/\.(exe|dll|so|dylib|bin|o|obj|a|class)$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoArchiveExtensionContract = sourcePathTokens.every(
    (token) => !/\.(zip|tar|tgz|gz|bz2|xz|zst|7z)$/i.test(cleanText(token, 260)),
  );
  const finalReportGeneratedAtTailNoLowercaseTContract = !finalReportGeneratedAtTailToken.includes('t');
  const finalReportGeneratedAtTailNoBidiControlsContract = !/[\u202A-\u202E\u2066-\u2069]/.test(
    finalReportGeneratedAtTailToken,
  );
  const finalReportGeneratedAtTailNoControlCharsContract = !/[\x00-\x1F\x7F]/.test(
    finalReportGeneratedAtTailToken,
  );
  const finalReportGeneratedAtTailDotCount = (finalReportGeneratedAtTailToken.match(/\./g) || []).length;
  const finalReportGeneratedAtTailMillisPresenceMatchesDotCountContract = finalReportGeneratedAtTailToken.includes('.')
    ? finalReportGeneratedAtTailDotCount === 1
    : finalReportGeneratedAtTailDotCount === 0;
  const finalReportGeneratedAtTailCanonicalizedIsoRoundtripContract = (() => {
    const parsedMs = Date.parse(finalReportGeneratedAtTailToken);
    if (Number.isNaN(parsedMs)) {
      return false;
    }
    const normalized = finalReportGeneratedAtTailToken.includes('.')
      ? finalReportGeneratedAtTailToken
      : finalReportGeneratedAtTailToken.replace('Z', '.000Z');
    return new Date(parsedMs).toISOString() === normalized;
  })();
  const finalReportGeneratedAtTailUtcComponentsMatchTokenContract = (() => {
    if (!finalReportGeneratedAtTailIsoUtcShapeContract) {
      return false;
    }
    const parsedMs = Date.parse(finalReportGeneratedAtTailToken);
    if (Number.isNaN(parsedMs)) {
      return false;
    }
    const parsed = new Date(parsedMs);
    return parsed.getUTCFullYear() === finalReportGeneratedAtTailYear
      && parsed.getUTCMonth() + 1 === finalReportGeneratedAtTailMonth
      && parsed.getUTCDate() === finalReportGeneratedAtTailDay
      && parsed.getUTCHours() === finalReportGeneratedAtTailHour
      && parsed.getUTCMinutes() === finalReportGeneratedAtTailMinute
      && parsed.getUTCSeconds() === finalReportGeneratedAtTailSecond;
  })();
  const finalReportGeneratedAtTailValidCalendarDateExactContract = (() => {
    if (!finalReportGeneratedAtTailIsoUtcShapeContract) {
      return false;
    }
    const calendar = new Date(
      Date.UTC(finalReportGeneratedAtTailYear, finalReportGeneratedAtTailMonth - 1, finalReportGeneratedAtTailDay),
    );
    return calendar.getUTCFullYear() === finalReportGeneratedAtTailYear
      && calendar.getUTCMonth() + 1 === finalReportGeneratedAtTailMonth
      && calendar.getUTCDate() === finalReportGeneratedAtTailDay;
  })();
  const finalReportGeneratedAtTailNoSignedYearPrefixContract = !finalReportGeneratedAtTailToken.startsWith('+')
    && !finalReportGeneratedAtTailToken.startsWith('-');
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_windows_reserved_device_names_contract',
    ok: finalSourcePathsTailNoWindowsReservedDeviceNamesContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_segment_trailing_dot_or_space_contract',
    ok: finalSourcePathsTailNoSegmentTrailingDotOrSpaceContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_basename_non_empty_contract',
    ok: finalSourcePathsTailBasenameNonEmptyContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_dirname_non_empty_contract',
    ok: finalSourcePathsTailDirnameNonEmptyContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_temp_suffix_contract',
    ok: finalSourcePathsTailNoTempSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_backup_suffix_contract',
    ok: finalSourcePathsTailNoBackupSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_reject_suffix_contract',
    ok: finalSourcePathsTailNoRejectSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_swap_suffix_contract',
    ok: finalSourcePathsTailNoSwapSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_bidi_controls_contract',
    ok: finalSourcePathsTailNoBidiControlsContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_segment_double_dot_pattern_contract',
    ok: finalSourcePathsTailNoSegmentDoubleDotPatternContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_binary_artifact_extension_contract',
    ok: finalSourcePathsTailNoBinaryArtifactExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_archive_extension_contract',
    ok: finalSourcePathsTailNoArchiveExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_lowercase_t_contract',
    ok: finalReportGeneratedAtTailNoLowercaseTContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_bidi_controls_contract',
    ok: finalReportGeneratedAtTailNoBidiControlsContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_control_chars_contract',
    ok: finalReportGeneratedAtTailNoControlCharsContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_millis_presence_matches_dot_count_contract',
    ok: finalReportGeneratedAtTailMillisPresenceMatchesDotCountContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_canonicalized_iso_roundtrip_contract',
    ok: finalReportGeneratedAtTailCanonicalizedIsoRoundtripContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_utc_components_match_token_contract',
    ok: finalReportGeneratedAtTailUtcComponentsMatchTokenContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_valid_calendar_date_exact_contract',
    ok: finalReportGeneratedAtTailValidCalendarDateExactContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_signed_year_prefix_contract',
    ok: finalReportGeneratedAtTailNoSignedYearPrefixContract,
    detail: finalReportGeneratedAtTailToken,
  });
  const finalSourcePathsTailNoNodeModulesSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'node_modules');
  });
  const finalSourcePathsTailNoTargetSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'target');
  });
  const finalSourcePathsTailNoDistSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'dist');
  });
  const finalSourcePathsTailNoBuildSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'build');
  });
  const finalSourcePathsTailNoOutSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'out');
  });
  const finalSourcePathsTailNoCoverageSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'coverage');
  });
  const finalSourcePathsTailNoGitSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.git');
  });
  const finalSourcePathsTailNoCacheSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.cache' && segment !== 'cache');
  });
  const finalSourcePathsTailNoTmpSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'tmp' && segment !== 'temp');
  });
  const finalSourcePathsTailNoVendorSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'vendor');
  });
  const finalSourcePathsTailNoGeneratedSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value
      .split('/')
      .every((segment) => segment !== 'generated' && segment !== '__generated__');
  });
  const finalSourcePathsTailNoSnapshotsSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value
      .split('/')
      .every((segment) => segment !== 'snapshots' && segment !== '__snapshots__');
  });
  const finalReportGeneratedAtTailDotPositionWhenMillisContract = !finalReportGeneratedAtTailToken.includes('.')
    || finalReportGeneratedAtTailToken[19] === '.';
  const finalReportGeneratedAtTailZPositionLastContract = finalReportGeneratedAtTailToken.lastIndexOf('Z')
    === finalReportGeneratedAtTailToken.length - 1;
  const finalReportGeneratedAtTailCharAtNineteenZOrDotContract = finalReportGeneratedAtTailToken.length >= 20
    && (finalReportGeneratedAtTailToken[19] === 'Z' || finalReportGeneratedAtTailToken[19] === '.');
  const finalReportGeneratedAtTailMillisPresenceMatchesLengthContract = finalReportGeneratedAtTailToken.includes('.')
    ? finalReportGeneratedAtTailToken.length === 24
    : finalReportGeneratedAtTailToken.length === 20;
  const finalReportGeneratedAtTailFebruaryDayMaxContract = finalReportGeneratedAtTailMonth !== 2
    || finalReportGeneratedAtTailDay <= 29;
  const finalReportGeneratedAtTailThirtyDayMonthLimitContract = ![4, 6, 9, 11].includes(
    finalReportGeneratedAtTailMonth,
  ) || finalReportGeneratedAtTailDay <= 30;
  const finalReportGeneratedAtTailThirtyOneDayMonthMembershipContract = finalReportGeneratedAtTailDay !== 31
    || [1, 3, 5, 7, 8, 10, 12].includes(finalReportGeneratedAtTailMonth);
  const finalReportGeneratedAtTailLeapDayRequiresLeapYearContract = !(
    finalReportGeneratedAtTailMonth === 2 && finalReportGeneratedAtTailDay === 29
  ) || (
    finalReportGeneratedAtTailYear % 4 === 0
      && (finalReportGeneratedAtTailYear % 100 !== 0 || finalReportGeneratedAtTailYear % 400 === 0)
  );
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_node_modules_segment_contract',
    ok: finalSourcePathsTailNoNodeModulesSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_target_segment_contract',
    ok: finalSourcePathsTailNoTargetSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_dist_segment_contract',
    ok: finalSourcePathsTailNoDistSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_build_segment_contract',
    ok: finalSourcePathsTailNoBuildSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_out_segment_contract',
    ok: finalSourcePathsTailNoOutSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_coverage_segment_contract',
    ok: finalSourcePathsTailNoCoverageSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_git_segment_contract',
    ok: finalSourcePathsTailNoGitSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_cache_segment_contract',
    ok: finalSourcePathsTailNoCacheSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_tmp_segment_contract',
    ok: finalSourcePathsTailNoTmpSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_vendor_segment_contract',
    ok: finalSourcePathsTailNoVendorSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_generated_segment_contract',
    ok: finalSourcePathsTailNoGeneratedSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_snapshots_segment_contract',
    ok: finalSourcePathsTailNoSnapshotsSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_dot_position_when_millis_contract',
    ok: finalReportGeneratedAtTailDotPositionWhenMillisContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_z_position_last_contract',
    ok: finalReportGeneratedAtTailZPositionLastContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_char_at_nineteen_z_or_dot_contract',
    ok: finalReportGeneratedAtTailCharAtNineteenZOrDotContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_millis_presence_matches_length_contract',
    ok: finalReportGeneratedAtTailMillisPresenceMatchesLengthContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_february_day_max_contract',
    ok: finalReportGeneratedAtTailFebruaryDayMaxContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_thirty_day_month_limit_contract',
    ok: finalReportGeneratedAtTailThirtyDayMonthLimitContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_thirty_one_day_month_membership_contract',
    ok: finalReportGeneratedAtTailThirtyOneDayMonthMembershipContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_leap_day_requires_leap_year_contract',
    ok: finalReportGeneratedAtTailLeapDayRequiresLeapYearContract,
    detail: finalReportGeneratedAtTailToken,
  });
  const finalSourcePathsTailNoDsStoreSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.ds_store');
  });
  const finalSourcePathsTailNoThumbsDbSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'thumbs.db');
  });
  const finalSourcePathsTailNoRecycleBinSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '$recycle.bin');
  });
  const finalSourcePathsTailNoPycacheSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '__pycache__');
  });
  const finalSourcePathsTailNoPytestCacheSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.pytest_cache');
  });
  const finalSourcePathsTailNoMypyCacheSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.mypy_cache');
  });
  const finalSourcePathsTailNoRuffCacheSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.ruff_cache');
  });
  const finalSourcePathsTailNoNextSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.next');
  });
  const finalSourcePathsTailNoTurboSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.turbo');
  });
  const finalSourcePathsTailNoParcelCacheSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.parcel-cache');
  });
  const finalSourcePathsTailNoPnpmStoreSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.pnpm-store');
  });
  const finalSourcePathsTailNoNpmSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.npm');
  });
  const finalReportGeneratedAtTailParsedMs = Date.parse(finalReportGeneratedAtTailToken);
  const finalReportGeneratedAtTailParsedMsFiniteContract = Number.isFinite(finalReportGeneratedAtTailParsedMs);
  const finalReportGeneratedAtTailParsedMsSafeIntegerContract = Number.isSafeInteger(finalReportGeneratedAtTailParsedMs);
  const finalReportGeneratedAtTailParsedNotBefore2024Contract = finalReportGeneratedAtTailParsedMsFiniteContract
    && finalReportGeneratedAtTailParsedMs >= Date.UTC(2024, 0, 1, 0, 0, 0, 0);
  const finalReportGeneratedAtTailNowMs = Date.now();
  const finalReportGeneratedAtTailParsedNotAfterNowPlusOneDayContract = finalReportGeneratedAtTailParsedMsFiniteContract
    && finalReportGeneratedAtTailParsedMs <= finalReportGeneratedAtTailNowMs + 86_400_000;
  const finalReportGeneratedAtTailParsedNotBeforeNowMinusTenYearsContract = finalReportGeneratedAtTailParsedMsFiniteContract
    && finalReportGeneratedAtTailParsedMs >= finalReportGeneratedAtTailNowMs - 315_576_000_000;
  const finalReportGeneratedAtTailMillisToken = finalReportGeneratedAtTailIsoMatch?.[7] || '';
  const finalReportGeneratedAtTailMillisTokenNumeric = finalReportGeneratedAtTailMillisToken.length > 0
    ? Number(finalReportGeneratedAtTailMillisToken)
    : 0;
  const finalReportGeneratedAtTailMillisAbsentImpliesParsedMsModuloZeroContract = !finalReportGeneratedAtTailParsedMsFiniteContract
    ? false
    : finalReportGeneratedAtTailMillisToken.length > 0 || finalReportGeneratedAtTailParsedMs % 1000 === 0;
  const finalReportGeneratedAtTailMillisPresentImpliesParsedMsModuloMatchesContract = !finalReportGeneratedAtTailParsedMsFiniteContract
    ? false
    : finalReportGeneratedAtTailMillisToken.length === 0
      || finalReportGeneratedAtTailParsedMs % 1000 === finalReportGeneratedAtTailMillisTokenNumeric;
  const finalReportGeneratedAtTailNoTimezoneWordContract = !/(UTC|GMT|PST|MST|CST|EST)/.test(
    finalReportGeneratedAtTailToken,
  );
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_ds_store_segment_contract',
    ok: finalSourcePathsTailNoDsStoreSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_thumbs_db_segment_contract',
    ok: finalSourcePathsTailNoThumbsDbSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_recycle_bin_segment_contract',
    ok: finalSourcePathsTailNoRecycleBinSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_pycache_segment_contract',
    ok: finalSourcePathsTailNoPycacheSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_pytest_cache_segment_contract',
    ok: finalSourcePathsTailNoPytestCacheSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_mypy_cache_segment_contract',
    ok: finalSourcePathsTailNoMypyCacheSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_ruff_cache_segment_contract',
    ok: finalSourcePathsTailNoRuffCacheSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_next_segment_contract',
    ok: finalSourcePathsTailNoNextSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_turbo_segment_contract',
    ok: finalSourcePathsTailNoTurboSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_parcel_cache_segment_contract',
    ok: finalSourcePathsTailNoParcelCacheSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_pnpm_store_segment_contract',
    ok: finalSourcePathsTailNoPnpmStoreSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_npm_segment_contract',
    ok: finalSourcePathsTailNoNpmSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_parsed_ms_finite_contract',
    ok: finalReportGeneratedAtTailParsedMsFiniteContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_parsed_ms_safe_integer_contract',
    ok: finalReportGeneratedAtTailParsedMsSafeIntegerContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_parsed_not_before_2024_contract',
    ok: finalReportGeneratedAtTailParsedNotBefore2024Contract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_parsed_not_after_now_plus_one_day_contract',
    ok: finalReportGeneratedAtTailParsedNotAfterNowPlusOneDayContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_parsed_not_before_now_minus_ten_years_contract',
    ok: finalReportGeneratedAtTailParsedNotBeforeNowMinusTenYearsContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_millis_absent_implies_parsed_ms_modulo_zero_contract',
    ok: finalReportGeneratedAtTailMillisAbsentImpliesParsedMsModuloZeroContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_millis_present_implies_parsed_ms_modulo_matches_contract',
    ok: finalReportGeneratedAtTailMillisPresentImpliesParsedMsModuloMatchesContract,
    detail: finalReportGeneratedAtTailToken,
  });
  checks.push({
    id: 'eval_autopilot_final_report_generated_at_tail_no_timezone_word_contract',
    ok: finalReportGeneratedAtTailNoTimezoneWordContract,
    detail: finalReportGeneratedAtTailToken,
  });
  const finalSourcePathsTailNoDotVenvSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.venv');
  });
  const finalSourcePathsTailNoVenvSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'venv');
  });
  const finalSourcePathsTailNoToxSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.tox');
  });
  const finalSourcePathsTailNoNoxSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.nox');
  });
  const finalSourcePathsTailNoGradleSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.gradle');
  });
  const finalSourcePathsTailNoTerraformSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.terraform');
  });
  const finalSourcePathsTailNoServerlessSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.serverless');
  });
  const finalSourcePathsTailNoAwsSamSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.aws-sam');
  });
  const finalSourcePathsTailNoIdeaSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.idea');
  });
  const finalSourcePathsTailNoVscodeSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.vscode');
  });
  const finalSourcePathsTailNoMacosxSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '__macosx');
  });
  const finalSourcePathsTailNoSassCacheSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.sass-cache');
  });
  const finalSourcePathsTailNoHypothesisSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.hypothesis');
  });
  const finalSourcePathsTailNoIpynbCheckpointsSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.ipynb_checkpoints');
  });
  const finalSourcePathsTailNoNuxtSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.nuxt');
  });
  const finalSourcePathsTailNoAngularSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.angular');
  });
  const finalSourcePathsTailNoExpoSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.expo');
  });
  const finalSourcePathsTailNoDartToolSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.dart_tool');
  });
  const finalSourcePathsTailNoSvelteKitSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.svelte-kit');
  });
  const finalSourcePathsTailNoWranglerSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== '.wrangler');
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_dot_venv_segment_contract',
    ok: finalSourcePathsTailNoDotVenvSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_venv_segment_contract',
    ok: finalSourcePathsTailNoVenvSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_tox_segment_contract',
    ok: finalSourcePathsTailNoToxSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_nox_segment_contract',
    ok: finalSourcePathsTailNoNoxSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_gradle_segment_contract',
    ok: finalSourcePathsTailNoGradleSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_terraform_segment_contract',
    ok: finalSourcePathsTailNoTerraformSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_serverless_segment_contract',
    ok: finalSourcePathsTailNoServerlessSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_aws_sam_segment_contract',
    ok: finalSourcePathsTailNoAwsSamSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_idea_segment_contract',
    ok: finalSourcePathsTailNoIdeaSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_vscode_segment_contract',
    ok: finalSourcePathsTailNoVscodeSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_macosx_segment_contract',
    ok: finalSourcePathsTailNoMacosxSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_sass_cache_segment_contract',
    ok: finalSourcePathsTailNoSassCacheSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_hypothesis_segment_contract',
    ok: finalSourcePathsTailNoHypothesisSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_ipynb_checkpoints_segment_contract',
    ok: finalSourcePathsTailNoIpynbCheckpointsSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_nuxt_segment_contract',
    ok: finalSourcePathsTailNoNuxtSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_angular_segment_contract',
    ok: finalSourcePathsTailNoAngularSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_expo_segment_contract',
    ok: finalSourcePathsTailNoExpoSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_dart_tool_segment_contract',
    ok: finalSourcePathsTailNoDartToolSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_svelte_kit_segment_contract',
    ok: finalSourcePathsTailNoSvelteKitSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_wrangler_segment_contract',
    ok: finalSourcePathsTailNoWranglerSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  const finalSourcePathsTailNoStorybookStaticSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'storybook-static');
  });
  const finalSourcePathsTailNoPlaywrightReportSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'playwright-report');
  });
  const finalSourcePathsTailNoTestResultsSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'test-results');
  });
  const finalSourcePathsTailNoJunitSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'junit');
  });
  const finalSourcePathsTailNoSurefireReportsSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'surefire-reports');
  });
  const finalSourcePathsTailNoAllureResultsSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'allure-results');
  });
  const finalSourcePathsTailNoAllureReportSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'allure-report');
  });
  const finalSourcePathsTailNoCypressSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'cypress');
  });
  const finalSourcePathsTailNoCypressCacheSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'cypress-cache');
  });
  const finalSourcePathsTailNoDetoxSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'detox');
  });
  const finalSourcePathsTailNoBenchmarkSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'benchmark');
  });
  const finalSourcePathsTailNoBenchmarksSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'benchmarks');
  });
  const finalSourcePathsTailNoPerfSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'perf');
  });
  const finalSourcePathsTailNoProfilesSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'profiles');
  });
  const finalSourcePathsTailNoTmpfilesSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'tmpfiles');
  });
  const finalSourcePathsTailNoArtifactSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'artifact');
  });
  const finalSourcePathsTailNoArtifactsSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'artifacts');
  });
  const finalSourcePathsTailNoDownloadsSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'downloads');
  });
  const finalSourcePathsTailNoUploadsSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'uploads');
  });
  const finalSourcePathsTailNoScratchSegmentContract = sourcePathTokens.every((token) => {
    const value = cleanText(token, 260).toLowerCase();
    return value.split('/').every((segment) => segment !== 'scratch');
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_storybook_static_segment_contract',
    ok: finalSourcePathsTailNoStorybookStaticSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_playwright_report_segment_contract',
    ok: finalSourcePathsTailNoPlaywrightReportSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_test_results_segment_contract',
    ok: finalSourcePathsTailNoTestResultsSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_junit_segment_contract',
    ok: finalSourcePathsTailNoJunitSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_surefire_reports_segment_contract',
    ok: finalSourcePathsTailNoSurefireReportsSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_allure_results_segment_contract',
    ok: finalSourcePathsTailNoAllureResultsSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_allure_report_segment_contract',
    ok: finalSourcePathsTailNoAllureReportSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_cypress_segment_contract',
    ok: finalSourcePathsTailNoCypressSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_cypress_cache_segment_contract',
    ok: finalSourcePathsTailNoCypressCacheSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_detox_segment_contract',
    ok: finalSourcePathsTailNoDetoxSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_benchmark_segment_contract',
    ok: finalSourcePathsTailNoBenchmarkSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_benchmarks_segment_contract',
    ok: finalSourcePathsTailNoBenchmarksSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_perf_segment_contract',
    ok: finalSourcePathsTailNoPerfSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_profiles_segment_contract',
    ok: finalSourcePathsTailNoProfilesSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_tmpfiles_segment_contract',
    ok: finalSourcePathsTailNoTmpfilesSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_artifact_segment_contract',
    ok: finalSourcePathsTailNoArtifactSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_artifacts_segment_contract',
    ok: finalSourcePathsTailNoArtifactsSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_downloads_segment_contract',
    ok: finalSourcePathsTailNoDownloadsSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_uploads_segment_contract',
    ok: finalSourcePathsTailNoUploadsSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_scratch_segment_contract',
    ok: finalSourcePathsTailNoScratchSegmentContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  const finalSourcePathsTailNoLogExtensionContract = sourcePathTokens.every(
    (token) => !/\.log$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoTraceExtensionContract = sourcePathTokens.every(
    (token) => !/\.trace$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoOutFileExtensionContract = sourcePathTokens.every(
    (token) => !/\.out$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoErrFileExtensionContract = sourcePathTokens.every(
    (token) => !/\.err$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoLcovExtensionContract = sourcePathTokens.every(
    (token) => !/\.lcov$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoSarifExtensionContract = sourcePathTokens.every(
    (token) => !/\.sarif$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoCpuProfileExtensionContract = sourcePathTokens.every(
    (token) => !/\.cpuprofile$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoHeapProfileExtensionContract = sourcePathTokens.every(
    (token) => !/\.heapprofile$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoProfExtensionContract = sourcePathTokens.every(
    (token) => !/\.prof$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoGcdaExtensionContract = sourcePathTokens.every(
    (token) => !/\.gcda$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoGcnoExtensionContract = sourcePathTokens.every(
    (token) => !/\.gcno$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoDmpExtensionContract = sourcePathTokens.every(
    (token) => !/\.dmp$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoStackdumpExtensionContract = sourcePathTokens.every(
    (token) => !/\.stackdump$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoPidExtensionContract = sourcePathTokens.every(
    (token) => !/\.pid$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoSeedExtensionContract = sourcePathTokens.every(
    (token) => !/\.seed$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoSnapshotExtensionContract = sourcePathTokens.every(
    (token) => !/\.snapshot$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoSnapExtensionContract = sourcePathTokens.every(
    (token) => !/\.snap$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoCoverageArtifactExtensionContract = sourcePathTokens.every(
    (token) => !/\.coverage$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoNycOutputExtensionContract = sourcePathTokens.every(
    (token) => !/\.nyc_output$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoRstestJsonExtensionContract = sourcePathTokens.every(
    (token) => !/\.rstest\.json$/i.test(cleanText(token, 260)),
  );
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_log_extension_contract',
    ok: finalSourcePathsTailNoLogExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_trace_extension_contract',
    ok: finalSourcePathsTailNoTraceExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_out_file_extension_contract',
    ok: finalSourcePathsTailNoOutFileExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_err_file_extension_contract',
    ok: finalSourcePathsTailNoErrFileExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_lcov_extension_contract',
    ok: finalSourcePathsTailNoLcovExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_sarif_extension_contract',
    ok: finalSourcePathsTailNoSarifExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_cpu_profile_extension_contract',
    ok: finalSourcePathsTailNoCpuProfileExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_heap_profile_extension_contract',
    ok: finalSourcePathsTailNoHeapProfileExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_prof_extension_contract',
    ok: finalSourcePathsTailNoProfExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_gcda_extension_contract',
    ok: finalSourcePathsTailNoGcdaExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_gcno_extension_contract',
    ok: finalSourcePathsTailNoGcnoExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_dmp_extension_contract',
    ok: finalSourcePathsTailNoDmpExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_stackdump_extension_contract',
    ok: finalSourcePathsTailNoStackdumpExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_pid_extension_contract',
    ok: finalSourcePathsTailNoPidExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_seed_extension_contract',
    ok: finalSourcePathsTailNoSeedExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_snapshot_extension_contract',
    ok: finalSourcePathsTailNoSnapshotExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_snap_extension_contract',
    ok: finalSourcePathsTailNoSnapExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_coverage_artifact_extension_contract',
    ok: finalSourcePathsTailNoCoverageArtifactExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_nyc_output_extension_contract',
    ok: finalSourcePathsTailNoNycOutputExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_rstest_json_extension_contract',
    ok: finalSourcePathsTailNoRstestJsonExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  const finalSourcePathsTailNoProfrawExtensionContract = sourcePathTokens.every(
    (token) => !/\.profraw$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoProfdataExtensionContract = sourcePathTokens.every(
    (token) => !/\.profdata$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoGcovExtensionContract = sourcePathTokens.every(
    (token) => !/\.gcov$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoCovExtensionContract = sourcePathTokens.every(
    (token) => !/\.cov$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoTrxExtensionContract = sourcePathTokens.every(
    (token) => !/\.trx$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoTapExtensionContract = sourcePathTokens.every(
    (token) => !/\.tap$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoJunitXmlSuffixContract = sourcePathTokens.every(
    (token) => !/junit.*\.xml$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoNunitXmlSuffixContract = sourcePathTokens.every(
    (token) => !/nunit.*\.xml$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoXunitXmlSuffixContract = sourcePathTokens.every(
    (token) => !/xunit.*\.xml$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoHeapdumpExtensionContract = sourcePathTokens.every(
    (token) => !/\.heapdump$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoHprofExtensionContract = sourcePathTokens.every(
    (token) => !/\.hprof$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoCrashExtensionContract = sourcePathTokens.every(
    (token) => !/\.crash$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoCoreExtensionContract = sourcePathTokens.every(
    (token) => !/\.core$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoPerfettoTraceExtensionContract = sourcePathTokens.every(
    (token) => !/\.perfetto-trace$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoPidLockExtensionContract = sourcePathTokens.every(
    (token) => !/\.pid\.lock$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoSockExtensionContract = sourcePathTokens.every(
    (token) => !/\.sock$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoSocketExtensionContract = sourcePathTokens.every(
    (token) => !/\.socket$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoCoverageFinalJsonSuffixContract = sourcePathTokens.every(
    (token) => !/coverage-final\.json$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoNycOutputJsonSuffixContract = sourcePathTokens.every(
    (token) => !/nyc-output\.json$/i.test(cleanText(token, 260)),
  );
  const finalSourcePathsTailNoFlamegraphSvgSuffixContract = sourcePathTokens.every(
    (token) => !/flamegraph\.svg$/i.test(cleanText(token, 260)),
  );
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_profraw_extension_contract',
    ok: finalSourcePathsTailNoProfrawExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_profdata_extension_contract',
    ok: finalSourcePathsTailNoProfdataExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_gcov_extension_contract',
    ok: finalSourcePathsTailNoGcovExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_cov_extension_contract',
    ok: finalSourcePathsTailNoCovExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_trx_extension_contract',
    ok: finalSourcePathsTailNoTrxExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_tap_extension_contract',
    ok: finalSourcePathsTailNoTapExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_junit_xml_suffix_contract',
    ok: finalSourcePathsTailNoJunitXmlSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_nunit_xml_suffix_contract',
    ok: finalSourcePathsTailNoNunitXmlSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_xunit_xml_suffix_contract',
    ok: finalSourcePathsTailNoXunitXmlSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_heapdump_extension_contract',
    ok: finalSourcePathsTailNoHeapdumpExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_hprof_extension_contract',
    ok: finalSourcePathsTailNoHprofExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_crash_extension_contract',
    ok: finalSourcePathsTailNoCrashExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_core_extension_contract',
    ok: finalSourcePathsTailNoCoreExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_perfetto_trace_extension_contract',
    ok: finalSourcePathsTailNoPerfettoTraceExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_pid_lock_extension_contract',
    ok: finalSourcePathsTailNoPidLockExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_sock_extension_contract',
    ok: finalSourcePathsTailNoSockExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_socket_extension_contract',
    ok: finalSourcePathsTailNoSocketExtensionContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_coverage_final_json_suffix_contract',
    ok: finalSourcePathsTailNoCoverageFinalJsonSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_nyc_output_json_suffix_contract',
    ok: finalSourcePathsTailNoNycOutputJsonSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
  checks.push({
    id: 'eval_autopilot_final_source_paths_tail_no_flamegraph_svg_suffix_contract',
    ok: finalSourcePathsTailNoFlamegraphSvgSuffixContract,
    detail: `count=${sourcePathTokens.length}`,
  });
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
