#!/usr/bin/env node
'use strict';

// SRS coverage: V10-PHONE-001.1, V10-PHONE-001.2, V10-PHONE-001.3,
// V10-PHONE-001.4, V10-PHONE-001.5

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const ts = require('typescript');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

if (!require.extensions['.ts']) {
  require.extensions['.ts'] = function compileTs(module, filename) {
    const source = fs.readFileSync(filename, 'utf8');
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        esModuleInterop: true,
        allowSyntheticDefaultImports: true
      },
      fileName: filename,
      reportDiagnostics: false
    }).outputText;
    module._compile(transpiled, filename);
  };
}

const bridge = require('../../client/runtime/lib/phone_runtime_bridge.ts');
const adapter = require('../../adapters/protocol/phone_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'phone-runtime-bridge-'));
  const statePath = path.join(tmpDir, 'state.json');
  const historyPath = path.join(tmpDir, 'history.jsonl');
  const backgroundStatePath = path.join(tmpDir, 'background.json');
  const sensorStatePath = path.join(tmpDir, 'sensors.json');

  const battery = bridge.batterySchedule({
    battery_pct: 9,
    charging: false,
    thermal_c: 41,
    critical_tasks: ['notifications', 'voice_reply'],
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(battery.battery_event.selected_profile, 'tiny-max');
  assert.strictEqual(battery.battery_event.paused_noncritical, true);

  const sensors = adapter.sensorIntake({
    allowed_sensors: ['gps', 'camera', 'mic'],
    requested_sensors: [
      { name: 'gps', available: true, consent: true },
      { name: 'camera', available: false, consent: true },
      { name: 'mic', available: true, consent: false }
    ],
    state_path: statePath,
    history_path: historyPath,
    sensor_state_path: sensorStatePath,
  });
  assert.strictEqual(sensors.sensor_event.accepted.length, 1);
  assert.strictEqual(fs.existsSync(sensorStatePath), true);

  const interaction = adapter.interactionMode({
    modality: 'voice',
    target_latency_ms: 180,
    local_model_available: false,
    notification_lane: 'notify',
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(interaction.interaction_mode.transport, 'text-fallback');

  const background = adapter.backgroundDaemon({
    action: 'wake',
    platform: 'ios',
    handoff: 'edge',
    drain_budget_pct_24h: 4,
    wake_reason: 'push_notification',
    state_path: statePath,
    history_path: historyPath,
    background_state_path: backgroundStatePath,
  });
  assert.strictEqual(background.background_event.mode, 'wake');
  assert.strictEqual(fs.existsSync(backgroundStatePath), true);

  const profile = bridge.phoneProfile({
    platform: 'android',
    device_class: 'legacy-phone',
    memory_mb: 1024,
    cpu_cores: 2,
    battery_pct: 14,
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(profile.phone_profile.selected_profile, 'tiny-max');
  assert.strictEqual(profile.phone_profile.shed_capabilities.includes('vision'), true);

  const status = bridge.status({ state_path: statePath, history_path: historyPath });
  assert.strictEqual(status.battery_events, 1);
  assert.strictEqual(status.sensor_events, 1);
  assert.strictEqual(status.interaction_modes, 1);
  assert.strictEqual(status.background_events, 1);
  assert.strictEqual(status.profiles, 1);

  assertNoPlaceholderOrPromptLeak({ battery, sensors, interaction, background, profile, status }, 'phone_runtime_bridge_test');\n  assertStableToolingEnvelope(status, 'phone_runtime_bridge_test');\n  console.log(JSON.stringify({ ok: true, type: 'phone_runtime_bridge_test' }));
}

run();
