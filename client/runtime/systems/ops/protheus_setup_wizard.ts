#!/usr/bin/env node
'use strict';

// Layer ownership: client/runtime/systems/ops (authoritative setup compatibility lane)

const fs = require('fs');
const path = require('path');
const readline = require('readline');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const STATE_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'protheus_setup_wizard',
  'latest.json'
);

function nowIso() {
  return new Date().toISOString();
}

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

function readState() {
  try {
    const raw = fs.readFileSync(STATE_PATH, 'utf8');
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === 'object') {
      return parsed;
    }
  } catch (_) {}
  return null;
}

function writeState(payload) {
  fs.mkdirSync(path.dirname(STATE_PATH), { recursive: true });
  fs.writeFileSync(STATE_PATH, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
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
  const existing = readState();
  if (existing && existing.completed === true && !opts.force) {
    const payload = {
      ok: true,
      type: 'protheus_setup_wizard',
      command: 'run',
      skipped: true,
      reason: 'already_completed',
      state_path: STATE_PATH,
      state: existing
    };
    emit(opts.json, payload, '[infring setup] already completed');
    return 0;
  }

  const nonInteractive = opts.yes || opts.defaults || !process.stdin.isTTY || !process.stdout.isTTY;
  let covenantAck = true;
  let interaction = pickInteraction(opts.interaction || 'proactive');
  let notifications = pickNotifications(opts.notifications || 'critical');
  let completionMode = nonInteractive ? 'defaults' : 'interactive';

  if (opts.skip) {
    completionMode = 'skipped';
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

  const payload = {
    type: 'protheus_setup_wizard_state',
    completed: true,
    completed_at: nowIso(),
    completion_mode: completionMode,
    covenant_acknowledged: covenantAck,
    interaction_style: interaction,
    notifications,
    profile: {
      interaction_style: interaction,
      notifications
    },
    version: 1
  };
  writeState(payload);

  emit(
    opts.json,
    {
      ok: true,
      type: 'protheus_setup_wizard',
      command: 'run',
      state_path: STATE_PATH,
      state: payload
    },
    `[infring setup] saved profile (interaction=${interaction}, notifications=${notifications})`
  );
  return 0;
}

function statusWizard(opts) {
  const existing = readState();
  const payload = {
    ok: true,
    type: 'protheus_setup_wizard',
    command: 'status',
    state_path: STATE_PATH,
    state: existing || {
      type: 'protheus_setup_wizard_state',
      completed: false,
      version: 1
    }
  };
  if (opts.json) {
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return 0;
  }
  if (existing && existing.completed === true) {
    process.stdout.write('[infring setup] completed\n');
  } else {
    process.stdout.write('[infring setup] pending\n');
  }
  return 0;
}

function resetWizard(opts) {
  let removed = false;
  try {
    fs.unlinkSync(STATE_PATH);
    removed = true;
  } catch (_) {}
  const payload = {
    ok: true,
    type: 'protheus_setup_wizard',
    command: 'reset',
    state_path: STATE_PATH,
    removed
  };
  emit(opts.json, payload, removed ? '[infring setup] reset complete' : '[infring setup] nothing to reset');
  return 0;
}

async function main(argv = process.argv.slice(2)) {
  const opts = parseArgs(argv);
  if (opts.command === 'help' || opts.command === '--help' || opts.command === '-h') {
    const usage = {
      ok: true,
      type: 'protheus_setup_wizard_help',
      usage: [
        'protheus setup [run|status|reset] [--json]',
        'protheus setup run [--force] [--yes] [--defaults] [--interaction=<proactive|silent>] [--notifications=<all|critical|none>]',
        'protheus setup run --skip',
        'protheus setup status',
        'protheus setup reset'
      ]
    };
    emit(opts.json, usage, usage.usage.join('\n'));
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
  resetWizard,
  STATE_PATH
};
