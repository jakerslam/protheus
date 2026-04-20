#!/usr/bin/env node
'use strict';

const path = require('path');
const readline = require('readline');
const { invokeKernelPayload } = require('./protheus_kernel_bridge.ts');

const ROOT = path.resolve(__dirname, '..', '..');
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

function invokeSetupWizardKernel(payload) {
  return invokeKernelPayload(
    'state-kernel',
    'setup-wizard',
    payload,
    {
      throwOnError: false,
      fallbackError: 'setup_wizard_kernel_bridge_failed',
    }
  );
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

function detectRuntimeMode() {
  const explicit = cleanText(
    process.env.INFRING_INSTALL_MODE ||
      process.env.INFRING_RUNTIME_MODE ||
      process.env.PROTHEUS_RUNTIME_MODE,
    40
  ).toLowerCase();
  if (['minimal', 'full', 'pure', 'tiny-max'].includes(explicit)) return explicit;
  if (asBool(process.env.INFRING_TINY_MAX_MODE, false)) return 'tiny-max';
  if (asBool(process.env.INFRING_PURE_MODE, false)) return 'pure';
  return 'full';
}

function detectWorkspaceRoot() {
  return cleanText(
    process.env.INFRING_WORKSPACE_ROOT ||
      process.env.PROTHEUS_WORKSPACE_ROOT ||
      process.cwd(),
    400
  );
}

function buildOnboardingReceipt(status, nextAction = 'none') {
  return {
    mode: detectRuntimeMode(),
    workspace_root: detectWorkspaceRoot(),
    status: cleanText(status, 80) || 'unknown',
    next_action: cleanText(nextAction, 160) || 'none'
  };
}

function attachOnboardingReceipt(payload, status, nextAction = 'none') {
  const base = payload && typeof payload === 'object' ? payload : {};
  base.onboarding_receipt = buildOnboardingReceipt(status, nextAction);
  return base;
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

function buildSetupFailureRecovery(errorCode = 'setup_wizard_kernel_failed') {
  const normalizedError = cleanText(errorCode, 120) || 'setup_wizard_kernel_failed';
  return {
    route: 'setup_failure_recovery_v1',
    error_code: normalizedError,
    retry_command: 'infring setup --yes --defaults',
    retry_expected_output: 'saved profile confirmation with onboarding receipt',
    status_command: 'infring setup status --json',
    status_expected_output: 'onboarding_receipt.status is completed or incomplete',
    diagnostics_command: 'infring doctor --json',
    diagnostics_expected_output: 'deterministic install/runtime diagnostics contract',
    escalation_command: 'infring recover',
    escalation_expected_output: 'runtime restart and gateway/install revalidation',
  };
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
  const explicitNonInteractiveOptIn = opts.yes || opts.defaults || asBool(process.env.INFRING_SETUP_NONINTERACTIVE_OPT_IN, false);
  let covenantAck = false;
  let interaction = pickInteraction(opts.interaction || 'silent');
  let notifications = pickNotifications(opts.notifications || 'none');

  if (opts.skip) {
    covenantAck = false;
    interaction = 'silent';
    notifications = 'none';
  } else if (nonInteractive) {
    covenantAck = !!explicitNonInteractiveOptIn;
    if (opts.interaction) {
      interaction = pickInteraction(opts.interaction);
    }
    if (opts.notifications) {
      notifications = pickNotifications(opts.notifications);
    }
  } else if (!nonInteractive) {
    covenantAck = true;
    interaction = pickInteraction(opts.interaction || 'proactive');
    notifications = pickNotifications(opts.notifications || 'critical');
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
    const recovery = buildSetupFailureRecovery(error);
    emit(
      opts.json,
      attachOnboardingReceipt(
        {
          ...(payload && typeof payload === 'object' ? payload : { ok: false, type: 'protheus_setup_wizard', error }),
          recovery,
        },
        'failed',
        'infring setup'
      ),
      ''
    );
    if (!opts.json) {
      process.stderr.write(`[infring setup] failed: ${error}\n`);
      process.stderr.write(`[infring setup] deterministic recovery path:\n`);
      process.stderr.write(`[infring setup]   retry: ${recovery.retry_command} (expect: ${recovery.retry_expected_output})\n`);
      process.stderr.write(`[infring setup]   status: ${recovery.status_command} (expect: ${recovery.status_expected_output})\n`);
      process.stderr.write(`[infring setup]   doctor: ${recovery.diagnostics_command} (expect: ${recovery.diagnostics_expected_output})\n`);
      process.stderr.write(`[infring setup]   escalate: ${recovery.escalation_command} (expect: ${recovery.escalation_expected_output})\n`);
    }
    return 1;
  }
  if (payload.skipped === true && String(payload.reason || '') === 'already_completed') {
    emit(
      opts.json,
      attachOnboardingReceipt(payload, 'already_completed', 'none'),
      '[infring setup] already completed'
    );
    return 0;
  }

  attachOnboardingReceipt(payload, 'completed', 'infring gateway');

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
  const completed = state && state.completed === true;
  const nextAction = completed ? 'none' : 'infring setup';
  const status = completed ? 'completed' : 'incomplete';
  const responsePayload = attachOnboardingReceipt(payload || {
    ok: false,
    type: 'protheus_setup_wizard',
    command: 'status',
    state_path: DEFAULT_STATE_PATH,
    state
  }, status, nextAction);
  responsePayload.setup_route = 'infring setup';
  if (opts.json) {
    process.stdout.write(`${JSON.stringify(responsePayload)}\n`);
    return 0;
  }
  if (completed) {
    process.stdout.write('[infring setup] completed\n');
  } else {
    process.stdout.write('[infring setup] pending (next: run `infring setup`)\n');
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

module.exports = {
  DEFAULT_STATE_PATH,
  cleanText,
  invokeSetupWizardKernel,
  main,
  parseArgs,
  resetWizard,
  runWizard,
  statusWizard,
};
