#!/usr/bin/env node
'use strict';

const path = require('path');
const fs = require('fs');
const ts = require('typescript');

if (!require.extensions['.ts']) {
  require.extensions['.ts'] = function compileTs(module, filename) {
    const source = fs.readFileSync(filename, 'utf8');
    const output = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        esModuleInterop: true,
        allowSyntheticDefaultImports: true,
        sourceMap: false,
        declaration: false,
        removeComments: false
      },
      fileName: filename,
      reportDiagnostics: false
    }).outputText;
    module._compile(output, filename);
  };
}

const runtimeHelper = require(path.join(
  __dirname,
  '..',
  '..',
  'client',
  'runtime',
  'lib',
  'legacy_retired_wrapper.ts'
));

function normalizeLaneId(raw) {
  return runtimeHelper.normalizeLaneId(raw, 'MEMORY-TEST-LEGACY-RETIRED');
}

function createTestModule(scriptDir, scriptName, laneId) {
  return runtimeHelper.createLegacyRetiredModule(
    scriptDir,
    scriptName,
    normalizeLaneId(laneId)
  );
}

module.exports = {
  createTestModule,
  runAsMain: runtimeHelper.runAsMain,
  normalizeLaneId
};
