#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { invokeKernelPayload } = require('../../lib/infring_kernel_bridge.ts');

process.env.INFRING_OPS_USE_PREBUILT = process.env.INFRING_OPS_USE_PREBUILT || '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';

function synthesizeEnvelope(row = {}) {
  const out = invokeKernelPayload(
    'conversation-eye-synthesizer-kernel',
    'synthesize-envelope',
    row && typeof row === 'object' ? row : {},
    {
      fallbackError: 'conversation_eye_synthesizer_kernel_synthesize-envelope_bridge_failed',
    },
  );
  return out.envelope && typeof out.envelope === 'object' ? out.envelope : null;
}

module.exports = {
  synthesizeEnvelope
};
