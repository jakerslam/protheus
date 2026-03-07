#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');

// Install TS require hook via a stable bootstrap wrapper.
require('../../../lib/rust_lane_bridge.js');

function requireJsOrTs(relFromClientRoot) {
  const clientRoot = path.resolve(__dirname, '..', '..', '..');
  const jsAbs = path.join(clientRoot, `${relFromClientRoot}.js`);
  if (fs.existsSync(jsAbs)) return require(jsAbs);
  const tsAbs = path.join(clientRoot, `${relFromClientRoot}.ts`);
  if (fs.existsSync(tsAbs)) return require(tsAbs);
  throw new Error(`missing_module_variant:${relFromClientRoot}`);
}

function run() {
  const layerStore = requireJsOrTs('systems/adaptive/core/layer_store');
  const catalogStore = requireJsOrTs('systems/adaptive/sensory/eyes/catalog_store');
  const focusStore = requireJsOrTs('systems/adaptive/sensory/eyes/focus_trigger_store');
  const habitStore = requireJsOrTs('systems/adaptive/habits/habit_store');
  const reflexStore = requireJsOrTs('systems/adaptive/reflex/reflex_store');
  const strategyStore = requireJsOrTs('systems/adaptive/strategy/strategy_store');

  assert.throws(
    () => layerStore.resolveAdaptivePath('/tmp/not_under_adaptive.json'),
    /outside adaptive root/
  );

  assert.throws(
    () => catalogStore.readCatalog('/tmp/not_catalog.json'),
    /override denied/
  );
  assert.throws(
    () => focusStore.readFocusState('/tmp/not_focus.json'),
    /override denied/
  );
  assert.throws(
    () => habitStore.readHabitState('/tmp/not_habits.json'),
    /override denied/
  );
  assert.throws(
    () => reflexStore.readReflexState('/tmp/not_reflex.json'),
    /override denied/
  );
  assert.throws(
    () => strategyStore.readStrategyState('/tmp/not_strategy.json'),
    /override denied/
  );

  console.log('adaptive_layer_boundary_guards.test.js: OK');
}

try {
  run();
} catch (err) {
  console.error(`adaptive_layer_boundary_guards.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
