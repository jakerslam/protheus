#!/usr/bin/env node
'use strict';

const suite = require('../../lib/infring_suite_tooling.ts');
suite.runTool('econ', process.argv.slice(2));
