#!/usr/bin/env node
'use strict';

const path = require('path');
const readline = require('readline');
const fs = require('fs');
const { invokeKernelPayload } = require('./infring_kernel_bridge.ts');

const ROOT = path.resolve(__dirname, '..', '..');
const DEFAULT_STATE_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'infring_setup_wizard',
  'latest.json'
);
const FIRST_RUN_POLICY_PATH = path.join(
  ROOT,
  'client',
  'runtime',
  'config',
  'first_run_onboarding_wizard_policy.json'
);
const DEFAULT_FIRST_RUN_POLICY = Object.freeze({
  schema_id: 'first_run_onboarding_wizard_policy',
  schema_version: '1.0',
  incomplete_state_route: 'infring setup',
  incomplete_state_status: 'pending_setup',
  incomplete_state_handoff: {
    retry_command: 'infring setup --yes --defaults',
    status_command: 'infring setup status --json',
    diagnostics_command: 'infring doctor --json'
  },
  receipt_contract: {
    required_fields: ['mode', 'workspace_root', 'status', 'next_action', 'mode_contract', 'handoff'],
    incomplete_next_action: 'infring setup'
  }
});
const BEHAVIOR_PROFILE_ROOT = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'behavior_profiles'
);
const BEHAVIOR_PROFILE_GLOBAL_PATH = path.join(BEHAVIOR_PROFILE_ROOT, 'global.json');
const BEHAVIOR_PROFILE_PROJECTS_DIR = path.join(BEHAVIOR_PROFILE_ROOT, 'projects');
const DEFAULT_BEHAVIOR_PROFILE_PRESET = 'balanced';
const DEFAULT_BEHAVIOR_PROFILE_PRESETS = Object.freeze({
  balanced: Object.freeze({
    verbosity: 'medium',
    autonomy: 'medium',
    risk_appetite: 'medium',
    tool_boundaries: 'balanced'
  }),
  conservative: Object.freeze({
    verbosity: 'low',
    autonomy: 'low',
    risk_appetite: 'low',
    tool_boundaries: 'strict'
  }),
  autonomous: Object.freeze({
    verbosity: 'medium',
    autonomy: 'high',
    risk_appetite: 'high',
    tool_boundaries: 'open'
  }),
  verbose: Object.freeze({
    verbosity: 'high',
    autonomy: 'medium',
    risk_appetite: 'medium',
    tool_boundaries: 'balanced'
  }),
  minimal: Object.freeze({
    verbosity: 'low',
    autonomy: 'low',
    risk_appetite: 'low',
    tool_boundaries: 'strict'
  })
});

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
    notifications: '',
    preset: '',
    verbosity: '',
    autonomy: '',
    riskAppetite: '',
    toolBoundaries: '',
    projectOverride: false
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
    if (token.startsWith('--preset=')) {
      out.preset = cleanText(token.slice('--preset='.length), 40);
      continue;
    }
    if (token === '--preset' && tokens[i + 1]) {
      out.preset = cleanText(tokens[i + 1], 40);
      i += 1;
      continue;
    }
    if (token.startsWith('--verbosity=')) {
      out.verbosity = cleanText(token.slice('--verbosity='.length), 40);
      continue;
    }
    if (token === '--verbosity' && tokens[i + 1]) {
      out.verbosity = cleanText(tokens[i + 1], 40);
      i += 1;
      continue;
    }
    if (token.startsWith('--autonomy=')) {
      out.autonomy = cleanText(token.slice('--autonomy='.length), 40);
      continue;
    }
    if (token === '--autonomy' && tokens[i + 1]) {
      out.autonomy = cleanText(tokens[i + 1], 40);
      i += 1;
      continue;
    }
    if (token.startsWith('--risk-appetite=')) {
      out.riskAppetite = cleanText(token.slice('--risk-appetite='.length), 40);
      continue;
    }
    if (token === '--risk-appetite' && tokens[i + 1]) {
      out.riskAppetite = cleanText(tokens[i + 1], 40);
      i += 1;
      continue;
    }
    if (token.startsWith('--tool-boundaries=')) {
      out.toolBoundaries = cleanText(token.slice('--tool-boundaries='.length), 40);
      continue;
    }
    if (token === '--tool-boundaries' && tokens[i + 1]) {
      out.toolBoundaries = cleanText(tokens[i + 1], 40);
      i += 1;
      continue;
    }
    if (token === '--project-override' || token === '--project-override=1') {
      out.projectOverride = true;
      continue;
    }
    if (token === '--project-override=0') {
      out.projectOverride = false;
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

function pickBehaviorPreset(raw) {
  const normalized = cleanText(raw, 40).toLowerCase();
  if (Object.prototype.hasOwnProperty.call(DEFAULT_BEHAVIOR_PROFILE_PRESETS, normalized)) {
    return normalized;
  }
  return DEFAULT_BEHAVIOR_PROFILE_PRESET;
}

function pickBehaviorVerbosity(raw, fallback = 'medium') {
  const normalized = cleanText(raw, 40).toLowerCase();
  if (['low', 'medium', 'high'].includes(normalized)) return normalized;
  return fallback;
}

function pickBehaviorAutonomy(raw, fallback = 'medium') {
  const normalized = cleanText(raw, 40).toLowerCase();
  if (['low', 'medium', 'high'].includes(normalized)) return normalized;
  return fallback;
}

function pickBehaviorRiskAppetite(raw, fallback = 'medium') {
  const normalized = cleanText(raw, 40).toLowerCase();
  if (['low', 'medium', 'high'].includes(normalized)) return normalized;
  return fallback;
}

function pickBehaviorToolBoundaries(raw, fallback = 'balanced') {
  const normalized = cleanText(raw, 40).toLowerCase();
  if (['strict', 'balanced', 'open'].includes(normalized)) return normalized;
  return fallback;
}

function profileProjectKey(workspaceRoot) {
  const normalized = cleanText(workspaceRoot, 300)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 96);
  return normalized || 'workspace_default';
}

