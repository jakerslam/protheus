'use strict';

(function registerWorkflowBuilderSegmentedStub() {
  if (typeof window === 'undefined') return;
  var register = window['__infringRegisterSegmentedStub'];
  if (typeof register === 'function') {
    register('pages/workflow-builder.ts', 'workflow-builder.ts.parts');
    return;
  }
  if (typeof console !== 'undefined' && typeof console.warn === 'function') {
    console.warn('[infring-dashboard] segmented stub registrar missing for pages/workflow-builder.ts');
  }
})();
