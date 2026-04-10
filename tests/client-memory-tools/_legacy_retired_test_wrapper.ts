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

function requireFresh(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function bindLegacyRetiredTest(
  currentModule,
  scriptDir,
  scriptName,
  laneId,
  argv = process.argv.slice(2)
) {
  const mod = createTestModule(scriptDir, scriptName, laneId);
  if (currentModule && require.main === currentModule) {
    runtimeHelper.runAsMain(mod, argv);
  }
  return mod;
}

module.exports = {
  bindLegacyRetiredTest,
  createTestModule,
  requireFresh,
  runAsMain: runtimeHelper.runAsMain,
  normalizeLaneId
};
