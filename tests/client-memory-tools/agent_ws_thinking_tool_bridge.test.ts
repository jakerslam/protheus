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
const renderSource = read(
  path.resolve(
    ROOT,
    'client/runtime/systems/ui/infring_static/js/pages/chat.ts.parts/160-runtime-events-and-render.part01.ts'
  )
);
const bodyPartSource = read(
  path.resolve(
    ROOT,
    'client/runtime/systems/ui/infring_static/index_body.html.parts/0005-body-part.html'
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
  !bridgeSource.includes('phaseCycle'),
  'agent ws bridge must not run synthetic rotating phase carousel text'
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
assert.ok(
  bodyPartSource.includes("thinkingStatusText(msg) || 'Thinking'") &&
    !bodyPartSource.includes('thinking-trace-list'),
  'thinking bubble template must render a single primary status line without stacked trace rows'
);
assert.ok(
  renderSource.includes('nextThoughtSentenceFrame') &&
    renderSource.includes("return 'Thinking';"),
  'thinking runtime must keep one single-line sentence and default to Thinking when no active line exists'
);
