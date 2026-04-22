#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type ParsedTable = {
  headers: string[];
  rows: string[][];
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_closure_feature_alignment_guard_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    markdownPath: cleanText(
      readFlag(argv, 'out-markdown') ||
        'local/workspace/reports/RUNTIME_CLOSURE_FEATURE_ALIGNMENT_GUARD_CURRENT.md',
      400,
    ),
    templatePath: cleanText(
      readFlag(argv, 'template') || '.github/pull_request_template.md',
      400,
    ),
    boardPath: cleanText(
      readFlag(argv, 'board') || 'client/runtime/config/runtime_closure_board.json',
      400,
    ),
  };
}

function readTextBestEffort(filePath: string): string {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return '';
  }
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function sectionBody(markdown: string, heading: string): string {
  const token = `## ${heading}`;
  const start = markdown.indexOf(token);
  if (start < 0) return '';
  const bodyStart = start + token.length;
  const rest = markdown.slice(bodyStart);
  const nextHeadingOffset = rest.search(/\n##\s+/);
  if (nextHeadingOffset < 0) return rest.trim();
  return rest.slice(0, nextHeadingOffset).trim();
}

function parseMarkdownTable(section: string): ParsedTable | null {
  const lines = section
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith('|'));
  if (lines.length < 2) return null;
  const splitRow = (line: string): string[] =>
    line
      .split('|')
      .slice(1, -1)
      .map((cell) => cleanText(cell, 240));
  const headers = splitRow(lines[0]);
  const rows = lines
    .slice(2)
    .map(splitRow)
    .filter((row) => row.some((cell) => cell.length > 0));
  return { headers, rows };
}

function indexOfHeader(headers: string[], name: string): number {
  const target = cleanText(name, 120).toLowerCase();
  return headers.findIndex((header) => cleanText(header, 120).toLowerCase() === target);
}

function renderMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Runtime Closure Feature Alignment Guard');
  lines.push('');
  lines.push(`- generated_at: ${cleanText(payload?.generated_at || '', 80)}`);
  lines.push(`- revision: ${cleanText(payload?.revision || '', 120)}`);
  lines.push(`- pass: ${payload?.ok === true ? 'true' : 'false'}`);
  lines.push(`- github_event_name: ${cleanText(payload?.summary?.github_event_name || '', 80) || 'unknown'}`);
  lines.push(`- pr_body_checked: ${payload?.summary?.pr_body_checked === true ? 'true' : 'false'}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- template_marker_failures: ${Number(payload?.summary?.template_marker_failures || 0)}`);
  lines.push(`- major_row_failures: ${Number(payload?.summary?.major_row_failures || 0)}`);
  lines.push(`- capability_row_failures: ${Number(payload?.summary?.capability_row_failures || 0)}`);
  lines.push(`- failure_count: ${Number(payload?.summary?.failure_count || 0)}`);
  lines.push('');
  const failures = Array.isArray(payload?.failures) ? payload.failures : [];
  if (failures.length > 0) {
    lines.push('## Failures');
    for (const failure of failures) {
      lines.push(
        `- ${cleanText(failure?.id || 'unknown', 120)}: ${cleanText(failure?.detail || '', 260)}`,
      );
    }
    lines.push('');
  }
  return `${lines.join('\n')}\n`;
}

