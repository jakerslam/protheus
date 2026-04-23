#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

const ROOT = process.cwd();
const INSTALL_SCRIPT_PATH = 'install.ps1';
const DEFAULT_OUT_JSON = 'core/local/artifacts/windows_installer_contract_guard_current.json';
const DEFAULT_OUT_MARKDOWN = 'local/workspace/reports/WINDOWS_INSTALLER_CONTRACT_GUARD_CURRENT.md';

type CheckRow = {
  id: string;
  ok: boolean;
  detail: string;
};

type Args = {
  strict: boolean;
  outJson: string;
  outMarkdown: string;
};

function parseArgs(argv: string[]): Args {
  const common = parseStrictOutArgs(argv, { strict: false, out: DEFAULT_OUT_JSON });
  return {
    strict: common.strict,
    outJson: cleanText(readFlag(argv, 'out-json') || common.out || DEFAULT_OUT_JSON, 600),
    outMarkdown: cleanText(readFlag(argv, 'out-markdown') || DEFAULT_OUT_MARKDOWN, 600),
  };
}

function countOccurrences(source: string, pattern: RegExp): number {
  const matches = source.match(pattern);
  return Array.isArray(matches) ? matches.length : 0;
}

function toMarkdown(payload: any): string {
  const lines: string[] = [];
  lines.push('# WINDOWS INSTALLER CONTRACT GUARD');
  lines.push('');
  lines.push(`- generated_at: ${payload.generated_at}`);
  lines.push(`- revision: ${payload.revision}`);
  lines.push(`- ok: ${payload.ok}`);
  lines.push(`- strict: ${payload.strict}`);
  lines.push('');
  lines.push('## Checks');
  for (const row of payload.checks || []) {
    lines.push(`- [${row.ok ? 'x' : ' '}] \`${row.id}\` — ${row.detail}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function run(argv: string[]): number {
  const args = parseArgs(argv);
  const source = fs.readFileSync(path.resolve(ROOT, INSTALL_SCRIPT_PATH), 'utf8');

  const checks: CheckRow[] = [];
  const asciiWrapperWriteCount = countOccurrences(
    source,
    /Set-Content\s+-Path \$Path -Value \$content -Encoding ASCII/g,
  );
  const parserUnsafeJoinPathRegexCount = countOccurrences(
    source,
    /Join-Path\\\\s\+\\\\/g,
  );

  checks.push({
    id: 'windows_install_script_no_merge_conflict_markers',
    ok: !source.includes('<<<<<<<') && !source.includes('=======') && !source.includes('>>>>>>>'),
    detail: 'install.ps1 must not contain unresolved merge markers',
  });

  checks.push({
    id: 'windows_install_script_single_repair_function_definition',
    ok: countOccurrences(source, /function\s+Invoke-RepairInstallDir\s*\{/g) === 1,
    detail: 'Invoke-RepairInstallDir must be declared exactly once',
  });

  checks.push({
    id: 'windows_install_script_single_repair_target_table',
    ok: countOccurrences(source, /\$targets\s*=\s*@\(/g) === 1,
    detail: 'repair target list must have one canonical declaration to prevent parser drift',
  });

  checks.push({
    id: 'windows_install_script_no_double_quoted_psscriptroot_regex',
    ok: !source.includes('-match "Join-Path\\\\s+\\\\$PSScriptRoot"'),
    detail: 'regex checks for $PSScriptRoot must avoid double-quoted interpolation hazards',
  });

  checks.push({
    id: 'windows_install_script_no_parser_unsafe_join_path_regex_sequences',
    ok: parserUnsafeJoinPathRegexCount === 0,
    detail: `install.ps1 must not contain parser-unsafe escaped Join-Path regex sequences (count=${parserUnsafeJoinPathRegexCount})`,
  });

  checks.push({
    id: 'windows_install_script_cmd_wrappers_ascii_encoded',
    ok: asciiWrapperWriteCount >= 3,
    detail: `cmd wrapper writers must emit ASCII wrappers for Windows cmd compatibility (ascii_writes=${asciiWrapperWriteCount})`,
  });

  checks.push({
    id: 'windows_install_script_powershell_shims_utf8_encoded',
    ok: source.includes('Set-Content -Path $Path -Value $content -Encoding UTF8'),
    detail: 'PowerShell shim writer must emit UTF8 content deterministically',
  });

  checks.push({
    id: 'windows_install_script_bootstrap_floor_invoked_during_repair_and_post_write',
    ok: countOccurrences(source, /Ensure-RepairBootstrapWrapperFloor\s+-InstallDir\s+\$InstallDir/g) >= 2,
    detail: 'wrapper floor must run during repair and after wrapper generation',
  });

  checks.push({
    id: 'windows_install_script_bootstrap_help_copy_present',
    ok:
      source.includes('[infring bootstrap] runtime binaries are not installed on this machine yet.')
      && source.includes('[infring bootstrap] run: install.ps1 -Repair -Full'),
    detail: 'installer wrappers must include deterministic bootstrap recovery guidance',
  });

  checks.push({
    id: 'windows_install_script_bootstrap_action_cmd_fallback_present',
    ok: source.includes('_BOOTSTRAP_ACTION') && source.includes('goto :bootstrap'),
    detail: 'failure bootstrap cmd wrapper must include deterministic action dispatch fallback',
  });

  checks.push({
    id: 'windows_install_script_repair_wrapper_map_covers_three_entrypoints',
    ok:
      source.includes('@{ cmd = "infring.cmd"; ps1 = "infring.ps1" }')
      && source.includes('@{ cmd = "infringctl.cmd"; ps1 = "infringctl.ps1" }')
      && source.includes('@{ cmd = "infringd.cmd"; ps1 = "infringd.ps1" }'),
    detail: 'repair/failure wrapper maps must preserve all three canonical entrypoint wrapper pairs',
  });

  checks.push({
    id: 'windows_install_script_failure_bootstrap_function_present',
    ok: source.includes('function Ensure-InstallFailureBootstrapWrappers'),
    detail: 'failure path must still emit bootstrap wrappers for deterministic recovery',
  });

  checks.push({
    id: 'windows_install_script_repair_detects_legacy_throw_wrappers',
    ok:
      source.includes("missing command wrapper")
      && source.includes('Test-RepairArtifactBroken')
      && source.includes('throw '),
    detail: 'repair logic must detect and replace legacy throw-based wrapper templates',
  });

  checks.push({
    id: 'windows_install_script_repair_cmd_write_uses_literalpath_force',
    ok: source.includes('Set-Content -LiteralPath $cmdPath -Value $cmdContent -Encoding ASCII -Force'),
    detail: 'repair cmd wrapper writes must use LiteralPath+Force for deterministic rewrite behavior',
  });

  checks.push({
    id: 'windows_install_script_repair_ps1_write_uses_literalpath_force',
    ok: source.includes('Set-Content -LiteralPath $ps1Path -Value $psContent -Encoding UTF8 -Force'),
    detail: 'repair PowerShell wrapper writes must use LiteralPath+Force for deterministic rewrite behavior',
  });

  checks.push({
    id: 'windows_install_script_repair_cmd_detects_placeholder_tokens',
    ok: source.includes('if ($content.Contains("__PS1__") -or $content.Contains("__TARGET__"))'),
    detail: 'repair cmd wrapper detector must fail closed on unresolved template placeholders',
  });

  checks.push({
    id: 'windows_install_script_repair_ps1_detects_placeholder_tokens',
    ok: source.includes('if ($content.Contains("__TARGET__") -or $content.Contains("__PS1__"))'),
    detail: 'repair ps1 wrapper detector must fail closed on unresolved template placeholders',
  });

  const ok = checks.every((row) => row.ok);
  const payload = {
    ok,
    strict: args.strict,
    type: 'windows_installer_contract_guard',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    inputs: {
      script_path: INSTALL_SCRIPT_PATH,
      out_json: args.outJson,
      out_markdown: args.outMarkdown,
    },
    summary: {
      check_count: checks.length,
      failed_check_count: checks.filter((row) => !row.ok).length,
    },
    checks,
  };

  writeTextArtifact(args.outMarkdown, toMarkdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outJson,
    strict: args.strict,
    ok,
  });
}

process.exit(run(process.argv.slice(2)));
