'use strict';

(function initSegmentedDashboardStubRegistry() {
  if (typeof window === 'undefined') return;
  var root = window;
  var register = root['__infringRegisterSegmentedStub'];
  if (typeof register !== 'function') {
    register = function registerSegmentedStub(segmentedFile, partsHint) {
      var file = String(segmentedFile || '').trim();
      var hint = String(partsHint || '').trim();
      if (!file || !hint) return;

      var rows = Array.isArray(root['__infringSegmentedStubRows']) ? root['__infringSegmentedStubRows'] : [];
      var exists = false;
      for (var i = 0; i < rows.length; i += 1) {
        if (rows[i] && String(rows[i].file || '') === file) {
          exists = true;
          break;
        }
      }
      if (!exists) {
        rows.push({
          file: file,
          parts: hint,
          ts: new Date().toISOString()
        });
      }
      root['__infringSegmentedStubRows'] = rows;

      if (root['__infringSegmentedStubWarned'] !== true) {
        root['__infringSegmentedStubWarned'] = true;
        try {
          root.dispatchEvent(new CustomEvent('infring:dashboard:segmented-stub', {
            detail: {
              ok: false,
              type: 'dashboard_segmented_stub_loaded',
              file: file,
              parts: hint
            }
          }));
        } catch (_) {}
        if (typeof console !== 'undefined' && typeof console.warn === 'function') {
          console.warn('[infring-dashboard] segmented source stub loaded; ensure *.parts assembly is present.');
        }
      }
    };
    root['__infringRegisterSegmentedStub'] = register;
  }

  register('app.ts', 'app.ts.parts');
})();
