'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..', '..');

function read(filePath) {
  return fs.readFileSync(filePath, 'utf8');
}

const bridgeSource = read(
  path.resolve(ROOT, 'client/runtime/systems/ui/agent_ws_bridge.ts')
);
const wsHandlerSource = read(
  path.resolve(
    ROOT,
    'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/150-ws-stream-handlers.ts'
  )
);

assert.ok(
  bridgeSource.includes("type: 'phase'"),
  'agent ws bridge must emit phase updates while turns are in-flight'
);
assert.ok(
  bridgeSource.includes("type: 'tool_start'") &&
    bridgeSource.includes("type: 'tool_result'") &&
    bridgeSource.includes("type: 'tool_end'"),
  'agent ws bridge must emit tool lifecycle events for thought bubble transparency'
);
assert.ok(
  bridgeSource.includes('tools: toolRows'),
  'agent ws bridge must pass normalized tool rows on final response payloads'
);
assert.ok(
  bridgeSource.includes('live_tool_steps') &&
    bridgeSource.includes('tool_status'),
  'agent ws bridge must use tool_completion receipt status for live tool sentence sync'
);
assert.ok(
  wsHandlerSource.includes('var responseTools = Array.isArray(data.tools)') &&
    wsHandlerSource.includes('streamedTools = responseTools;'),
  'chat ws response handler must hydrate fallback tool cards from response.tools'
);
assert.ok(
  wsHandlerSource.includes('data && data.tool_status'),
  'chat ws tool handlers must prefer receipt-driven tool_status labels when available'
);
assert.ok(
  wsHandlerSource.includes("rtool.name || '').toLowerCase() === 'thought_process'"),
  'chat ws response handler must recover thought content from thought_process tool cards'
);
