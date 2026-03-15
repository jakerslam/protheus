#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { validateFinding, normalizeFinding } = require('./schema_runtime.ts');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const DEFAULT_SCRATCHPAD_DIR = path.join(ROOT, 'local', 'workspace', 'scratchpad');
const TASK_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{2,127}$/;
const SCHEMA_VERSION = 'scratchpad/v1';

function parseArgs(argv = []) {
  const positional = [];
  const flags = {};
  for (const raw of Array.isArray(argv) ? argv : []) {
    const token = String(raw || '').trim();
    if (!token) continue;
    if (token.startsWith('--')) {
      const body = token.slice(2);
      const eq = body.indexOf('=');
      if (eq >= 0) flags[body.slice(0, eq)] = body.slice(eq + 1);
      else flags[body] = '1';
      continue;
    }
    positional.push(token);
  }
  return { positional, flags };
}

function taskIdFrom(parsed, fallback = '') {
  return String(
    parsed.flags['task-id']
      || parsed.flags.task_id
      || parsed.positional[1]
      || fallback
  ).trim();
}

function assertTaskId(taskId) {
  if (!TASK_ID_PATTERN.test(String(taskId || ''))) {
    throw new Error(`invalid_task_id:${taskId || '<empty>'}`);
  }
}

function scratchpadPath(taskId, options = {}) {
  assertTaskId(taskId);
  const rootDir = options.rootDir || DEFAULT_SCRATCHPAD_DIR;
  return path.join(rootDir, `${taskId}.json`);
}

function emptyScratchpad(taskId) {
  const now = new Date().toISOString();
  return {
    schema_version: SCHEMA_VERSION,
    task_id: taskId,
    created_at: now,
    updated_at: now,
    progress: {
      processed: 0,
      total: 0
    },
    findings: [],
    checkpoints: []
  };
}

function loadScratchpad(taskId, options = {}) {
  const filePath = scratchpadPath(taskId, options);
  try {
    const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    return {
      scratchpad: parsed,
      filePath,
      exists: true
    };
  } catch {
    const fresh = emptyScratchpad(taskId);
    return {
      scratchpad: fresh,
      filePath,
      exists: false
    };
  }
}

function writeScratchpad(taskId, patch = {}, options = {}) {
  const loaded = loadScratchpad(taskId, options);
  const filePath = loaded.filePath;
  const base = loaded.scratchpad;
  const now = new Date().toISOString();

  const next = Object.assign({}, base, patch, {
    schema_version: SCHEMA_VERSION,
    task_id: taskId,
    updated_at: now,
    created_at: base.created_at || now
  });

  if (!next.progress || typeof next.progress !== 'object') {
    next.progress = { processed: 0, total: 0 };
  }
  next.progress.processed = Number.isFinite(Number(next.progress.processed))
    ? Number(next.progress.processed)
    : 0;
  next.progress.total = Number.isFinite(Number(next.progress.total))
    ? Number(next.progress.total)
    : 0;

  if (!Array.isArray(next.findings)) next.findings = [];
  if (!Array.isArray(next.checkpoints)) next.checkpoints = [];

  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(next, null, 2)}\n`);
  return {
    ok: true,
    type: 'orchestration_scratchpad_write',
    task_id: taskId,
    file_path: filePath,
    scratchpad: next
  };
}

function appendFinding(taskId, finding, options = {}) {
  const normalized = normalizeFinding(finding);
  const validation = validateFinding(normalized);
  if (!validation.ok) {
    return {
      ok: false,
      type: 'orchestration_scratchpad_append_finding',
      reason_code: validation.reason_code,
      task_id: taskId
    };
  }

  const loaded = loadScratchpad(taskId, options);
  const findings = Array.isArray(loaded.scratchpad.findings)
    ? loaded.scratchpad.findings.slice()
    : [];
  findings.push(normalized);
  const out = writeScratchpad(taskId, { findings }, options);
  return Object.assign({}, out, {
    type: 'orchestration_scratchpad_append_finding',
    finding_count: findings.length
  });
}

function appendCheckpoint(taskId, checkpoint, options = {}) {
  const loaded = loadScratchpad(taskId, options);
  const rows = Array.isArray(loaded.scratchpad.checkpoints)
    ? loaded.scratchpad.checkpoints.slice()
    : [];
  rows.push(
    Object.assign(
      {
        created_at: new Date().toISOString()
      },
      checkpoint && typeof checkpoint === 'object' ? checkpoint : {}
    )
  );
  const out = writeScratchpad(taskId, { checkpoints: rows }, options);
  return Object.assign({}, out, {
    type: 'orchestration_scratchpad_append_checkpoint',
    checkpoint_count: rows.length
  });
}

function cleanupScratchpad(taskId, options = {}) {
  const filePath = scratchpadPath(taskId, options);
  try {
    fs.unlinkSync(filePath);
  } catch {}
  return {
    ok: true,
    type: 'orchestration_scratchpad_cleanup',
    task_id: taskId,
    file_path: filePath,
    removed: !fs.existsSync(filePath)
  };
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'status').trim().toLowerCase();
  const taskId = taskIdFrom(parsed);

  try {
    if (command === 'status' || command === 'read') {
      assertTaskId(taskId);
      const loaded = loadScratchpad(taskId);
      return {
        ok: true,
        type: 'orchestration_scratchpad_status',
        task_id: taskId,
        file_path: loaded.filePath,
        exists: loaded.exists,
        scratchpad: loaded.scratchpad
      };
    }

    if (command === 'write') {
      assertTaskId(taskId);
      const payload = parsed.flags['payload-json'] || parsed.flags.payload_json || '{}';
      let patch = {};
      try {
        patch = JSON.parse(String(payload));
      } catch {
        return {
          ok: false,
          type: 'orchestration_scratchpad_write',
          reason_code: 'invalid_payload_json'
        };
      }
      return writeScratchpad(taskId, patch);
    }

    if (command === 'append-finding') {
      assertTaskId(taskId);
      const payload = parsed.flags['finding-json'] || parsed.flags.finding_json || '{}';
      let finding = {};
      try {
        finding = JSON.parse(String(payload));
      } catch {
        return {
          ok: false,
          type: 'orchestration_scratchpad_append_finding',
          reason_code: 'invalid_finding_json'
        };
      }
      return appendFinding(taskId, finding);
    }

    if (command === 'checkpoint') {
      assertTaskId(taskId);
      const payload = parsed.flags['checkpoint-json'] || parsed.flags.checkpoint_json || '{}';
      let checkpoint = {};
      try {
        checkpoint = JSON.parse(String(payload));
      } catch {
        return {
          ok: false,
          type: 'orchestration_scratchpad_append_checkpoint',
          reason_code: 'invalid_checkpoint_json'
        };
      }
      return appendCheckpoint(taskId, checkpoint);
    }

    if (command === 'cleanup') {
      assertTaskId(taskId);
      return cleanupScratchpad(taskId);
    }

    return {
      ok: false,
      type: 'orchestration_scratchpad_command',
      reason_code: `unsupported_command:${command}`,
      commands: ['status', 'read', 'write', 'append-finding', 'checkpoint', 'cleanup']
    };
  } catch (error) {
    return {
      ok: false,
      type: 'orchestration_scratchpad_command',
      reason_code: String(error && error.message ? error.message : error)
    };
  }
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
  run
};
