#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer3/cognition + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const path = require('path');
const { sanitizeBridgeArg, normalizeReceiptHash } = require('../../../../../client/runtime/lib/runtime_system_entrypoint.ts');

const RUNTIME_PATH = path.resolve(
  __dirname,
  '..',
  '..',
  '..',
  '..',
  '..',
  'client',
  'cognition',
  'shared',
  'lib',
  'legacy_retired_wrapper.ts'
);

function errorPayload(errorCode, detail) {
  const payload = {
    ok: false,
    type: 'smtp_compat_wrapper',
    error: errorCode,
    detail: sanitizeBridgeArg(detail || '', 240),
    runtime_path: path.relative(path.resolve(__dirname, '..', '..', '..', '..', '..'), RUNTIME_PATH).replace(/\\/g, '/'),
  };
  payload.receipt_hash = normalizeReceiptHash(payload);
  return payload;
}

let runtime = null;
try {
  runtime = require(RUNTIME_PATH);
} catch (error) {
  const payload = errorPayload('smtp_runtime_load_failed', error && error.message ? error.message : String(error));
  if (require.main === module) {
    process.stderr.write(`${JSON.stringify(payload)}\n`);
    process.exit(1);
  }
  module.exports = payload;
}

if (runtime && typeof runtime.createCognitionModule === 'function' && typeof runtime.runAsMain === 'function') {
  const mod = runtime.createCognitionModule(
    __dirname,
    'smtp',
    'COGNITION-SKILLS-IMAP-SMTP-EMAIL-SCRIPTS-SMTP'
  );

  if (require.main === module) {
    runtime.runAsMain(mod, process.argv.slice(2));
  }

  module.exports = mod;
} else if (!module.exports || module.exports.ok !== false) {
  const payload = errorPayload('smtp_runtime_contract_invalid', 'legacy wrapper missing required exports');
  if (require.main === module) {
    process.stderr.write(`${JSON.stringify(payload)}\n`);
    process.exit(1);
  }
  module.exports = payload;
}
