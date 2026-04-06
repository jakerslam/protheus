#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative setup state + policy lane)
// Thin TypeScript UX wrapper only.

const path = require('path');
const { spawnSync } = require('child_process');
const readline = require('readline');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const OPS_WRAPPER = path.join(
  ROOT,
  'client',
  'runtime',
  'systems',
  'ops',
  'run_protheus_ops.ts'
);
const TS_ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const DEFAULT_STATE_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'protheus_setup_wizard',
  'latest.json'
);

function cleanText(raw, maxLen = 120) {
  return String(raw || '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function asBool(raw, fallback = false) {
  const value = String(raw || '').trim().toLowerCase();
  if (!value) return fallback;
  if (['1', 'true', 'yes', 'on', 'y'].includes(value)) return true;
  if (['0', 'false', 'no', 'off', 'n'].includes(value)) return false;
  return fallback;
}

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => String(line || '').trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch (_) {}
  }
  return null;
}

function invokeSetupWizardKernel(payload) {
  const serialized = JSON.stringify(payload && typeof payload === 'object' ? payload : {});
  const run = spawnSync(
    process.execPath,
    [
      TS_ENTRYPOINT,
      OPS_WRAPPER,
      'state-kernel',
      'setup-wizard',
      `--payload-base64=${encodeBase64(serialized)}`
    ],
    {
      cwd: ROOT,
      env: { ...process.env },
      encoding: 'utf8'
    }
  );
  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  const receipt = parseLastJson(run.stdout);
  const payloadOut = receipt && typeof receipt === 'object'
    && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (status !== 0 || !payloadOut || typeof payloadOut !== 'object') {
    return {
      ok: false,
      type: 'protheus_setup_wizard',
      error: cleanText(
        run && run.stderr ? String(run.stderr) : 'setup_wizard_kernel_bridge_failed',
        240
      )
    };
  }
  return payloadOut;
}

function parseArgs(argv = process.argv.slice(2)) {
  const out = {
    command: 'run',
    json: false,
    force: false,
    skip: false,
    defaults: false,
    yes: false,
    interaction: '',
    notifications: ''
  };
  const tokens = Array.isArray(argv) ? argv.slice() : [];
  if (tokens.length > 0 && !String(tokens[0] || '').startsWith('--')) {
    out.command = cleanText(tokens.shift(), 40).toLowerCase() || 'run';
  }
  for (let i = 0; i < tokens.length; i += 1) {
    const token = String(tokens[i] || '').trim();
    if (!token) continue;
    if (token === '--json' || token === '--json=1') {
      out.json = true;
      continue;
    }
    if (token === '--force' || token === '--force=1') {
      out.force = true;
      continue;
    }
    if (token === '--skip' || token === '--skip=1') {
      out.skip = true;
      continue;
    }
    if (token === '--defaults' || token === '--defaults=1') {
      out.defaults = true;
      continue;
    }
    if (token === '--yes' || token === '-y' || token === '--yes=1') {
      out.yes = true;
      continue;
    }
    if (token.startsWith('--interaction=')) {
      out.interaction = cleanText(token.slice('--interaction='.length), 40);
      continue;
    }
    if (token === '--interaction' && tokens[i + 1]) {
      out.interaction = cleanText(tokens[i + 1], 40);
      i += 1;
      continue;
    }
    if (token.startsWith('--notifications=')) {
      out.notifications = cleanText(token.slice('--notifications='.length), 40);
      continue;
    }
    if (token === '--notifications' && tokens[i + 1]) {
      out.notifications = cleanText(tokens[i + 1], 40);
      i += 1;
      continue;
    }
  }
  return out;
}

function pickInteraction(raw) {
  const normalized = cleanText(raw, 40).toLowerCase();
  if (['silent', 'quiet'].includes(normalized)) return 'silent';
  return 'proactive';
}

function pickNotifications(raw) {
  const normalized = cleanText(raw, 40).toLowerCase();
  if (['all', 'critical', 'none'].includes(normalized)) return normalized;
  return 'critical';
}

function emit(jsonMode, payload, line) {
  if (jsonMode) {
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return;
  }
  if (line) {
    process.stdout.write(`${line}\n`);
  }
}

function ask(prompt, fallback) {
  return new Promise((resolve) => {
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout
    });
    rl.question(prompt, (answer) => {
      rl.close();
      const value = cleanText(answer, 80);
      resolve(value || fallback);
    });
  });
}

