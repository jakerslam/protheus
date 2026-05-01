#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();

const DEFAULT_SRS_PATH = 'docs/workspace/SRS.md';
const DEFAULT_TODO_PATH = 'docs/workspace/TODO.md';
const DEFAULT_OUT_JSON = 'core/local/artifacts/srs_todo_section_guard_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/SRS_TODO_SECTION_GUARD_CURRENT.md';

type SrsStatus =
  | 'queued'
  | 'in_progress'
  | 'blocked'
  | 'blocked_external_prepared'
  | 'done'
  | 'existing-coverage-validated';

type SrsRow = {
  id: string;
  status: SrsStatus;
  section: string;
};

type SectionSummary = {
  section: string;
  queued: number;
  in_progress: number;
  blocked: number;
  blocked_external_prepared: number;
  done: number;
  existing_coverage_validated: number;
};

type RollupSummary = {
  total_rows: number;
  queued: number;
  in_progress: number;
  blocked: number;
  blocked_external_prepared: number;
  done: number;
  existing_coverage_validated: number;
};

type ParsedTodoSection = {
  checkbox: 'x' | ' ';
  summary: SectionSummary;
};

type Args = {
  strict: boolean;
  srsPath: string;
  todoPath: string;
  outJson: string;
  outMarkdown: string;
};

function rel(value: string): string {
  return path.relative(ROOT, value).replace(/\\/g, '/');
}

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, {
    strict: false,
    out: DEFAULT_OUT_JSON,
  });
  return {
    strict: common.strict,
    srsPath: cleanText(readFlag(argv, 'srs') || DEFAULT_SRS_PATH, 400),
    todoPath: cleanText(readFlag(argv, 'todo') || DEFAULT_TODO_PATH, 400),
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MD, 400),
  };
}

