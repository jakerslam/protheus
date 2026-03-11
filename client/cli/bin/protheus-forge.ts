#!/usr/bin/env node
'use strict';

const suite = require('../../runtime/systems/cli/protheus_suite_tooling.ts');
suite.runTool('forge', process.argv.slice(2));
