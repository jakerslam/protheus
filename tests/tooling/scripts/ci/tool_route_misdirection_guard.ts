#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type Route = 'workspace_read' | 'workspace_search' | 'tool_route' | 'web_search' | 'web_fetch';

type MatrixCase = {
  id: string;
  intent: string;
  expected_route: Route;
  rejected_routes: Route[];
  reason_codes: string[];
  evidence_tests: string[];
};

type DecisionRow = {
  request_id: string;
  route: Route;
  rejected_routes: Route[];
  reason_codes: string[];
  evidence_tests: string[];
  missing_evidence_tests: string[];
  local_intent: boolean;
  route_misdirected: boolean;
  rejected_route_contract_ok: boolean;
  ok: boolean;
};

type FailureRow = {
  id: string;
  detail: string;
};

const ALLOWED_ROUTES = new Set<Route>([
  'workspace_read',
  'workspace_search',
  'tool_route',
  'web_search',
  'web_fetch',
]);

const WEB_ROUTES = new Set<Route>(['web_search', 'web_fetch']);
const LOCAL_ROUTES = new Set<Route>(['workspace_read', 'workspace_search', 'tool_route']);
const REQUIRED_LOCAL_REJECTED_ROUTES: Route[] = ['web_search', 'web_fetch'];
const CASE_ID_TOKEN = /^[a-z0-9][a-z0-9_-]*$/;
const REASON_CODE_TOKEN = /^[a-z0-9][a-z0-9._:-]*$/;
const EVIDENCE_TEST_TOKEN = /^[a-zA-Z_][a-zA-Z0-9_]*$/;

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/tool_route_decision_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    markdownPath: cleanText(
      readFlag(argv, 'out-markdown') ||
        'local/workspace/reports/TOOL_ROUTE_MISDIRECTION_GUARD_CURRENT.md',
      400,
    ),
    fixturePath: cleanText(
      readFlag(argv, 'fixture') || 'tests/tooling/fixtures/tool_route_misdirection_matrix.json',
      400,
    ),
    ingressPath: cleanText(readFlag(argv, 'ingress') || 'surface/orchestration/src/ingress.rs', 400),
    classifierPath: cleanText(
      readFlag(argv, 'classifier') || 'surface/orchestration/src/request_classifier.rs',
      400,
    ),
  };
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function readTextBestEffort(filePath: string): string {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return '';
  }
}

function duplicateValues(values: string[]): string[] {
  return values.filter((value, index, all) => all.indexOf(value) !== index);
}

function isCanonicalRelativePath(value: string, requiredPrefix = ''): boolean {
  const normalized = cleanText(value || '', 400);
  if (!normalized) return false;
  if (path.isAbsolute(normalized)) return false;
  if (normalized.includes('\\')) return false;
  if (normalized.includes('..')) return false;
  if (normalized.includes('//')) return false;
  if (requiredPrefix && !normalized.startsWith(requiredPrefix)) return false;
  return true;
}

function parseCases(raw: unknown): MatrixCase[] {
  if (!Array.isArray(raw)) return [];
  const cases: MatrixCase[] = [];
  for (const row of raw) {
    const id = cleanText((row as any)?.id || '', 120);
    const intent = cleanText((row as any)?.intent || '', 240);
    const expectedRoute = cleanText((row as any)?.expected_route || '', 80) as Route;
    const rejectedRoutes = Array.isArray((row as any)?.rejected_routes)
      ? (row as any).rejected_routes
          .map((value: any) => cleanText(value || '', 80) as Route)
          .filter((value: Route) => Boolean(value))
      : [];
    const reasonCodes = Array.isArray((row as any)?.reason_codes)
      ? (row as any).reason_codes.map((value: any) => cleanText(value || '', 120)).filter(Boolean)
      : [];
    const evidenceTests = Array.isArray((row as any)?.evidence_tests)
      ? (row as any).evidence_tests.map((value: any) => cleanText(value || '', 160)).filter(Boolean)
      : [];
    if (!id) continue;
    cases.push({
      id,
      intent,
      expected_route: expectedRoute,
      rejected_routes: rejectedRoutes,
      reason_codes: reasonCodes,
      evidence_tests: evidenceTests,
    });
  }
  return cases;
}

function hasRustTest(source: string, testName: string): boolean {
  const escaped = testName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const testPattern = new RegExp(`fn\\s+${escaped}\\s*\\(`, 'm');
  return testPattern.test(source);
}

