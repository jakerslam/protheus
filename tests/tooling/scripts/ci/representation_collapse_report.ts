#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision, trackedFiles } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_OUT_JSON = 'core/local/artifacts/representation_collapse_report_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/REPRESENTATION_COLLAPSE_REPORT_CURRENT.md';

type RepresentationKind = 'source_of_truth' | 'projection' | 'cache' | 'detail';
type EntityId = 'message' | 'session' | 'tool_result' | 'trace' | 'workflow';

type EntityPolicy = {
  id: EntityId;
  label: string;
  tokens: string[];
  canonical_source_domains: string[];
  expected_detail_tokens: string[];
};

type RepresentationHit = {
  path: string;
  kind: RepresentationKind;
  matched_tokens: string[];
  risk_tokens: string[];
};

type EntityReport = {
  entity: EntityId;
  label: string;
  representations: Record<RepresentationKind, RepresentationHit[]>;
  counts: Record<RepresentationKind, number>;
  risks: Risk[];
};

type Risk = {
  kind: string;
  severity: 'info' | 'warning' | 'high';
  entity: EntityId;
  path?: string;
  detail: string;
};

const ENTITY_POLICIES: EntityPolicy[] = [
  {
    id: 'message',
    label: 'Chat/message entity',
    tokens: ['message', 'messages', 'conversation', 'chat_row', 'chat row', 'ShellMessageProjection'],
    canonical_source_domains: ['core/', 'orchestration/', 'adapters/'],
    expected_detail_tokens: ['detail_ref', 'message detail', 'message_detail'],
  },
  {
    id: 'session',
    label: 'Session/conversation entity',
    tokens: ['session', 'conversation_id', 'conversationCache', 'session_id'],
    canonical_source_domains: ['core/', 'orchestration/', 'adapters/'],
    expected_detail_tokens: ['session detail', 'session_detail', 'detail_ref'],
  },
  {
    id: 'tool_result',
    label: 'Tool result entity',
    tokens: ['tool_result', 'tool result', 'toolResult', 'tool.result', 'raw_tool_result'],
    canonical_source_domains: ['core/', 'orchestration/', 'adapters/'],
    expected_detail_tokens: ['tool result detail', 'tool_result_detail', 'detail_ref'],
  },
  {
    id: 'trace',
    label: 'Trace entity',
    tokens: ['trace_id', 'decision_trace', 'trace_body', 'trace detail', 'trace_detail'],
    canonical_source_domains: ['observability/', 'core/', 'orchestration/'],
    expected_detail_tokens: ['trace detail', 'trace_detail', 'detail_ref'],
  },
  {
    id: 'workflow',
    label: 'Workflow entity',
    tokens: ['workflow', 'workflow_graph', 'workflow detail', 'workflow_detail'],
    canonical_source_domains: ['orchestration/', 'core/'],
    expected_detail_tokens: ['workflow detail', 'workflow_detail', 'detail_ref'],
  },
];

const SCAN_ROOTS = ['core/', 'orchestration/', 'adapters/', 'client/', 'observability/', 'validation/'];
const SCAN_EXTENSIONS = new Set(['.rs', '.ts', '.tsx', '.html', '.md', '.json']);
const IGNORE_PARTS = [
  '/target/',
  '/node_modules/',
  '/vendor/',
  '.min.ts',
  'docs/workspace/SRS.md',
  'docs/workspace/todo/TODO.md',
  'docs/workspace/UPGRADE_BACKLOG.md',
];

const PROJECTION_TOKENS = ['projection', 'preview', 'detail_ref', 'row', 'render', 'display', 'summary'];
const CACHE_TOKENS = ['cache', 'localStorage', 'sessionStorage', 'window.__', 'conversationCache'];
const DETAIL_TOKENS = ['detail_ref', 'detail route', 'detail_route', 'message_detail', 'tool_result_detail', 'trace_detail', 'workflow_detail'];
const SOURCE_TOKENS = ['source of truth', 'canonical truth', 'authoritative', 'receipt', 'state transition'];
const RAW_IN_PROJECTION_TOKENS = [
  'raw_tool_result',
  'raw_tool_input',
  'rawToolResult',
  'tool.input',
  'tool.result',
  'trace_body',
  'plan_graph',
  'workflow_graph',
  'execution_observation',
  'all_messages',
  'conversation_tree',
  'full_state',
  'raw_runtime_state',
];

