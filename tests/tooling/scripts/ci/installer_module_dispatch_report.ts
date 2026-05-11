#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (installer module dispatch report)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/installer_module_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));

function read(rel) {
  try { return fs.readFileSync(path.join(root, rel), 'utf8'); } catch { return ''; }
}

const rows = [];
for (const target of policy.dispatch_targets || []) {
  const installerSource = read(target.installer);
  const moduleSource = read(target.module);
  const symbol = String(target.module_symbol || '');
  const referencesModulePath = installerSource.includes(target.module) || installerSource.includes(path.basename(target.module));
  const referencesSymbol = symbol && installerSource.includes(symbol);
  const moduleDefinesSymbol = symbol && moduleSource.includes(symbol);
  const hasWindowsWrapperMirror = symbol === 'Write-InfringWindowsWrappers' && [
    'Write-CmdWrapper',
    'Write-DaemonCmdWrapper',
    'Write-BootstrapGatewayCmdWrapper',
    'Write-PowerShellShim',
    'infring.cmd',
    'infring.ps1',
  ].every((token) => installerSource.includes(token));
  const hasMirrorImplementation = symbol
    ? hasWindowsWrapperMirror || installerSource.includes(symbol.replace(/^Write-Infring/, 'Write-')) || installerSource.includes(symbol.replace(/^infring_install_/, 'emit_install_'))
    : false;
  const status = referencesModulePath || referencesSymbol
    ? 'referenced'
    : moduleDefinesSymbol && hasMirrorImplementation
      ? 'mirror_only'
      : moduleDefinesSymbol
        ? 'unwired'
        : 'module_symbol_missing';
  rows.push({
    installer: target.installer,
    module: target.module,
    module_symbol: symbol,
    status,
    references_module_path: referencesModulePath,
    references_symbol: referencesSymbol,
    module_defines_symbol: moduleDefinesSymbol,
    has_mirror_implementation: hasMirrorImplementation,
    next_action: status === 'referenced'
      ? null
      : status === 'mirror_only'
        ? 'Gradually replace mirrored installer implementation with module dispatch after platform replay coverage is green.'
        : 'Wire installer to the named module or explain why this module is no longer needed.',
  });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'installer_module_dispatch_report',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  ok: true,
  referenced_count: rows.filter((row) => row.status === 'referenced').length,
  mirror_only_count: rows.filter((row) => row.status === 'mirror_only').length,
  unwired_count: rows.filter((row) => row.status === 'unwired' || row.status === 'module_symbol_missing').length,
  rows,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/installer_module_dispatch_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
