#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const mod = require('../../client/lib/redaction_classification.ts');

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'redaction-classification-'));
const policyPath = path.join(tempDir, 'policy.json');
fs.writeFileSync(policyPath, JSON.stringify({
  rules: [
    {
      id: 'email',
      category: 'pii',
      action: 'redact',
      regex: '[A-Z0-9._%+-]+@[A-Z0-9.-]+\\.[A-Z]{2,}',
      flags: 'gi'
    },
    {
      id: 'secret',
      category: 'secret',
      action: 'block',
      regex: 'sk-[A-Za-z0-9]{12,}',
      flags: 'g'
    }
  ]
}, null, 2));

const policy = mod.loadPolicy(policyPath);
assert(Array.isArray(policy.rules), 'expected rules policy to load');

const classified = mod.classifyText('reach me at jay@example.com with sk-123456789012', policyPath);
assert(classified.labels.includes('pii'), 'expected pii label');
assert(classified.labels.includes('secret'), 'expected secret label');

const redacted = mod.redactText('jay@example.com sk-123456789012', policyPath, '[MASKED]');
assert(redacted.text.includes('[MASKED]'), 'expected sensitive values to redact');

const combined = mod.classifyAndRedact('jay@example.com sk-123456789012', policyPath, '[MASKED]');
assert(combined.classification.labels.includes('pii'));
assert(combined.redaction.text.includes('[MASKED]'));

assertNoPlaceholderOrPromptLeak({ policy, classified, redacted, combined }, 'redaction_classification_rust_bridge_test');\nassertStableToolingEnvelope(combined.classification, 'redaction_classification_rust_bridge_test');\nfs.rmSync(tempDir, { recursive: true, force: true });
console.log(JSON.stringify({ ok: true, type: 'redaction_classification_rust_bridge_test' }));