function abs(relPath: string): string {
  return path.resolve(ROOT, relPath);
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600),
    includeControlledViolation: parseBool(readFlag(argv, 'include-controlled-violation'), false),
  };
}

function readText(relPath: string): string {
  return fs.readFileSync(abs(relPath), 'utf8');
}

function shouldScan(file: string): boolean {
  if (!SCAN_ROOTS.some((root) => file.startsWith(root))) return false;
  if (!SCAN_EXTENSIONS.has(path.extname(file))) return false;
  return !IGNORE_PARTS.some((part) => file.includes(part));
}

function containsAny(source: string, tokens: string[]): string[] {
  const lower = source.toLowerCase();
  return tokens.filter((token) => lower.includes(token.toLowerCase()));
}

function topDomain(file: string): string {
  const [first] = file.split('/');
  return `${first || file}/`;
}

function classifyHit(file: string, source: string, matchedTokens: string[]): RepresentationKind | null {
  const lower = source.toLowerCase();
  if (containsAny(source, CACHE_TOKENS).length > 0) return 'cache';
  if (containsAny(source, DETAIL_TOKENS).length > 0) return 'detail';
  if (containsAny(source, PROJECTION_TOKENS).length > 0 || file.startsWith('client/')) return 'projection';
  if (containsAny(source, SOURCE_TOKENS).length > 0 || file.startsWith('core/')) return 'source_of_truth';
  if (file.startsWith('orchestration/') || file.startsWith('adapters/') || file.startsWith('observability/')) {
    return matchedTokens.length > 1 ? 'source_of_truth' : null;
  }
  return null;
}

function pushLimited(target: RepresentationHit[], hit: RepresentationHit): void {
  if (target.length < 25) target.push(hit);
}

function buildReport(includeControlledViolation: boolean) {
  const files = trackedFiles(ROOT).filter(shouldScan);
  const reports: EntityReport[] = ENTITY_POLICIES.map((policy) => ({
    entity: policy.id,
    label: policy.label,
    representations: {
      source_of_truth: [],
      projection: [],
      cache: [],
      detail: [],
    },
    counts: {
      source_of_truth: 0,
      projection: 0,
      cache: 0,
      detail: 0,
    },
    risks: [],
  }));
  const reportByEntity = new Map(reports.map((report) => [report.entity, report]));

  for (const file of files) {
    let source = '';
    try {
      source = readText(file);
    } catch {
      continue;
    }
    for (const policy of ENTITY_POLICIES) {
      const matchedTokens = containsAny(source, policy.tokens);
      if (matchedTokens.length === 0) continue;
      const kind = classifyHit(file, source, matchedTokens);
      if (!kind) continue;
      const riskTokens = containsAny(source, RAW_IN_PROJECTION_TOKENS);
      const report = reportByEntity.get(policy.id)!;
      report.counts[kind] += 1;
      pushLimited(report.representations[kind], {
        path: file,
        kind,
        matched_tokens: matchedTokens.slice(0, 8),
        risk_tokens: riskTokens.slice(0, 8),
      });
      if ((kind === 'projection' || kind === 'cache') && riskTokens.length > 0) {
        report.risks.push({
          kind: 'raw_in_projection_or_cache',
          severity: 'high',
          entity: policy.id,
          path: file,
          detail: `Projection/cache surface includes raw runtime tokens: ${riskTokens.slice(0, 6).join(', ')}`,
        });
      }
      if (file.startsWith('client/') && containsAny(source, SOURCE_TOKENS).length > 0) {
        report.risks.push({
          kind: 'shell_source_of_truth_language',
          severity: 'warning',
          entity: policy.id,
          path: file,
          detail: 'Shell path contains source-of-truth language; verify it is display-only.',
        });
      }
    }
  }

  for (const policy of ENTITY_POLICIES) {
    const report = reportByEntity.get(policy.id)!;
    const sourceDomains = new Set(report.representations.source_of_truth.map((hit) => topDomain(hit.path)));
    const unexpectedDomains = [...sourceDomains].filter((domain) => !policy.canonical_source_domains.includes(domain));
    if (unexpectedDomains.length > 0) {
      report.risks.push({
        kind: 'duplicate_truth_domain_risk',
        severity: 'warning',
        entity: policy.id,
        detail: `Source-of-truth-like representations appear outside canonical domains: ${unexpectedDomains.join(', ')}`,
      });
    }
    if (report.counts.detail === 0 || !report.representations.detail.some((hit) => containsAny(hit.matched_tokens.join(' '), policy.expected_detail_tokens).length > 0)) {
      report.risks.push({
        kind: 'missing_detail_representation_signal',
        severity: 'info',
        entity: policy.id,
        detail: 'No bounded detail representation signal was found in the tracked scan window.',
      });
    }
  }

  if (includeControlledViolation) {
    reports[0].risks.push({
      kind: 'controlled_raw_in_projection_violation',
      severity: 'high',
      entity: 'message',
      path: '__controlled_violation__',
      detail: 'Controlled violation injected for guard/regression testing.',
    });
    reports[1].risks.push({
      kind: 'controlled_cache_source_of_truth_violation',
      severity: 'high',
      entity: 'session',
      path: '__controlled_violation__',
      detail: 'Controlled cache-as-source-of-truth violation injected for guard/regression testing.',
    });
    reports[4].risks.push({
      kind: 'controlled_duplicate_representation_violation',
      severity: 'high',
      entity: 'workflow',
      path: '__controlled_violation__',
      detail: 'Controlled duplicate representation violation injected for guard/regression testing.',
    });
  }

  const risks = reports.flatMap((report) => report.risks);
  return {
    ok: true,
    type: 'representation_collapse_report',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    summary: {
      entity_count: reports.length,
      scanned_file_count: files.length,
      risk_count: risks.length,
      high_risk_count: risks.filter((risk) => risk.severity === 'high').length,
      warning_risk_count: risks.filter((risk) => risk.severity === 'warning').length,
      info_risk_count: risks.filter((risk) => risk.severity === 'info').length,
    },
    entities: reports,
    risks,
  };
}

