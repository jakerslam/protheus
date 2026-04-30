function infringInitAppShell(page) {
  var self = page;
  var appStoreBridge = typeof page.shellAppStoreBridge === 'function' ? page.shellAppStoreBridge() : null;
  if (appStoreBridge && typeof appStoreBridge.registerShellRoot === 'function') {
    appStoreBridge.registerShellRoot(page);
  }
  page._bootSplashStartedAt = Date.now();
  page.bootSplashVisible = true;
  page.applyOverlayGlassTemplate('simple-glass', true);
  if (typeof page.resetBootProgress === 'function') page.resetBootProgress();
  if (typeof page.setBootProgressEvent === 'function') page.setBootProgressEvent('splash_visible');
  if (typeof page.hideDashboardPopupBySource === 'function') page.hideDashboardPopupBySource('sidebar');
  if (page._bootSplashMaxTimer) {
    clearTimeout(page._bootSplashMaxTimer);
    page._bootSplashMaxTimer = 0;
  }
  page._bootSplashMaxTimer = window.setTimeout(function() {
    self.releaseBootSplash(true);
  }, Number(page._bootSplashMaxMs || 5000));
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function(e) {
    if (self.themeMode === 'system') {
      self.beginInstantThemeFlip();
      self.theme = e.matches ? 'dark' : 'light';
    }
  });
  var validPages = ['chat','agents','sessions','approvals','comms','workflows','scheduler','channels','eyes','skills','hands','overview','analytics','logs','runtime','settings','wizard'];
  var pageRedirects = {
    'automation': 'scheduler',
    'templates': 'agents',
    'triggers': 'workflows',
    'cron': 'scheduler',
    'schedules': 'scheduler',
    'memory': 'sessions',
    'audit': 'logs',
    'security': 'settings',
    'peers': 'settings',
    'migration': 'settings',
    'usage': 'analytics',
    'approval': 'approvals'
  };
  page.syncAgentChatsSectionForPage = function() {
    this.agentChatsSectionCollapsed = false;
  };
  page.toggleAgentChatsSection = function() {
    this.agentChatsSectionCollapsed = false;
  };
  var searchParams = new URLSearchParams(window.location.search || '');
  var embeddedDashboardMode = searchParams.get('embed') === '1';
  var embeddedPage = String(searchParams.get('page') || '').trim().toLowerCase();
  var pathnamePage = '';
  try {
    var pathname = String(window.location.pathname || '').trim();
    if (pathname.indexOf('/dashboard/') === 0) {
      pathnamePage = pathname.slice('/dashboard/'.length).split('/')[0].trim().toLowerCase();
    }
  } catch (_) {}
  if (embeddedDashboardMode && document && document.body && document.body.classList) {
    document.body.classList.add('dashboard-embedded-shell');
  }
  function handleHash() {
    var hash = window.location.hash.replace('#', '') || embeddedPage || pathnamePage || 'chat';
    if (pageRedirects[hash]) {
      hash = pageRedirects[hash];
      window.location.hash = hash;
    }
    if (validPages.indexOf(hash) >= 0) {
      self.page = hash;
      self.syncAgentChatsSectionForPage(hash);
      if (typeof self.syncPageHistory === 'function') self.syncPageHistory(hash);
      if (typeof self.notifyShellAppStore === 'function') self.notifyShellAppStore('route_changed');
    }
  }
  window.addEventListener('hashchange', handleHash);
  handleHash();

  document.addEventListener('keydown', function(e) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      self.navigate('agents');
    }
    if ((e.ctrlKey || e.metaKey) && e.key === 'n' && !e.shiftKey) {
      e.preventDefault();
      self.createSidebarAgentChat();
    }
    if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'F') {
      e.preventDefault();
      var keyStore = self.getAppStore();
      if (keyStore && typeof keyStore.toggleFocusMode === 'function') {
        keyStore.toggleFocusMode();
      }
    }
    if (e.key === 'Escape') {
      self.mobileMenuOpen = false;
    }
  });

  InfringAPI.onConnectionChange(function(state) {
    var connStore = self.getAppStore();
    if (connStore) connStore.connectionState = state;
    self.connectionState = state;
    self.queueConnectionIndicatorState(state);
  });

  if (!window.__infringToastCaptureInstalled) {
    window.addEventListener('infring:toast', function(ev) {
      var detail = (ev && ev.detail) ? ev.detail : {};
      var store = self.getAppStore();
      if (store && typeof store.addNotification === 'function') {
        store.addNotification(detail);
      }
    });
    window.__infringToastCaptureInstalled = true;
  }

  page.pollStatus();
  var initStore = page.getAppStore();
  if (initStore && typeof initStore.checkOnboarding === 'function') initStore.checkOnboarding();
  if (initStore && typeof initStore.checkAuth === 'function') initStore.checkAuth();
  if (!page._dashboardClockTimer) page._dashboardClockTimer = setInterval(function() { self.clockTick = Date.now(); }, 1000);
  if (!page._dashboardStatusTimer) page._dashboardStatusTimer = setInterval(function() {
    if (document && document.hidden) return;
    self.pollStatus();
  }, 10000);
  if (!page._dashboardVisibilityHandler && document) {
    page._dashboardVisibilityHandler = function() { if (!document.hidden) self.pollStatus(); };
    document.addEventListener('visibilitychange', page._dashboardVisibilityHandler);
  }
  window.addEventListener('resize', function() {
    self.scheduleSidebarScrollIndicators();
  });
  page.$nextTick(function() {
    self.scheduleSidebarScrollIndicators();
  });
}
