function infringSetTheme(page, mode) {
  page.beginInstantThemeFlip();
  page.themeMode = mode;
  localStorage.setItem('infring-theme-mode', mode);
  if (mode === 'system') {
    page.theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  } else {
    page.theme = mode;
  }
}

function infringOverlayGlassTemplateNormalized(modeRaw) {
  var mode = String(modeRaw || '').trim().toLowerCase();
  if (mode === 'simple-glass') return 'simple-glass';
  if (mode === 'fogged-glass') return 'fogged-glass';
  if (mode === 'warped-glass' || mode === 'magnified-glass') return 'warped-glass';
  if (mode === 'liquid-glass') return 'fogged-glass';
  return 'simple-glass';
}

function infringApplyOverlayGlassTemplate(page, modeRaw, persistRaw) {
  var mode = page.overlayGlassTemplateNormalized(modeRaw);
  page.overlayGlassTemplate = mode;
  var persist = persistRaw !== false;
  if (document && document.documentElement) {
    try {
      document.documentElement.setAttribute('data-overlay-glass-template', mode);
    } catch (_) {}
  }
  if (persist) {
    try {
      localStorage.setItem('infring-overlay-glass-template', mode);
    } catch (_) {}
  }
  return mode;
}

function infringUiBackgroundTemplateNormalized(page, modeRaw) {
  var service = page.taskbarDockService ? page.taskbarDockService() : infringTaskbarDockService();
  if (service && typeof service.normalizeBackgroundTemplate === 'function') return service.normalizeBackgroundTemplate(modeRaw);
  var mode = String(modeRaw || '').trim().toLowerCase();
  if (mode === 'unsplash-paper') return 'light-wood';
  if (mode === 'default-grid') return 'default-grid';
  if (mode === 'light-wood') return 'light-wood';
  if (mode === 'sand') return 'sand';
  return 'sand';
}

function infringApplyUiBackgroundTemplate(page, modeRaw, persistRaw) {
  var mode = page.uiBackgroundTemplateNormalized(modeRaw);
  page.uiBackgroundTemplate = mode;
  var persist = persistRaw !== false;
  if (document && document.documentElement) {
    try {
      document.documentElement.setAttribute('data-ui-background-template', mode);
    } catch (_) {}
  }
  if (persist) {
    try {
      var service = page.taskbarDockService ? page.taskbarDockService() : infringTaskbarDockService();
      if (service && typeof service.writeDisplayBackground === 'function') service.writeDisplayBackground(mode);
      else {
        var rawDisplaySettings = localStorage.getItem('infring-display-settings') || '';
        var displaySettings = rawDisplaySettings ? JSON.parse(rawDisplaySettings) : {};
        displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
        displaySettings.background = mode;
        localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
      }
    } catch (_) {}
  }
  return mode;
}

function infringBeginInstantThemeFlip(page) {
  var body = document && document.body ? document.body : null;
  if (!body) return;
  body.classList.add('theme-switching');
  // Force style flush so no-transition styles are applied before theme variables swap.
  void body.offsetHeight;
  if (page._themeSwitchReset) {
    clearTimeout(page._themeSwitchReset);
  }
  page._themeSwitchReset = window.setTimeout(function() {
    body.classList.remove('theme-switching');
    page._themeSwitchReset = 0;
  }, 260);
}

function infringToggleTheme(page) {
  var modes = ['light', 'system', 'dark'];
  var next = modes[(modes.indexOf(page.themeMode) + 1) % modes.length];
  page.setTheme(next);
}
