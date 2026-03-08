#!/usr/bin/env node
'use strict';
export {};

function nowIso() {
  return new Date().toISOString();
}

function usage() {
  process.stdout.write('Usage: improvement_orchestrator.js propose|start-next|evaluate-open|status [options]\n');
}

function emit(payload: Record<string, unknown>, exitCode = 0) {
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(exitCode);
}

function failClosed(command: string) {
  emit({
    ok: false,
    type: 'improvement_orchestrator',
    ts: nowIso(),
    command,
    fail_closed: true,
    reason: 'improvement_orchestrator_ts_stub_unimplemented'
  }, 1);
}

function run(command: string) {
  if (command === 'status') {
    emit({
      ok: true,
      type: 'improvement_orchestrator_status',
      ts: nowIso(),
      mode: 'stub',
      available: false,
      reason: 'improvement_orchestrator_ts_stub_unimplemented'
    }, 0);
    return;
  }
  if (command === 'propose' || command === 'start-next' || command === 'evaluate-open') {
    failClosed(command);
    return;
  }
  usage();
  process.exit(2);
}

function main() {
  const cmd = String(process.argv[2] || '').trim().toLowerCase();
  if (!cmd || cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    process.exit(0);
    return;
  }
  run(cmd);
}

if (require.main === module) {
  main();
}

module.exports = {
  run
};
