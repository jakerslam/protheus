'use strict';

(function registerWizardSegmentedStub() {
  if (typeof window === 'undefined') return;
  var register = window['__infringRegisterSegmentedStub'];
  if (typeof register === 'function') {
    register('pages/wizard.ts', 'wizard.ts.parts');
    return;
  }
  if (typeof console !== 'undefined' && typeof console.warn === 'function') {
    console.warn('[infring-dashboard] segmented stub registrar missing for pages/wizard.ts');
  }
})();
