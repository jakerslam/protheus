#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { requireFresh } = require('./_legacy_retired_test_wrapper.ts');

const ROOT = path.resolve(__dirname, '../..');
const mod = requireFresh(path.join(ROOT, 'client/runtime/systems/sensory/conversation_eye_synthesizer.ts'));
const envelope = mod.synthesizeEnvelope({ message: 'hello world', severity: 'high', tags: ['urgent'] });
assert.equal(envelope.level, 1);
assert.equal(envelope.level_token, 'jot1');
assert.equal(envelope.node_kind, 'insight');
assert.equal(envelope.node_tags.includes('urgent'), true);
assert.match(envelope.node_id, /^conversation-eye-/);
console.log(JSON.stringify({ ok: true, type: 'conversation_eye_synthesizer_rust_bridge_test' }));
