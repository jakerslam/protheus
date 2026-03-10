#!/usr/bin/env node
'use strict';

// App ownership: apps/examples/comment-mapper-demo (toolkit example app)

const { runToolkit } = require('../_shared/run_protheus_toolkit.js');

runToolkit([
  'comment-mapper',
  '--persona=vikram_menon',
  '--query=Should we prioritize memory or security first?',
  '--gap=1',
  '--active=1'
], { input: 'a\n' });
