'use strict';

(function registerHandsSegmentedStub() {
  if (typeof window === 'undefined') return;
  var register = window['__infringRegisterSegmentedStub'];
  if (typeof register === 'function') {
    register('pages/hands.ts', 'hands.ts.parts');
    return;
  }
  if (typeof console !== 'undefined' && typeof console.warn === 'function') {
    console.warn('[infring-dashboard] segmented stub registrar missing for pages/hands.ts');
  }
})();
