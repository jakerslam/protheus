#!/usr/bin/env node
/* eslint-disable no-console */
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { extname, resolve } from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const DEFAULT_GROWTH_POLICY = 'client/runtime/config/shell_alpine_growth_policy.json';
const DEFAULT_OWNERSHIP_MAP = 'client/runtime/config/shell_alpine_ownership_map.json';
const DEFAULT_OUT_JSON = 'core/local/artifacts/shell_alpine_ownership_inventory_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/SHELL_ALPINE_OWNERSHIP_INVENTORY_CURRENT.md';
const REQUIRED_CLASSES = ['bootstrap_only', 'interactive', 'delete_ready'];

type PatternConfig = {
  id: string;
  description?: string;
  regex: string;
};

type GrowthPolicy = {
  scan_roots?: string[];
  scan_extensions?: string[];
  ignore_path_contains?: string[];
  patterns?: PatternConfig[];
};

type OwnershipRule = {
  id: string;
  owner_feature: string;
  target_replacement: string;
  migration_class: string;
  paths?: string[];
  path_prefixes?: string[];
  path_contains?: string[];
  applies_to_patterns: string[];
  notes?: string;
};

type OwnershipMap = {
  version?: string;
  srs_id?: string;
  source_growth_policy?: string;
  migration_classes?: Record<string, string>;
  rules?: OwnershipRule[];
};

type Args = {
  growthPolicyPath: string;
  ownershipMapPath: string;
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

type InventoryRow = {
  file: string;
  pattern_id: string;
  description: string;
  count: number;
  owner_rule_id: string;
  owner_feature: string;
  target_replacement: string;
  migration_class: string;
};

type Violation = {
  kind: string;
  rule_id?: string;
  pattern_id?: string;
  path?: string;
  detail: string;
};

function readArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: true, out: DEFAULT_OUT_JSON });
  return {
    growthPolicyPath: cleanText(readFlag(argv, 'growth-policy') || DEFAULT_GROWTH_POLICY, 400),
    ownershipMapPath: cleanText(readFlag(argv, 'ownership-map') || DEFAULT_OWNERSHIP_MAP, 400),
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 400),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 400),
  };
}

function readJson<T>(path: string): T {
  return JSON.parse(readFileSync(resolve(ROOT, path), 'utf8')) as T;
}