function readJsonFileSafe(filePath, fallback = {}) {
  try {
    const raw = fs.readFileSync(filePath, 'utf8');
    const parsed = JSON.parse(String(raw || '{}'));
    if (!parsed || typeof parsed !== 'object') return fallback;
    return parsed;
  } catch (_) {
    return fallback;
  }
}

function writeJsonFile(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function resolveBehaviorProfile(opts, workspaceRoot, source) {
  const preset = pickBehaviorPreset(
    opts.preset ||
      process.env.INFRING_BEHAVIOR_PRESET ||
      process.env.INFRING_BEHAVIOR_PRESET ||
      DEFAULT_BEHAVIOR_PROFILE_PRESET
  );
  const presetProfile = DEFAULT_BEHAVIOR_PROFILE_PRESETS[preset] || DEFAULT_BEHAVIOR_PROFILE_PRESETS.balanced;
  const profile = {
    verbosity: pickBehaviorVerbosity(opts.verbosity || process.env.INFRING_BEHAVIOR_VERBOSITY, presetProfile.verbosity),
    autonomy: pickBehaviorAutonomy(opts.autonomy || process.env.INFRING_BEHAVIOR_AUTONOMY, presetProfile.autonomy),
    risk_appetite: pickBehaviorRiskAppetite(
      opts.riskAppetite || process.env.INFRING_BEHAVIOR_RISK_APPETITE,
      presetProfile.risk_appetite
    ),
    tool_boundaries: pickBehaviorToolBoundaries(
      opts.toolBoundaries || process.env.INFRING_BEHAVIOR_TOOL_BOUNDARIES,
      presetProfile.tool_boundaries
    )
  };
  const diff = {};
  for (const key of Object.keys(profile)) {
    if (profile[key] !== presetProfile[key]) {
      diff[key] = { from: presetProfile[key], to: profile[key] };
    }
  }
  const workspace = cleanText(workspaceRoot, 400) || detectWorkspaceRoot();
  const projectKey = profileProjectKey(workspace);
  const projectPath = path.join(BEHAVIOR_PROFILE_PROJECTS_DIR, `${projectKey}.json`);
  const isProjectOverride =
    !!opts.projectOverride ||
    asBool(process.env.INFRING_BEHAVIOR_PROJECT_OVERRIDE, false);
  const scope = isProjectOverride ? 'project_override' : 'global_default';
  const persistedAt = new Date().toISOString();
  const entry = {
    schema_id: 'infring_behavior_profile_v1',
    schema_version: '1.0',
    source: cleanText(source, 80) || 'setup',
    workspace_root: workspace,
    project_key: projectKey,
    scope,
    preset,
    profile,
    diff,
    persisted_at: persistedAt
  };
  if (isProjectOverride) {
    writeJsonFile(projectPath, entry);
  } else {
    writeJsonFile(BEHAVIOR_PROFILE_GLOBAL_PATH, entry);
  }
  return {
    ...entry,
    profile_global_path: BEHAVIOR_PROFILE_GLOBAL_PATH,
    profile_project_path: projectPath
  };
}

function loadBehaviorProfileSnapshot(workspaceRoot) {
  const workspace = cleanText(workspaceRoot, 400) || detectWorkspaceRoot();
  const projectKey = profileProjectKey(workspace);
  const projectPath = path.join(BEHAVIOR_PROFILE_PROJECTS_DIR, `${projectKey}.json`);
  const projectEntry = readJsonFileSafe(projectPath, null);
  if (projectEntry && typeof projectEntry === 'object' && Object.keys(projectEntry).length > 0) {
    return {
      ...projectEntry,
      profile_global_path: BEHAVIOR_PROFILE_GLOBAL_PATH,
      profile_project_path: projectPath,
      active_scope: 'project_override'
    };
  }
  const globalEntry = readJsonFileSafe(BEHAVIOR_PROFILE_GLOBAL_PATH, null);
  if (globalEntry && typeof globalEntry === 'object' && Object.keys(globalEntry).length > 0) {
    return {
      ...globalEntry,
      profile_global_path: BEHAVIOR_PROFILE_GLOBAL_PATH,
      profile_project_path: projectPath,
      active_scope: 'global_default'
    };
  }
  const preset = DEFAULT_BEHAVIOR_PROFILE_PRESET;
  const profile = DEFAULT_BEHAVIOR_PROFILE_PRESETS[preset];
  return {
    schema_id: 'infring_behavior_profile_v1',
    schema_version: '1.0',
    source: 'default',
    workspace_root: workspace,
    project_key: projectKey,
    scope: 'global_default',
    active_scope: 'default',
    preset,
    profile,
    diff: {},
    profile_global_path: BEHAVIOR_PROFILE_GLOBAL_PATH,
    profile_project_path: projectPath
  };
}

function detectRuntimeMode() {
  const explicit = cleanText(
    process.env.INFRING_INSTALL_MODE ||
      process.env.INFRING_RUNTIME_MODE ||
      process.env.INFRING_RUNTIME_MODE,
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
      process.env.INFRING_WORKSPACE_ROOT ||
      process.cwd(),
    400
  );
}

function loadFirstRunOnboardingPolicy() {
  try {
    const raw = fs.readFileSync(FIRST_RUN_POLICY_PATH, 'utf8');
    const parsed = JSON.parse(String(raw || '{}'));
    if (!parsed || typeof parsed !== 'object') {
      return DEFAULT_FIRST_RUN_POLICY;
    }
    return {
      ...DEFAULT_FIRST_RUN_POLICY,
      ...parsed,
      incomplete_state_handoff: {
        ...DEFAULT_FIRST_RUN_POLICY.incomplete_state_handoff,
        ...(parsed.incomplete_state_handoff && typeof parsed.incomplete_state_handoff === 'object'
          ? parsed.incomplete_state_handoff
          : {})
      },
      receipt_contract: {
        ...DEFAULT_FIRST_RUN_POLICY.receipt_contract,
        ...(parsed.receipt_contract && typeof parsed.receipt_contract === 'object'
          ? parsed.receipt_contract
          : {})
      }
    };
  } catch (_) {
    return DEFAULT_FIRST_RUN_POLICY;
  }
}

function buildModeContract(mode) {
  const normalized = cleanText(mode, 40).toLowerCase() || 'full';
  if (normalized === 'pure') {
    return {
      mode: 'pure',
      gateway_available: true,
      dashboard_surface: 'limited_optional',
      dashboard_opt_in_command: 'none',
      dashboard_opt_in_reason: 'pure_mode_keeps_optional_ui_surfaces_limited',
      auto_open_dashboard_noninteractive: false,
      setup_interaction_default: 'conservative_noninteractive',
      capability_reason: 'rust_first_profile_optional_ui_surfaces_limited'
    };
  }
  if (normalized === 'tiny-max') {
    return {
      mode: 'tiny-max',
      gateway_available: true,
      dashboard_surface: 'limited_optional',
      dashboard_opt_in_command: 'none',
      dashboard_opt_in_reason: 'tiny_max_mode_keeps_optional_ui_surfaces_limited',
      auto_open_dashboard_noninteractive: false,
      setup_interaction_default: 'conservative_noninteractive',
      capability_reason: 'tiny_max_profile_minimal_footprint_optional_ui_surfaces_limited'
    };
  }
  if (normalized === 'minimal') {
    return {
      mode: 'minimal',
      gateway_available: true,
      dashboard_surface: 'optional_limited',
      dashboard_opt_in_command: 'infring gateway start --dashboard-open=1',
      dashboard_opt_in_reason: 'minimal_mode_requires_explicit_dashboard_opt_in',
      auto_open_dashboard_noninteractive: false,
      setup_interaction_default: 'explicit_opt_in_recommended',
      capability_reason: 'minimal_profile_install_light_optional_surfaces_may_require_explicit_setup'
    };
  }
  return {
    mode: 'full',
    gateway_available: true,
    dashboard_surface: 'available',
    dashboard_opt_in_command: 'infring gateway start --dashboard-open=1',
    dashboard_opt_in_reason: 'full_mode_requires_explicit_opt_in_for_noninteractive_dashboard_open',
    auto_open_dashboard_noninteractive: false,
    setup_interaction_default: 'interactive_on_tty_conservative_noninteractive',
    capability_reason: 'full_profile_enables_complete_operator_surface'
  };
}

function buildOnboardingReceipt(status, nextAction = 'none') {
  const policy = loadFirstRunOnboardingPolicy();
  const mode = detectRuntimeMode();
  const normalizedStatus = cleanText(status, 80).toLowerCase() || 'unknown';
  const setupIncomplete = !['completed', 'already_completed'].includes(normalizedStatus);
  const incompleteRoute = cleanText(policy.incomplete_state_route, 160) || 'infring setup';
  const incompleteStatus = cleanText(policy.incomplete_state_status, 80).toLowerCase() || 'pending_setup';
  const fallbackRetryCommand = cleanText(policy.incomplete_state_handoff.retry_command, 200) || 'infring setup --yes --defaults';
  const statusCommand = cleanText(policy.incomplete_state_handoff.status_command, 200) || 'infring setup status --json';
  const diagnosticsCommand = cleanText(policy.incomplete_state_handoff.diagnostics_command, 200) || 'infring doctor --json';
  const guidedNextAction = setupIncomplete
    ? incompleteRoute
    : cleanText(nextAction, 160) || 'none';
  const handoff = {
    route: setupIncomplete ? 'incomplete_state_setup_handoff_v1' : 'completed_state_noop_handoff_v1',
    setup_route: incompleteRoute,
    status_command: statusCommand,
    status_expected_output:
      'onboarding_receipt.status is completed or incomplete with mode/workspace metadata',
    retry_command: setupIncomplete ? fallbackRetryCommand : 'none',
    retry_expected_output: setupIncomplete
      ? 'saved profile confirmation with onboarding receipt'
      : 'none',
    diagnostics_command: diagnosticsCommand,
    diagnostics_expected_output: 'deterministic install/runtime diagnostics contract',
    route_reason: setupIncomplete ? incompleteStatus : 'setup_completed',
  };
  const requiredFields = Array.isArray(policy.receipt_contract.required_fields)
    ? policy.receipt_contract.required_fields.map((entry) => cleanText(entry, 80)).filter(Boolean)
    : DEFAULT_FIRST_RUN_POLICY.receipt_contract.required_fields;
  return {
    mode,
    workspace_root: detectWorkspaceRoot(),
    status: normalizedStatus,
    next_action: guidedNextAction,
    setup_route: incompleteRoute,
    setup_status_command: statusCommand,
    mode_contract: buildModeContract(mode),
    policy: {
      path: FIRST_RUN_POLICY_PATH,
      schema_id: cleanText(policy.schema_id, 80) || DEFAULT_FIRST_RUN_POLICY.schema_id,
      schema_version: cleanText(policy.schema_version, 40) || DEFAULT_FIRST_RUN_POLICY.schema_version,
      incomplete_state_route: incompleteRoute,
      incomplete_state_status: incompleteStatus
    },
    receipt_contract: {
      required_fields: requiredFields,
      incomplete_next_action:
        cleanText(policy.receipt_contract.incomplete_next_action, 160) || incompleteRoute
    },
    handoff,
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
    recovery_contract_version: '1.1',
    error_code: normalizedError,
    retry_command: 'infring setup --yes --defaults',
    retry_expected_output: 'saved profile confirmation with onboarding receipt',
    status_command: 'infring setup status --json',
    status_expected_output:
      'onboarding_receipt.status is completed or incomplete with mode/workspace metadata',
    diagnostics_command: 'infring doctor --json',
    diagnostics_expected_output: 'deterministic install/runtime diagnostics contract',
    escalation_command: 'infring recover',
    escalation_expected_output: 'runtime restart and gateway/install revalidation',
    recovery_step_order: [
      'retry_command',
      'status_command',
      'diagnostics_command',
      'escalation_command'
    ]
  };
}

function buildNonInteractiveOptInRecovery() {
  const mode = detectRuntimeMode();
  const modeContract = buildModeContract(mode);
  return {
    route: 'setup_noninteractive_opt_in_required_v1',
    reason: 'noninteractive_opt_in_required',
    mode,
    mode_contract: modeContract,
    noninteractive_opt_in_required: true,
    noninteractive_opt_in_command: 'infring setup --yes --defaults',
    noninteractive_opt_in_expected_output:
      'setup profile saved with onboarding receipt and deterministic next action',
    retry_command: 'infring setup --yes --defaults',
    retry_expected_output: 'saved profile confirmation with onboarding receipt',
    status_command: 'infring setup status --json',
    status_expected_output:
      'onboarding_receipt.status is completed or incomplete with mode/workspace metadata',
    diagnostics_command: 'infring doctor --json',
    diagnostics_expected_output: 'deterministic install/runtime diagnostics contract',
    dashboard_opt_in_command: modeContract.dashboard_opt_in_command || 'none',
    dashboard_opt_in_reason:
      modeContract.dashboard_opt_in_reason || 'explicit_opt_in_required_for_optional_surfaces',
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
  const explicitNonInteractiveOptIn =
    opts.yes || opts.defaults || asBool(process.env.INFRING_SETUP_NONINTERACTIVE_OPT_IN, false);
  const requireNonInteractiveOptIn =
    nonInteractive && !opts.skip && !explicitNonInteractiveOptIn;
  let covenantAck = false;
  let interaction = pickInteraction(opts.interaction || 'silent');
  let notifications = pickNotifications(opts.notifications || 'none');

  if (requireNonInteractiveOptIn) {
    const recovery = buildNonInteractiveOptInRecovery();
    const payload = attachOnboardingReceipt(
      {
        ok: true,
        type: 'infring_setup_wizard',
        command: 'run',
        deferred: true,
        deferred_reason: 'noninteractive_opt_in_required',
        noninteractive: true,
        noninteractive_opt_in_required: true,
        noninteractive_opt_in_command: recovery.noninteractive_opt_in_command,
        noninteractive_opt_in_expected_output: recovery.noninteractive_opt_in_expected_output,
        recovery,
      },
      'incomplete',
      recovery.retry_command,
    );
    emit(
      opts.json,
      payload,
      '[infring setup] non-interactive session detected; setup deferred until explicit opt-in (--yes --defaults).',
    );
    return 0;
  }

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
  const behaviorProfile = resolveBehaviorProfile(
    opts,
    detectWorkspaceRoot(),
    nonInteractive ? 'setup_noninteractive' : 'setup_interactive'
  );

  const payload = invokeSetupWizardKernel({
    command: 'run',
    force: !!opts.force,
    skip: !!opts.skip,
    defaults: !!opts.defaults,
    yes: !!opts.yes,
    interaction,
    notifications,
    covenant_acknowledged: covenantAck,
    behavior_profile: behaviorProfile.profile,
    behavior_profile_preset: behaviorProfile.preset,
    behavior_profile_scope: behaviorProfile.scope,
    behavior_profile_diff: behaviorProfile.diff
  });
  if (!payload || payload.ok !== true) {
    const error = cleanText(payload && payload.error ? payload.error : 'setup_wizard_kernel_failed', 240);
    const recovery = buildSetupFailureRecovery(error);
    emit(
      opts.json,
      attachOnboardingReceipt(
        {
          ...(payload && typeof payload === 'object' ? payload : { ok: false, type: 'infring_setup_wizard', error }),
          behavior_profile: behaviorProfile.profile,
          behavior_profile_preset: behaviorProfile.preset,
          behavior_profile_scope: behaviorProfile.scope,
          behavior_profile_diff: behaviorProfile.diff,
          behavior_profile_paths: {
            global: behaviorProfile.profile_global_path,
            project: behaviorProfile.profile_project_path
          },
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
  payload.behavior_profile = behaviorProfile.profile;
  payload.behavior_profile_preset = behaviorProfile.preset;
  payload.behavior_profile_scope = behaviorProfile.scope;
  payload.behavior_profile_diff = behaviorProfile.diff;
  payload.behavior_profile_paths = {
    global: behaviorProfile.profile_global_path,
    project: behaviorProfile.profile_project_path
  };

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
    type: 'infring_setup_wizard_state',
    completed: false,
    version: 1
  };
  const state = payload && payload.state && typeof payload.state === 'object'
    ? payload.state
    : fallbackState;
  const completed = state && state.completed === true;
  const nextAction = completed ? 'none' : 'infring setup';
  const status = completed ? 'completed' : 'incomplete';
  const behaviorProfile = loadBehaviorProfileSnapshot(detectWorkspaceRoot());
  const responsePayload = attachOnboardingReceipt(payload || {
    ok: false,
    type: 'infring_setup_wizard',
    command: 'status',
    state_path: DEFAULT_STATE_PATH,
    state
  }, status, nextAction);
  responsePayload.behavior_profile = behaviorProfile.profile || {};
  responsePayload.behavior_profile_preset = behaviorProfile.preset || DEFAULT_BEHAVIOR_PROFILE_PRESET;
  responsePayload.behavior_profile_scope = behaviorProfile.active_scope || behaviorProfile.scope || 'default';
  responsePayload.behavior_profile_diff = behaviorProfile.diff || {};
  responsePayload.behavior_profile_paths = {
    global: behaviorProfile.profile_global_path || BEHAVIOR_PROFILE_GLOBAL_PATH,
    project: behaviorProfile.profile_project_path || path.join(BEHAVIOR_PROFILE_PROJECTS_DIR, `${profileProjectKey(detectWorkspaceRoot())}.json`)
  };
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
  emit(opts.json, payload || { ok: false, type: 'infring_setup_wizard', command: 'reset' }, removed ? '[infring setup] reset complete' : '[infring setup] nothing to reset');
  return 0;
}

async function main(argv = process.argv.slice(2)) {
  const opts = parseArgs(argv);
  if (opts.command === 'help' || opts.command === '--help' || opts.command === '-h') {
    const usage = invokeSetupWizardKernel({ command: 'help' });
    const lines = usage && Array.isArray(usage.usage)
      ? usage.usage
      : [
          'infring setup [run|status|reset] [--json]',
          'infring setup run [--force] [--yes] [--defaults] [--interaction=<proactive|silent>] [--notifications=<all|critical|none>]',
          'infring setup run --skip',
          'infring setup status',
          'infring setup reset'
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
