'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

const ROOT = path.resolve(__dirname, '..', '..');
const SERVICE_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js/shell/shared_shell_services.ts'
);

function loadServices() {
  const source = fs.readFileSync(SERVICE_PATH, 'utf8');
  const context = {
    window: {},
    Array,
    Math,
    Number,
    Object,
    String,
  };
  vm.createContext(context);
  vm.runInContext(source, context, { filename: SERVICE_PATH });
  return context.window.InfringSharedShellServices;
}

const services = loadServices();

assert(services, 'shared shell services should be published on window');
assert(services.message, 'message shell service should be available');
assert(services.overlay, 'overlay shell service should be available');
assert(services.glass, 'glass shell service should be available');

assert.strictEqual(
  services.message.canRequestEvalIssueReport({ role: 'assistant', text: 'done' }, { id: 'agent-1' }),
  true,
  'assistant-authored messages with content can request eval review'
);
assert.strictEqual(
  services.message.canRequestEvalIssueReport({ role: 'agent', tools: [{}] }, { id: 'agent-1' }),
  true,
  'agent-authored tool-only messages can request eval review'
);
assert.strictEqual(
  services.message.canRequestEvalIssueReport({ role: 'user', text: 'help' }, { id: 'agent-1' }),
  false,
  'user-authored messages cannot request eval review'
);
assert.strictEqual(
  services.message.canRequestEvalIssueReport({ role: 'assistant', thinking: true, text: 'thinking' }, { id: 'agent-1' }),
  false,
  'thinking placeholder messages cannot request eval review'
);

const placement = services.overlay.resolvePlacement(
  { left: 12, right: 52, top: 540, bottom: 580, width: 40, height: 40 },
  { left: 0, right: 1000, top: 0, bottom: 600, width: 1000, height: 600 }
);
assert.strictEqual(placement.horizontal, 'right', 'overlay should open away from the nearest vertical viewport wall');
assert.strictEqual(placement.vertical, 'above', 'overlay should open away from the nearest horizontal viewport wall');

assert.strictEqual(
  services.glass.surfaceClass('magnified-glass'),
  'warped-glass',
  'legacy magnified glass naming should normalize to warped glass'
);

console.log(JSON.stringify({ ok: true, type: 'shared_shell_services_test' }));