function gitFiles(args: string[]): string[] {
  try {
    const output = execFileSync('git', args, { cwd: ROOT, encoding: 'utf8' });
    return output.split('\0').map((file) => file.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

function shellFiles(policy: GrowthPolicy): string[] {
  const roots = policy.scan_roots ?? [];
  const extensions = new Set(policy.scan_extensions ?? []);
  const ignored = policy.ignore_path_contains ?? [];
  const files = new Set([...gitFiles(['ls-files', '-z']), ...gitFiles(['ls-files', '--others', '--exclude-standard', '-z'])]);
  return [...files].filter((file) => {
    const underRoot = roots.length === 0 || roots.some((root) => file === root || file.startsWith(`${root}/`));
    const hasExtension = extensions.size === 0 || extensions.has(extname(file));
    const isIgnored = ignored.some((needle) => needle && file.includes(needle));
    return underRoot && hasExtension && !isIgnored && existsSync(resolve(ROOT, file));
  }).sort();
}

function duplicateValues(values: string[]): string[] {
  const seen = new Set<string>();
  const dupes = new Set<string>();
  for (const value of values) {
    if (seen.has(value)) dupes.add(value);
    seen.add(value);
  }
  return [...dupes].sort();
}

function compile(pattern: PatternConfig): RegExp | null {
  try {
    return new RegExp(pattern.regex, 'g');
  } catch {
    return null;
  }
}

function countMatches(source: string, regex: RegExp): number {
  regex.lastIndex = 0;
  let count = 0;
  let match = regex.exec(source);
  while (match) {
    count += 1;
    if (match[0] === '') regex.lastIndex += 1;
    match = regex.exec(source);
  }
  return count;
}

function ruleMatchesPath(rule: OwnershipRule, file: string): boolean {
  const exact = rule.paths ?? [];
  const prefixes = rule.path_prefixes ?? [];
  const contains = rule.path_contains ?? [];
  return exact.includes(file) || prefixes.some((prefix) => file.startsWith(prefix)) || contains.some((needle) => file.includes(needle));
}

function ruleMatchesPattern(rule: OwnershipRule, patternId: string): boolean {
  return (rule.applies_to_patterns ?? []).includes(patternId);
}

function findRules(rules: OwnershipRule[], file: string, patternId: string): OwnershipRule[] {
  return rules.filter((rule) => ruleMatchesPath(rule, file) && ruleMatchesPattern(rule, patternId));
}

function validateOwnershipMap(map: OwnershipMap, policy: GrowthPolicy, args: Args): Violation[] {
  const violations: Violation[] = [];
  const patternIds = new Set((policy.patterns ?? []).map((pattern) => pattern.id));
  const classNames = new Set(Object.keys(map.migration_classes ?? {}));
  for (const className of REQUIRED_CLASSES) {
    if (!classNames.has(className)) {
      violations.push({
        kind: 'missing_migration_class',
        detail: `Ownership map must define migration class ${className}.`,
      });
    }
  }
  if (map.source_growth_policy !== args.growthPolicyPath) {
    violations.push({
      kind: 'growth_policy_mismatch',
      detail: `Ownership map source policy must match ${args.growthPolicyPath}.`,
    });
  }
  for (const duplicate of duplicateValues((map.rules ?? []).map((rule) => rule.id))) {
    violations.push({
      kind: 'duplicate_rule_id',
      rule_id: duplicate,
      detail: 'Ownership rule IDs must be unique.',
    });
  }
  for (const rule of map.rules ?? []) {
    const matcherCount = (rule.paths?.length ?? 0) + (rule.path_prefixes?.length ?? 0) + (rule.path_contains?.length ?? 0);
    if (!rule.owner_feature.trim()) {
      violations.push({ kind: 'missing_owner_feature', rule_id: rule.id, detail: 'Ownership rule must name the owning shell feature.' });
    }
    if (!rule.target_replacement.trim()) {
      violations.push({ kind: 'missing_target_replacement', rule_id: rule.id, detail: 'Ownership rule must name the target replacement.' });
    }
    const target = rule.target_replacement.toLowerCase();
    if (!target.includes('svelte') && !target.includes('shared shell')) {
      violations.push({
        kind: 'target_not_shell_migration',
        rule_id: rule.id,
        detail: 'Target replacement must explicitly name Svelte or shared shell services.',
      });
    }
    if (!REQUIRED_CLASSES.includes(rule.migration_class)) {
      violations.push({
        kind: 'invalid_migration_class',
        rule_id: rule.id,
        detail: `Migration class must be one of ${REQUIRED_CLASSES.join(', ')}.`,
      });
    }
    if (matcherCount === 0) {
      violations.push({ kind: 'missing_path_matcher', rule_id: rule.id, detail: 'Ownership rule must include at least one path matcher.' });
    }
    if (!rule.applies_to_patterns?.length) {
      violations.push({ kind: 'missing_pattern_matcher', rule_id: rule.id, detail: 'Ownership rule must include at least one Alpine pattern matcher.' });
    }
    for (const patternId of rule.applies_to_patterns ?? []) {
      if (!patternIds.has(patternId)) {
        violations.push({
          kind: 'unknown_pattern_id',
          rule_id: rule.id,
          pattern_id: patternId,
          detail: 'Ownership rule references an Alpine pattern that is not in the growth policy.',
        });
      }
    }
  }
  return violations;
}

function inventory(policy: GrowthPolicy, map: OwnershipMap, files: string[]): { rows: InventoryRow[]; violations: Violation[] } {
  const rows: InventoryRow[] = [];
  const violations: Violation[] = [];
  const rules = map.rules ?? [];
  for (const pattern of policy.patterns ?? []) {
    const regex = compile(pattern);
    if (!regex) {
      violations.push({ kind: 'invalid_regex', pattern_id: pattern.id, detail: pattern.regex });
      continue;
    }
    for (const file of files) {
      const count = countMatches(readFileSync(resolve(ROOT, file), 'utf8'), regex);
      if (count === 0) continue;
      const matches = findRules(rules, file, pattern.id);
      if (matches.length === 0) {
        violations.push({
          kind: 'unowned_alpine_usage',
          pattern_id: pattern.id,
          path: file,
          detail: 'Current Alpine usage has no ownership/migration-boundary rule.',
        });
        continue;
      }
      if (matches.length > 1) {
        violations.push({
          kind: 'ambiguous_alpine_owner',
          pattern_id: pattern.id,
          path: file,
          detail: `Current Alpine usage matches multiple ownership rules: ${matches.map((rule) => rule.id).join(', ')}.`,
        });
        continue;
      }
      const rule = matches[0];
      rows.push({
        file,
        pattern_id: pattern.id,
        description: pattern.description ?? '',
        count,
        owner_rule_id: rule.id,
        owner_feature: rule.owner_feature,
        target_replacement: rule.target_replacement,
        migration_class: rule.migration_class,
      });
    }
  }
  return { rows, violations };
}

function classSummary(rows: InventoryRow[]): Record<string, number> {
  const summary: Record<string, number> = Object.fromEntries(REQUIRED_CLASSES.map((className) => [className, 0]));
  for (const row of rows) {
    summary[row.migration_class] = (summary[row.migration_class] ?? 0) + row.count;
  }
  return summary;
}

function markdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# Shell Alpine Ownership Inventory');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push(`- pass: ${payload.ok}`);
  lines.push(`- scanned_files: ${payload.summary.scanned_files}`);
  lines.push(`- inventory_rows: ${payload.summary.inventory_rows}`);
  lines.push(`- total_alpine_hits: ${payload.summary.total_alpine_hits}`);
  lines.push(`- violations: ${payload.summary.violations}`);
  lines.push('');
  lines.push('## Migration Class Counts');
  lines.push('| class | alpine hits |');
  lines.push('| --- | ---: |');
  for (const [className, count] of Object.entries(payload.summary.migration_class_counts)) {
    lines.push(`| ${className} | ${count} |`);
  }
  lines.push('');
  lines.push('## Ownership Rows');
  lines.push('| file | pattern | hits | class | owner | replacement |');
  lines.push('| --- | --- | ---: | --- | --- | --- |');
  for (const row of payload.inventory) {
    lines.push(`| ${row.file} | ${row.pattern_id} | ${row.count} | ${row.migration_class} | ${row.owner_feature} | ${row.target_replacement} |`);
  }
  lines.push('');
  lines.push('## Violations');
  if (!payload.violations.length) {
    lines.push('- none');
  } else {
    lines.push('| kind | rule | pattern | path | detail |');
    lines.push('| --- | --- | --- | --- | --- |');
    for (const row of payload.violations) {
      lines.push(`| ${row.kind} | ${row.rule_id ?? ''} | ${row.pattern_id ?? ''} | ${row.path ?? ''} | ${row.detail} |`);
    }
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main(): void {
  const args = readArgs(process.argv.slice(2));
  const policy = readJson<GrowthPolicy>(args.growthPolicyPath);
  const map = readJson<OwnershipMap>(args.ownershipMapPath);
  const files = shellFiles(policy);
  const mapViolations = validateOwnershipMap(map, policy, args);
  const inventoryResult = inventory(policy, map, files);
  const violations = [...mapViolations, ...inventoryResult.violations];
  const ok = violations.length === 0;
  const payload = {
    ok,
    type: 'shell_alpine_ownership_inventory',
    generated_at: new Date().toISOString(),
    strict: args.strict,
    growth_policy_path: args.growthPolicyPath,
    ownership_map_path: args.ownershipMapPath,
    summary: {
      pass: ok,
      scanned_files: files.length,
      ownership_rules: map.rules?.length ?? 0,
      inventory_rows: inventoryResult.rows.length,
      total_alpine_hits: inventoryResult.rows.reduce((sum, row) => sum + row.count, 0),
      migration_class_counts: classSummary(inventoryResult.rows),
      violations: violations.length,
    },
    inventory: inventoryResult.rows,
    violations,
    artifact_paths: [args.outJson, args.outMarkdown],
  };
  writeTextArtifact(args.outMarkdown, markdown(payload));
  process.exitCode = emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

main();