function parseSrsRows(markdown: string): SrsRow[] {
  const out: SrsRow[] = [];
  const lines = markdown.split('\n');
  let section = 'Uncategorized';
  for (const line of lines) {
    const sectionMatch = line.match(/^##\s+(.+)$/);
    if (sectionMatch) {
      section = sectionMatch[1].trim();
      continue;
    }
    const statusMatch = line.match(
      /^\|\s*(V[^|\n]+?)\s*\|\s*(queued|in_progress|blocked|blocked_external_prepared|done|existing-coverage-validated)\s*\|/i,
    );
    if (!statusMatch) continue;
    const id = statusMatch[1].trim().toUpperCase();
    if (!/^V[0-9A-Z._-]+$/.test(id)) continue;
    const status = statusMatch[2].toLowerCase() as SrsStatus;
    out.push({ id, status, section });
  }
  return out;
}

function summarizeBySection(rows: SrsRow[]): Map<string, SectionSummary> {
  const map = new Map<string, SectionSummary>();
  for (const row of rows) {
    if (!map.has(row.section)) {
      map.set(row.section, {
        section: row.section,
        queued: 0,
        in_progress: 0,
        blocked: 0,
        blocked_external_prepared: 0,
        done: 0,
        existing_coverage_validated: 0,
      });
    }
    const item = map.get(row.section)!;
    switch (row.status) {
      case 'queued':
        item.queued += 1;
        break;
      case 'in_progress':
        item.in_progress += 1;
        break;
      case 'blocked':
        item.blocked += 1;
        break;
      case 'blocked_external_prepared':
        item.blocked_external_prepared += 1;
        break;
      case 'done':
        item.done += 1;
        break;
      case 'existing-coverage-validated':
        item.existing_coverage_validated += 1;
        break;
      default:
        break;
    }
  }
  return map;
}

function summarizeRollup(rows: SrsRow[]): RollupSummary {
  const rollup: RollupSummary = {
    total_rows: rows.length,
    queued: 0,
    in_progress: 0,
    blocked: 0,
    blocked_external_prepared: 0,
    done: 0,
    existing_coverage_validated: 0,
  };
  for (const row of rows) {
    switch (row.status) {
      case 'queued':
        rollup.queued += 1;
        break;
      case 'in_progress':
        rollup.in_progress += 1;
        break;
      case 'blocked':
        rollup.blocked += 1;
        break;
      case 'blocked_external_prepared':
        rollup.blocked_external_prepared += 1;
        break;
      case 'done':
        rollup.done += 1;
        break;
      case 'existing-coverage-validated':
        rollup.existing_coverage_validated += 1;
        break;
      default:
        break;
    }
  }
  return rollup;
}

function parseTodoGlobalRollup(markdown: string): RollupSummary | null {
  const keys = [
    'total_rows',
    'queued',
    'in_progress',
    'blocked',
    'blocked_external_prepared',
    'done',
    'existing_coverage_validated',
  ] as const;
  const out: Record<string, number> = {};
  for (const key of keys) {
    const match = markdown.match(new RegExp(`^- ${key}:\\s*(\\d+)\\s*$`, 'm'));
    if (!match) return null;
    out[key] = Number(match[1]);
  }
  return out as RollupSummary;
}

function parseTodoSectionChecklist(markdown: string): Map<string, ParsedTodoSection> {
  const map = new Map<string, ParsedTodoSection>();
  const lines = markdown.split('\n');
  const start = lines.findIndex((line) => line.trim() === '## SRS Section Checklist');
  if (start < 0) return map;
  for (let idx = start + 1; idx < lines.length; idx += 1) {
    const line = lines[idx].trim();
    if (!line) continue;
    if (line.startsWith('## ')) break;
    const match = line.match(
      /^- \[([ x])\] (.+?) — queued=(\d+), in_progress=(\d+), blocked=(\d+), blocked_external_prepared=(\d+), done=(\d+), existing_coverage_validated=(\d+)\s*$/,
    );
    if (!match) continue;
    const checkbox = (match[1] === 'x' ? 'x' : ' ') as 'x' | ' ';
    const section = match[2].trim();
    map.set(section, {
      checkbox,
      summary: {
        section,
        queued: Number(match[3]),
        in_progress: Number(match[4]),
        blocked: Number(match[5]),
        blocked_external_prepared: Number(match[6]),
        done: Number(match[7]),
        existing_coverage_validated: Number(match[8]),
      },
    });
  }
  return map;
}

function expectedCheckbox(summary: SectionSummary): 'x' | ' ' {
  const open =
    summary.queued + summary.in_progress + summary.blocked + summary.blocked_external_prepared;
  return open === 0 ? 'x' : ' ';
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# SRS/TODO Section Consistency Guard');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push('');
  lines.push('## Summary');
  lines.push(`- missing_files: ${payload.summary.missing_files}`);
  lines.push(`- rollup_mismatches: ${payload.summary.rollup_mismatches}`);
  lines.push(`- section_missing_in_todo: ${payload.summary.section_missing_in_todo}`);
  lines.push(`- section_extra_in_todo: ${payload.summary.section_extra_in_todo}`);
  lines.push(`- section_count_mismatches: ${payload.summary.section_count_mismatches}`);
  lines.push(`- section_checkbox_mismatches: ${payload.summary.section_checkbox_mismatches}`);
  lines.push(`- violation_count: ${payload.summary.violation_count}`);
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) {
    lines.push('- none');
  } else {
    for (const item of payload.violations) {
      lines.push(`- ${item.type}: ${item.detail}`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const srsAbs = path.resolve(ROOT, args.srsPath);
  const todoAbs = path.resolve(ROOT, args.todoPath);
  const missingFiles: string[] = [];
  if (!fs.existsSync(srsAbs)) missingFiles.push(rel(srsAbs));
  if (!fs.existsSync(todoAbs)) missingFiles.push(rel(todoAbs));

  const violations: Array<{ type: string; detail: string }> = [];
  for (const missingFile of missingFiles) {
    violations.push({ type: 'missing_file', detail: missingFile });
  }

  let rollupMismatchCount = 0;
  let sectionMissingInTodo = 0;
  let sectionExtraInTodo = 0;
  let sectionCountMismatchCount = 0;
  let sectionCheckboxMismatchCount = 0;

  if (!missingFiles.length) {
    const srsSource = fs.readFileSync(srsAbs, 'utf8');
    const todoSource = fs.readFileSync(todoAbs, 'utf8');
    const srsRows = parseSrsRows(srsSource);
    const expectedRollup = summarizeRollup(srsRows);
    const expectedSections = summarizeBySection(srsRows);
    const todoRollup = parseTodoGlobalRollup(todoSource);
    const todoSections = parseTodoSectionChecklist(todoSource);

    if (!todoRollup) {
      violations.push({
        type: 'todo_rollup_missing',
        detail:
          'docs/workspace/TODO.md is missing one or more required Global Rollup keys (total_rows, queued, in_progress, blocked, blocked_external_prepared, done, existing_coverage_validated)',
      });
    } else {
      const rollupKeys: Array<keyof RollupSummary> = [
        'total_rows',
        'queued',
        'in_progress',
        'blocked',
        'blocked_external_prepared',
        'done',
        'existing_coverage_validated',
      ];
      for (const key of rollupKeys) {
        if (todoRollup[key] !== expectedRollup[key]) {
          rollupMismatchCount += 1;
          violations.push({
            type: 'todo_rollup_mismatch',
            detail: `${key}: todo=${todoRollup[key]} expected=${expectedRollup[key]}`,
          });
        }
      }
    }

    for (const [section, expected] of expectedSections.entries()) {
      const todo = todoSections.get(section);
      if (!todo) {
        sectionMissingInTodo += 1;
        violations.push({
          type: 'todo_section_missing',
          detail: section,
        });
        continue;
      }
      const keys: Array<keyof Omit<SectionSummary, 'section'>> = [
        'queued',
        'in_progress',
        'blocked',
        'blocked_external_prepared',
        'done',
        'existing_coverage_validated',
      ];
      for (const key of keys) {
        if (todo.summary[key] !== expected[key]) {
          sectionCountMismatchCount += 1;
          violations.push({
            type: 'todo_section_count_mismatch',
            detail: `${section} :: ${key}: todo=${todo.summary[key]} expected=${expected[key]}`,
          });
        }
      }
      const expectedCheck = expectedCheckbox(expected);
      if (todo.checkbox !== expectedCheck) {
        sectionCheckboxMismatchCount += 1;
        violations.push({
          type: 'todo_section_checkbox_mismatch',
          detail: `${section}: todo=[${todo.checkbox}] expected=[${expectedCheck}]`,
        });
      }
    }

    for (const section of todoSections.keys()) {
      if (!expectedSections.has(section)) {
        sectionExtraInTodo += 1;
        violations.push({
          type: 'todo_section_unknown',
          detail: section,
        });
      }
    }
  }

  const ok = violations.length === 0;
  const payload = {
    type: 'srs_todo_section_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict: args.strict,
    srs_path: rel(srsAbs),
    todo_path: rel(todoAbs),
    summary: {
      pass: ok,
      missing_files: missingFiles.length,
      rollup_mismatches: rollupMismatchCount,
      section_missing_in_todo: sectionMissingInTodo,
      section_extra_in_todo: sectionExtraInTodo,
      section_count_mismatches: sectionCountMismatchCount,
      section_checkbox_mismatches: sectionCheckboxMismatchCount,
      violation_count: violations.length,
    },
    missing_files: missingFiles,
    violations,
  };

  writeTextArtifact(path.resolve(ROOT, args.outMarkdown), toMarkdown({ ...payload, ok }));
  process.exit(
    emitStructuredResult(payload, {
      outPath: path.resolve(ROOT, args.outJson),
      strict: args.strict,
      ok,
    }),
  );
}

main();
