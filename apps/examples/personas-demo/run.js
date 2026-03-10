#!/usr/bin/env node
'use strict';

// App ownership: apps/examples/personas-demo (toolkit example app)

const { runToolkit } = require('../_shared/run_protheus_toolkit.js');

runToolkit(['personas', '--list']);
