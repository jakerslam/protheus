#!/usr/bin/env node
'use strict';

const path = require('node:path');
const { ROOT, parseArgs, parseJson, invokeOrchestration } = require('./core_bridge.ts');
const DEFAULT_SCRATCHPAD_DIR = path.join(ROOT, 'local', 'workspace', 'scratchpad');
const TASK_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{2,127}$/;
const SCHEMA_VERSION = 'scratchpad/v1';

function taskIdFrom(parsed, fallback = '') {
  return String(
    parsed.flags['task-id']
      || parsed.flags.task_id
      || parsed.positional[1]
      || fallback
  ).trim();
}

function withFilePathAlias(out) {
  if (!out || typeof out !== 'object') return out;
  if (out.file_path && !out.filePath) {
    return { ...out, filePath: out.file_path };
  }
  return out;
}

function scratchpadPath(taskId, options = {}) {
  const out = invokeOrchestration('scratchpad.path', {
    task_id: String(taskId || '').trim(),
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  if (out && out.ok && out.file_path) return String(out.file_path);
  throw new Error(String(out && out.reason_code ? out.reason_code : 'orchestration_bridge_error'));
}

function loadScratchpad(taskId, options = {}) {
  return withFilePathAlias(invokeOrchestration('scratchpad.status', {
    task_id: String(taskId || '').trim(),
    root_dir: options.rootDir || options.root_dir || undefined,
  }));
}

function writeScratchpad(taskId, patch = {}, options = {}) {
  return withFilePathAlias(invokeOrchestration('scratchpad.write', {
    task_id: String(taskId || '').trim(),
    patch: patch && typeof patch === 'object' ? patch : {},
    root_dir: options.rootDir || options.root_dir || undefined,
  }));
}

function appendFinding(taskId, finding, options = {}) {
  return withFilePathAlias(invokeOrchestration('scratchpad.append_finding', {
    task_id: String(taskId || '').trim(),
    finding: finding && typeof finding === 'object' ? finding : {},
    root_dir: options.rootDir || options.root_dir || undefined,
  }));
}

function appendCheckpoint(taskId, checkpoint, options = {}) {
  return withFilePathAlias(invokeOrchestration('scratchpad.append_checkpoint', {
    task_id: String(taskId || '').trim(),
    checkpoint: checkpoint && typeof checkpoint === 'object' ? checkpoint : {},
    root_dir: options.rootDir || options.root_dir || undefined,
  }));
}

function cleanupScratchpad(taskId, options = {}) {
  return withFilePathAlias(invokeOrchestration('scratchpad.cleanup', {
    task_id: String(taskId || '').trim(),
    root_dir: options.rootDir || options.root_dir || undefined,
  }));
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'status').trim().toLowerCase();
  const taskId = taskIdFrom(parsed);

  if (command === 'status' || command === 'read') {
    return loadScratchpad(taskId);
  }

  if (command === 'write') {
    const patchPayload = parseJson(parsed.flags['payload-json'] || parsed.flags.payload_json, {}, 'invalid_payload_json');
    if (!patchPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_scratchpad_write',
        reason_code: patchPayload.reason_code,
      };
    }
    return writeScratchpad(taskId, patchPayload.value);
  }

  if (command === 'append-finding') {
    const findingPayload = parseJson(parsed.flags['finding-json'] || parsed.flags.finding_json, {}, 'invalid_finding_json');
    if (!findingPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_scratchpad_append_finding',
        reason_code: findingPayload.reason_code,
      };
    }
    return appendFinding(taskId, findingPayload.value);
  }

  if (command === 'checkpoint') {
    const checkpointPayload = parseJson(
      parsed.flags['checkpoint-json'] || parsed.flags.checkpoint_json,
      {},
      'invalid_checkpoint_json',
    );
    if (!checkpointPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_scratchpad_append_checkpoint',
        reason_code: checkpointPayload.reason_code,
      };
    }
    return appendCheckpoint(taskId, checkpointPayload.value);
  }

  if (command === 'cleanup') {
    return cleanupScratchpad(taskId);
  }

  return {
    ok: false,
    type: 'orchestration_scratchpad_command',
    reason_code: `unsupported_command:${command}`,
    commands: ['status', 'read', 'write', 'append-finding', 'checkpoint', 'cleanup'],
  };
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out.ok ? 0 : 1);
}

module.exports = {
  ROOT,
  DEFAULT_SCRATCHPAD_DIR,
  SCHEMA_VERSION,
  TASK_ID_PATTERN,
  parseArgs,
  taskIdFrom,
  scratchpadPath,
  loadScratchpad,
  writeScratchpad,
  appendFinding,
  appendCheckpoint,
  cleanupScratchpad,
  run,
};