function renderMarkdown(report: any): string {
  const lines: string[] = [];
  lines.push('# Tool Route Misdirection Guard (Current)');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(report?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(report?.revision || '', 120)}`);
  lines.push(`- pass: ${report?.ok === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- total_cases: ${Number(report?.summary?.total_cases || 0)}`);
  lines.push(`- local_intent_cases: ${Number(report?.summary?.local_intent_cases || 0)}`);
  lines.push(`- misdirection_count: ${Number(report?.summary?.misdirection_count || 0)}`);
  lines.push(`- missing_evidence_cases: ${Number(report?.summary?.missing_evidence_cases || 0)}`);
  lines.push(`- failing_case_count: ${Number(report?.summary?.failing_case_count || 0)}`);
  lines.push('');
  lines.push('## Decisions');
  const decisions = Array.isArray(report?.decisions) ? report.decisions : [];
  for (const row of decisions) {
    lines.push(
      `- ${cleanText(row?.request_id || 'unknown', 120)}: route=${cleanText(
        row?.route || '',
        60,
      )} local_intent=${row?.local_intent === true ? 'true' : 'false'} misdirected=${row?.route_misdirected === true ? 'true' : 'false'} rejected_ok=${row?.rejected_route_contract_ok === true ? 'true' : 'false'} missing_evidence=${(Array.isArray(row?.missing_evidence_tests) ? row.missing_evidence_tests : []).join(',') || 'none'}`,
    );
  }
  const failures = Array.isArray(report?.failures) ? report.failures : [];
  if (failures.length > 0) {
    lines.push('');
    lines.push('## Failures');
    for (const failure of failures) {
      lines.push(
        `- ${cleanText(failure?.id || 'unknown', 120)}: ${cleanText(failure?.detail || '', 240)}`,
      );
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const fixtureAbs = path.resolve(root, args.fixturePath);
  const ingressAbs = path.resolve(root, args.ingressPath);
  const classifierAbs = path.resolve(root, args.classifierPath);
  const markdownAbs = path.resolve(root, args.markdownPath);

  const failures: FailureRow[] = [];
  const fixture = readJsonBestEffort(fixtureAbs);
  const cases = parseCases(fixture?.cases);

  if (cleanText(fixture?.schema_id || '', 120) !== 'tool_route_misdirection_matrix') {
    failures.push({
      id: 'fixture_schema_id_invalid',
      detail: `schema_id expected tool_route_misdirection_matrix got ${cleanText(fixture?.schema_id || 'missing', 120)}`,
    });
  }
  if (Number(fixture?.schema_version || 0) !== 1) {
    failures.push({
      id: 'fixture_schema_version_invalid',
      detail: `schema_version expected 1 got ${cleanText(String(fixture?.schema_version ?? 'missing'), 40)}`,
    });
  }
  if (!isCanonicalRelativePath(args.outPath, 'core/local/artifacts/')) {
    failures.push({ id: 'out_path_not_canonical', detail: 'out path must be canonical relative core/local/artifacts/' });
  }
  if (!cleanText(args.outPath, 400).endsWith('_current.json')) {
    failures.push({ id: 'out_path_not_current_json', detail: 'out path must end with _current.json' });
  }
  if (!isCanonicalRelativePath(args.fixturePath, 'tests/tooling/fixtures/')) {
    failures.push({ id: 'fixture_path_not_canonical', detail: 'fixture path must be canonical under tests/tooling/fixtures/' });
  }
  if (!isCanonicalRelativePath(args.markdownPath, 'local/workspace/reports/')) {
    failures.push({
      id: 'markdown_path_not_canonical',
      detail: 'markdown path must be canonical under local/workspace/reports/',
    });
  }
  if (
    cleanText(args.markdownPath, 400) !==
    'local/workspace/reports/TOOL_ROUTE_MISDIRECTION_GUARD_CURRENT.md'
  ) {
    failures.push({
      id: 'markdown_path_contract_drift',
      detail: `markdown path contract drift: ${args.markdownPath}`,
    });
  }
  if (!isCanonicalRelativePath(args.ingressPath, 'surface/orchestration/src/')) {
    failures.push({
      id: 'ingress_path_not_canonical',
      detail: 'ingress path must be canonical under surface/orchestration/src/',
    });
  }
  if (!isCanonicalRelativePath(args.classifierPath, 'surface/orchestration/src/')) {
    failures.push({
      id: 'classifier_path_not_canonical',
      detail: 'classifier path must be canonical under surface/orchestration/src/',
    });
  }
  if (!fs.existsSync(fixtureAbs)) {
    failures.push({
      id: 'fixture_file_missing',
      detail: args.fixturePath,
    });
  }
  if (cases.length === 0) {
    failures.push({
      id: 'fixture_cases_empty',
      detail: args.fixturePath,
    });
  }

  const ids = cases.map((row) => row.id);
  const duplicateIds = Array.from(new Set(duplicateValues(ids)));
  if (duplicateIds.length > 0) {
    failures.push({
      id: 'fixture_case_ids_duplicate',
      detail: `duplicate case ids: ${duplicateIds.join(',')}`,
    });
  }
  const noncanonicalCaseIds = cases
    .filter((row) => !CASE_ID_TOKEN.test(row.id))
    .map((row) => row.id);
  if (noncanonicalCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_ids_noncanonical',
      detail: Array.from(new Set(noncanonicalCaseIds)).join(','),
    });
  }
  const caseIdsSortedCanonical = ids.join('|') === [...ids].sort().join('|');
  if (!caseIdsSortedCanonical) {
    failures.push({
      id: 'fixture_case_ids_order_noncanonical',
      detail: ids.join(','),
    });
  }
  const intentMissingCaseIds = cases.filter((row) => row.intent.length === 0).map((row) => row.id);
  if (intentMissingCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_intent_missing',
      detail: intentMissingCaseIds.join(','),
    });
  }
  const rejectedRoutesDuplicateCaseIds = cases
    .filter((row) => duplicateValues(row.rejected_routes).length > 0)
    .map((row) => row.id);
  if (rejectedRoutesDuplicateCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_rejected_routes_duplicate',
      detail: rejectedRoutesDuplicateCaseIds.join(','),
    });
  }
  const rejectedRoutesNoncanonicalCaseIds = cases
    .filter((row) => row.rejected_routes.some((route) => !ALLOWED_ROUTES.has(route)))
    .map((row) => row.id);
  if (rejectedRoutesNoncanonicalCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_rejected_routes_noncanonical',
      detail: rejectedRoutesNoncanonicalCaseIds.join(','),
    });
  }
  const reasonCodesDuplicateCaseIds = cases
    .filter((row) => duplicateValues(row.reason_codes).length > 0)
    .map((row) => row.id);
  if (reasonCodesDuplicateCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_reason_codes_duplicate',
      detail: reasonCodesDuplicateCaseIds.join(','),
    });
  }
  const reasonCodesNoncanonicalCaseIds = cases
    .filter(
      (row) => row.reason_codes.length === 0 || row.reason_codes.some((code) => !REASON_CODE_TOKEN.test(code)),
    )
    .map((row) => row.id);
  if (reasonCodesNoncanonicalCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_reason_codes_noncanonical',
      detail: reasonCodesNoncanonicalCaseIds.join(','),
    });
  }
  const evidenceTestsDuplicateCaseIds = cases
    .filter((row) => duplicateValues(row.evidence_tests).length > 0)
    .map((row) => row.id);
  if (evidenceTestsDuplicateCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_evidence_tests_duplicate',
      detail: evidenceTestsDuplicateCaseIds.join(','),
    });
  }
  const evidenceTestsNoncanonicalCaseIds = cases
    .filter(
      (row) =>
        row.evidence_tests.length === 0 || row.evidence_tests.some((testName) => !EVIDENCE_TEST_TOKEN.test(testName)),
    )
    .map((row) => row.id);
  if (evidenceTestsNoncanonicalCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_evidence_tests_noncanonical',
      detail: evidenceTestsNoncanonicalCaseIds.join(','),
    });
  }
  const localRouteMissingLocalReasonCaseIds = cases
    .filter(
      (row) =>
        LOCAL_ROUTES.has(row.expected_route) &&
        !row.reason_codes.includes('local_workspace_intent') &&
        !row.reason_codes.includes('local_tool_intent'),
    )
    .map((row) => row.id);
  if (localRouteMissingLocalReasonCaseIds.length > 0) {
    failures.push({
      id: 'fixture_case_local_route_missing_local_reason_code',
      detail: localRouteMissingLocalReasonCaseIds.join(','),
    });
  }

  const source = `${readTextBestEffort(ingressAbs)}\n${readTextBestEffort(classifierAbs)}`;
  const decisions: DecisionRow[] = [];
  for (const row of cases) {
    if (!ALLOWED_ROUTES.has(row.expected_route)) {
      failures.push({
        id: `case:${row.id}:route_invalid`,
        detail: `expected_route ${row.expected_route} is not allowed`,
      });
      continue;
    }
    const localIntent =
      row.reason_codes.includes('local_workspace_intent') || row.reason_codes.includes('local_tool_intent');
    const routeMisdirected = localIntent && WEB_ROUTES.has(row.expected_route);
    const rejectedRouteContractOk =
      !localIntent ||
      REQUIRED_LOCAL_REJECTED_ROUTES.every((requiredRoute) => row.rejected_routes.includes(requiredRoute));
    const missingEvidenceTests = row.evidence_tests.filter((testName) => !hasRustTest(source, testName));
    const caseOk = !routeMisdirected && rejectedRouteContractOk && missingEvidenceTests.length === 0;
    decisions.push({
      request_id: row.id,
      route: row.expected_route,
      rejected_routes: row.rejected_routes,
      reason_codes: row.reason_codes,
      evidence_tests: row.evidence_tests,
      missing_evidence_tests: missingEvidenceTests,
      local_intent: localIntent,
      route_misdirected: routeMisdirected,
      rejected_route_contract_ok: rejectedRouteContractOk,
      ok: caseOk,
    });
    if (routeMisdirected) {
      failures.push({
        id: `case:${row.id}:route_misdirection`,
        detail: `local intent case routed to web capability ${row.expected_route}`,
      });
    }
    if (!rejectedRouteContractOk) {
      failures.push({
        id: `case:${row.id}:rejected_routes_missing`,
        detail: `local intent case must reject ${REQUIRED_LOCAL_REJECTED_ROUTES.join(',')}`,
      });
    }
    if (missingEvidenceTests.length > 0) {
      failures.push({
        id: `case:${row.id}:evidence_tests_missing`,
        detail: `missing evidence tests: ${missingEvidenceTests.join(',')}`,
      });
    }
  }

  const localIntentCases = decisions.filter((row) => row.local_intent);
  const misdirectionCount = localIntentCases.filter((row) => row.route_misdirected).length;
  const missingEvidenceCases = decisions.filter((row) => row.missing_evidence_tests.length > 0).length;
  const failingCaseCount = decisions.filter((row) => !row.ok).length;
  const decisionIds = decisions.map((row) => row.request_id);
  const decisionDuplicateIds = Array.from(new Set(duplicateValues(decisionIds)));
  if (decisionDuplicateIds.length > 0) {
    failures.push({
      id: 'decisions_request_ids_duplicate',
      detail: decisionDuplicateIds.join(','),
    });
  }
  const misdirectionCountFromDecisions = decisions.filter(
    (row) => row.local_intent && WEB_ROUTES.has(row.route),
  ).length;
  if (misdirectionCount !== misdirectionCountFromDecisions) {
    failures.push({
      id: 'summary_misdirection_count_mismatch',
      detail: `summary=${misdirectionCount};derived=${misdirectionCountFromDecisions}`,
    });
  }
  const missingEvidenceCountFromDecisions = decisions.filter(
    (row) => row.missing_evidence_tests.length > 0,
  ).length;
  if (missingEvidenceCases !== missingEvidenceCountFromDecisions) {
    failures.push({
      id: 'summary_missing_evidence_cases_mismatch',
      detail: `summary=${missingEvidenceCases};derived=${missingEvidenceCountFromDecisions}`,
    });
  }
  const failingCountFromDecisions = decisions.filter((row) => row.ok !== true).length;
  if (failingCaseCount !== failingCountFromDecisions) {
    failures.push({
      id: 'summary_failing_case_count_mismatch',
      detail: `summary=${failingCaseCount};derived=${failingCountFromDecisions}`,
    });
  }

  const report = {
    ok: failures.length === 0,
    type: 'tool_route_misdirection_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    inputs: {
      strict: args.strict,
      out: args.outPath,
      fixture: args.fixturePath,
      ingress: args.ingressPath,
      classifier: args.classifierPath,
      markdown: args.markdownPath,
    },
    summary: {
      total_cases: decisions.length,
      local_intent_cases: localIntentCases.length,
      misdirection_count: misdirectionCount,
      missing_evidence_cases: missingEvidenceCases,
      failing_case_count: failingCaseCount,
    },
    decisions,
    failures,
  };

  writeTextArtifact(markdownAbs, renderMarkdown(report));
  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (import.meta.url === `file://${process.argv[1]}`) {
  process.exit(run());
}