async function runWizard(opts) {
  const nonInteractive = opts.yes || opts.defaults || !process.stdin.isTTY || !process.stdout.isTTY;
  let covenantAck = true;
  let interaction = pickInteraction(opts.interaction || 'proactive');
  let notifications = pickNotifications(opts.notifications || 'critical');

  if (opts.skip) {
    covenantAck = false;
    interaction = 'silent';
    notifications = 'none';
  } else if (!nonInteractive) {
    const covenantInput = await ask(
      'Confirm covenant-first defaults? [Y/n]: ',
      'y'
    );
    covenantAck = asBool(covenantInput, true);
    const interactionInput = await ask(
      'Interaction style [proactive/silent] (default proactive): ',
      interaction
    );
    interaction = pickInteraction(interactionInput);
    const notificationsInput = await ask(
      'Notifications [all/critical/none] (default critical): ',
      notifications
    );
    notifications = pickNotifications(notificationsInput);
  }

  const payload = invokeSetupWizardKernel({
    command: 'run',
    force: !!opts.force,
    skip: !!opts.skip,
    defaults: !!opts.defaults,
    yes: !!opts.yes,
    interaction,
    notifications,
    covenant_acknowledged: covenantAck
  });
  if (!payload || payload.ok !== true) {
    const error = cleanText(payload && payload.error ? payload.error : 'setup_wizard_kernel_failed', 240);
    emit(opts.json, payload || { ok: false, type: 'protheus_setup_wizard', error }, '');
    if (!opts.json) process.stderr.write(`${error}\n`);
    return 1;
  }
  if (payload.skipped === true && String(payload.reason || '') === 'already_completed') {
    emit(opts.json, payload, '[infring setup] already completed');
    return 0;
  }

  emit(
    opts.json,
    payload,
    `[infring setup] saved profile (interaction=${interaction}, notifications=${notifications})`
  );
  return 0;
}

function statusWizard(opts) {
  const payload = invokeSetupWizardKernel({ command: 'status' });
  const fallbackState = {
    type: 'protheus_setup_wizard_state',
    completed: false,
    version: 1
  };
  const state = payload && payload.state && typeof payload.state === 'object'
    ? payload.state
    : fallbackState;
  if (opts.json) {
    process.stdout.write(`${JSON.stringify(payload || {
      ok: false,
      type: 'protheus_setup_wizard',
      command: 'status',
      state_path: DEFAULT_STATE_PATH,
      state
    })}\n`);
    return 0;
  }
  if (state && state.completed === true) {
    process.stdout.write('[infring setup] completed\n');
  } else {
    process.stdout.write('[infring setup] pending\n');
  }
  return 0;
}

function resetWizard(opts) {
  const payload = invokeSetupWizardKernel({ command: 'reset' });
  const removed = !!(payload && payload.removed);
  emit(opts.json, payload || { ok: false, type: 'protheus_setup_wizard', command: 'reset' }, removed ? '[infring setup] reset complete' : '[infring setup] nothing to reset');
  return 0;
}

async function main(argv = process.argv.slice(2)) {
  const opts = parseArgs(argv);
  if (opts.command === 'help' || opts.command === '--help' || opts.command === '-h') {
    const usage = invokeSetupWizardKernel({ command: 'help' });
    const lines = usage && Array.isArray(usage.usage)
      ? usage.usage
      : [
          'protheus setup [run|status|reset] [--json]',
          'protheus setup run [--force] [--yes] [--defaults] [--interaction=<proactive|silent>] [--notifications=<all|critical|none>]',
          'protheus setup run --skip',
          'protheus setup status',
          'protheus setup reset'
        ];
    emit(opts.json, usage, lines.join('\n'));
    return 0;
  }

  if (opts.command === 'status') return statusWizard(opts);
  if (opts.command === 'reset') return resetWizard(opts);
  if (opts.command === 'complete') {
    opts.yes = true;
    opts.defaults = true;
    return runWizard(opts);
  }
  return runWizard(opts);
}

if (require.main === module) {
  Promise.resolve(main(process.argv.slice(2)))
    .then((code) => process.exit(Number.isFinite(code) ? code : 0))
    .catch((err) => {
      process.stderr.write(
        `${JSON.stringify({
          ok: false,
          type: 'protheus_setup_wizard',
          error: cleanText(err && err.message ? err.message : err, 220)
        })}\n`
      );
      process.exit(1);
    });
}

module.exports = {
  parseArgs,
  runWizard,
  statusWizard,
  resetWizard
};