function writeMarkdown(filePath: string, body: string): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, body, 'utf8');
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const templatePath = path.resolve(root, args.templatePath);
  const boardPath = path.resolve(root, args.boardPath);
  const template = readTextBestEffort(templatePath);
  const board = readJsonBestEffort(boardPath);
  const bucketIds = new Set<string>(
    Array.isArray(board?.buckets)
      ? board.buckets
          .map((row: any) => cleanText(row?.id || '', 80))
          .filter(Boolean)
      : [],
  );
  const failures: Array<{ id: string; detail: string }> = [];

  const templateMarkers = [
    '## Runtime Closure Feature Alignment (required for major surface features)',
    '| Feature Surface | Scope (`major`/`minor`) | Runtime Closure Bucket | Validation Artifact / Gate |',
    'If any feature scope is `major`, each major feature maps to a runtime-closure bucket and directly validates it with a linked proof artifact, replay fixture, or release gate.',
    'Every visible capability change links to at least one proof artifact, replay fixture, or release gate.',
  ];
  for (const marker of templateMarkers) {
    if (!template.includes(marker)) {
      failures.push({
        id: 'runtime_closure_template_marker_missing',
        detail: marker,
      });
    }
  }

  const eventName = cleanText(process.env.GITHUB_EVENT_NAME || '', 80);
  const eventPath = cleanText(process.env.GITHUB_EVENT_PATH || '', 400);
  const eventPayload =
    eventPath.length > 0 && fs.existsSync(eventPath) ? readJsonBestEffort(eventPath) : null;
  const prBody = cleanText(eventPayload?.pull_request?.body || '', 40_000);
  const shouldCheckPrBody = eventName === 'pull_request' || eventName === 'pull_request_target';

  let majorRowsChecked = 0;
  let capabilityRowsChecked = 0;

  if (shouldCheckPrBody) {
    if (!prBody) {
      failures.push({
        id: 'runtime_closure_pr_body_missing',
        detail: 'pull_request.body is empty',
      });
    } else {
      const closureSection = sectionBody(prBody, 'Runtime Closure Feature Alignment');
      const closureTable = parseMarkdownTable(closureSection);
      if (!closureTable) {
        failures.push({
          id: 'runtime_closure_pr_alignment_table_missing',
          detail: 'Runtime Closure Feature Alignment table not found in PR body',
        });
      } else {
        const headers = closureTable.headers;
        const surfaceIx = indexOfHeader(headers, 'Feature Surface');
        const scopeIx = indexOfHeader(headers, 'Scope (`major`/`minor`)');
        const bucketIx = indexOfHeader(headers, 'Runtime Closure Bucket');
        const validationIx = indexOfHeader(headers, 'Validation Artifact / Gate');
        if ([surfaceIx, scopeIx, bucketIx, validationIx].some((ix) => ix < 0)) {
          failures.push({
            id: 'runtime_closure_pr_alignment_headers_missing',
            detail: headers.join('|'),
          });
        } else {
          for (const row of closureTable.rows) {
            const featureSurface = cleanText(row[surfaceIx] || '', 200);
            const scope = cleanText(row[scopeIx] || '', 80).toLowerCase();
            const bucket = cleanText(row[bucketIx] || '', 120);
            const validation = cleanText(row[validationIx] || '', 260);
            if (!featureSurface) continue;
            if (scope.includes('major')) {
              majorRowsChecked += 1;
              if (!bucket) {
                failures.push({
                  id: 'runtime_closure_pr_major_bucket_missing',
                  detail: featureSurface,
                });
              } else if (!bucketIds.has(bucket)) {
                failures.push({
                  id: 'runtime_closure_pr_major_bucket_unknown',
                  detail: `${featureSurface}:${bucket}`,
                });
              }
              if (!validation) {
                failures.push({
                  id: 'runtime_closure_pr_major_validation_missing',
                  detail: featureSurface,
                });
              }
            }
          }
        }
      }

      const capabilitySection = sectionBody(prBody, 'Capability Proof Burden');
      const capabilityTable = parseMarkdownTable(capabilitySection);
      if (!capabilityTable) {
        failures.push({
          id: 'runtime_closure_pr_capability_table_missing',
          detail: 'Capability Proof Burden table not found in PR body',
        });
      } else {
        const capabilityIx = indexOfHeader(capabilityTable.headers, 'Capability');
        const proofIx = indexOfHeader(
          capabilityTable.headers,
          'Proof Artifact / Replay Fixture / Gate',
        );
        if (capabilityIx < 0 || proofIx < 0) {
          failures.push({
            id: 'runtime_closure_pr_capability_headers_missing',
            detail: capabilityTable.headers.join('|'),
          });
        } else {
          for (const row of capabilityTable.rows) {
            const capability = cleanText(row[capabilityIx] || '', 200);
            const proof = cleanText(row[proofIx] || '', 260);
            if (!capability) continue;
            capabilityRowsChecked += 1;
            if (!proof) {
              failures.push({
                id: 'runtime_closure_pr_capability_proof_link_missing',
                detail: capability,
              });
            }
          }
        }
      }
    }
  }

  const payload = {
    ok: failures.length === 0,
    type: 'runtime_closure_feature_alignment_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    template_path: args.templatePath,
    board_path: args.boardPath,
    summary: {
      pass: failures.length === 0,
      github_event_name: eventName,
      pr_body_checked: shouldCheckPrBody,
      runtime_closure_bucket_count: bucketIds.size,
      major_rows_checked: majorRowsChecked,
      capability_rows_checked: capabilityRowsChecked,
      template_marker_failures: failures.filter(
        (row) => row.id === 'runtime_closure_template_marker_missing',
      ).length,
      major_row_failures: failures.filter((row) => row.id.includes('runtime_closure_pr_major_'))
        .length,
      capability_row_failures: failures.filter((row) =>
        row.id.includes('runtime_closure_pr_capability_'),
      ).length,
      failure_count: failures.length,
    },
    failures,
  };

  writeMarkdown(path.resolve(root, args.markdownPath), renderMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