function renderMarkdown(report: ReturnType<typeof buildReport>): string {
  const lines = [
    '# Representation Collapse Report',
    '',
    `Generated: ${report.generated_at}`,
    `Revision: ${report.revision}`,
    '',
    `Scanned files: ${report.summary.scanned_file_count}`,
    `Risks: ${report.summary.risk_count} (high=${report.summary.high_risk_count}, warning=${report.summary.warning_risk_count}, info=${report.summary.info_risk_count})`,
    '',
    '| Entity | Source of truth | Projection | Cache | Detail | Risks |',
    '|---|---:|---:|---:|---:|---:|',
  ];
  for (const entity of report.entities) {
    lines.push(`| ${entity.entity} | ${entity.counts.source_of_truth} | ${entity.counts.projection} | ${entity.counts.cache} | ${entity.counts.detail} | ${entity.risks.length} |`);
  }
  lines.push('', '## Risks');
  if (report.risks.length === 0) {
    lines.push('', 'No representation-collapse risks detected.');
  } else {
    for (const risk of report.risks.slice(0, 80)) {
      lines.push(`- **${risk.severity}** ${risk.entity}/${risk.kind}${risk.path ? ` at \`${risk.path}\`` : ''}: ${risk.detail}`);
    }
  }
  lines.push('', '## Sample Representations');
  for (const entity of report.entities) {
    lines.push('', `### ${entity.label}`);
    for (const kind of ['source_of_truth', 'projection', 'cache', 'detail'] as RepresentationKind[]) {
      const samples = entity.representations[kind].slice(0, 6).map((hit) => `\`${hit.path}\``).join(', ');
      lines.push(`- ${kind}: ${samples || 'none detected'}`);
    }
  }
  return `${lines.join('\n')}\n`;
}

const args = parseArgs(process.argv.slice(2));
const report = buildReport(args.includeControlledViolation);
writeTextArtifact(args.outMarkdown, renderMarkdown(report));
process.exitCode = emitStructuredResult(report, {
  outPath: args.outJson,
  strict: false,
  ok: true,
  stdout: false,
});
process.stdout.write(`${JSON.stringify({
  ok: true,
  type: report.type,
  summary: report.summary,
  artifact_paths: [args.outJson, args.outMarkdown],
}, null, 2)}\n`);
