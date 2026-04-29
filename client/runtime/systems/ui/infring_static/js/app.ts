// Canonical Shell source-of-truth: assembled runtime app surface.
// Decomposition debt lives under ./app.ts.parts/** and must not count as additive production source.
// Shared rendering helpers split out to keep dashboard part files under size caps.


// Infring App — Alpine.js init, hash router, global store
'use strict';



// Temporary Alpine compatibility registration for the canonical Shell app-store bridge.
document.addEventListener('alpine:init', function() {
  // Restore saved API key on load
  var savedKey = localStorage.getItem('infring-api-key');
  if (savedKey) InfringAPI.setAuthToken(savedKey);

  var appStoreDefinition = {
    agents: [],
    connected: false,
    booting: true,
    agentsLoading: true,
    agentsHydrated: false,
    wsConnected: false,
    connectionState: 'connecting',
    statusFailureStreak: 0,
    lastError: '',
    bootStage: 'starting',
    statusDegraded: false,
    lastStatusLatencyMs: 0,
    lastStatusAt: '',
    version: (window.__INFRING_APP_VERSION || '0.0.0'),
    serverVersion: '',
    gitBranch: '',
    assistantName: 'Assistant',
    assistantAvatar: null,
    assistantAgentId: null,
    agentCount: 0,
    localMediaPreviewRoots: [],
    embedSandboxMode: 'scripts',
    allowExternalEmbedUrls: false,
    pendingAgent: null,
    pendingFreshAgentId: null,
    activeAgentId: (() => {
      try {
        var saved = localStorage.getItem('infring-last-active-agent-id');
        return saved ? String(saved) : null;
      } catch(_) {
        return null;
      }
    })(),
    focusMode: localStorage.getItem('infring-focus') === 'true',
    showOnboarding: false,
    showAuthPrompt: false,
    authMode: 'apikey',
    sessionUser: null,
    notifications: [],
    notificationsOpen: false,
    unreadNotifications: 0,
    notificationBubble: null,
    notificationBellPulse: false,
    _notificationBellPulseTimer: null,
    _notificationBellPulseSeq: 0,
    _notificationBubbleTimer: null,
    _notificationSeq: 0,
    taskbarRefreshTurns: 0,
    taskbarSearchOpen: false,
    taskbarSearchQuery: '',
    _taskbarSearchFocusTimer: 0,
    agentChatPreviews: {},
    agentLiveActivity: {},
    agentsEmptyResponseStreak: 0,
    agentsLastNonEmptyAt: 0,
    agentsFetchAttempts: 0,
    agentsLastError: '',
    agentTransientHoldMs: 20000,
    _refreshAgentsInFlight: null,
    _lastAgentsRefreshAt: 0,
    runtimeSync: null,
    lastErrorCode: '',
    _sessionActivityByAgent: {},
    _sessionActivityBootstrapped: false,
    _lastSessionActivityPollAt: 0,

    toggleFocusMode() {
      this.focusMode = !this.focusMode;
      localStorage.setItem('infring-focus', this.focusMode);
    },

    bumpTaskbarRefreshTurn() {
      var current = Number(this.taskbarRefreshTurns || 0);
      if (!Number.isFinite(current) || current < 0) current = 0;
      this.taskbarRefreshTurns = (current + 1) % 4096;
    },

    setActiveAgentId(agentId) {
      this.activeAgentId = agentId ? String(agentId) : null;
      if (this.activeAgentId && this.agentChatPreviews && this.agentChatPreviews[this.activeAgentId]) {
        this.agentChatPreviews[this.activeAgentId].unread_response = false;
      }
      try {
        if (this.activeAgentId) localStorage.setItem('infring-last-active-agent-id', this.activeAgentId);
        else localStorage.removeItem('infring-last-active-agent-id');
      } catch(_) {}
    },

    isArchivedLikeAgent(agent) {
      return infringIsArchivedLikeAgent(agent);
    },

    markAgentPreviewUnread(agentId, unread) {
      infringMarkAgentPreviewUnread(this, agentId, unread);
    },

    async refreshAgents(opts) {
      await infringRefreshAgents(this, opts);
    },

    async checkStatus() {
      await infringCheckStatus(this);
    },

    async pollSessionActivity(force) {
      await infringPollSessionActivity(this, force);
    },

    normalizeDashboardAssistantIdentity(payload) {
      return infringNormalizeDashboardAssistantIdentity(payload);
    },

    applyBootstrapRuntimeState(statusObj, versionObj) {
      infringApplyBootstrapRuntimeState(this, statusObj, versionObj);
    },

    focusTaskbarSearchInput() {
      infringFocusTaskbarSearchInput(this);
    },

    openTaskbarSearch() {
      infringOpenTaskbarSearch(this);
    },

    closeTaskbarSearch() {
      infringCloseTaskbarSearch(this);
    },

    toggleTaskbarSearch() {
      infringToggleTaskbarSearch(this);
    },

    async checkOnboarding() {
      await infringCheckOnboarding(this);
    },

    dismissOnboarding() {
      infringDismissOnboarding(this);
    },

    async checkAuth() {
      await infringCheckAuth(this);
    },

    submitApiKey(key) {
      infringSubmitApiKey(this, key);
    },

    async sessionLogin(username, password) {
      await infringSessionLogin(this, username, password);
    },

    async sessionLogout() {
      await infringSessionLogout(this);
    },

    normalizeNotificationType(rawType, message) {
      return infringNormalizeNotificationType(rawType, message);
    },

    addNotification(payload) {
      infringAddNotification(this, payload);
    },
    ringNotificationBell() {
      infringRingNotificationBell(this);
    },
    showNotificationBubble(note) {
      infringShowNotificationBubble(this, note);
    },

    toggleNotifications() {
      infringToggleNotifications(this);
    },

    markNotificationRead(id) {
      infringMarkNotificationRead(this, id);
    },

    markAllNotificationsRead() {
      infringMarkAllNotificationsRead(this);
    },

    dismissNotification(id) {
      infringDismissNotification(this, id);
    },

    clearNotifications() {
      infringClearNotifications(this);
    },

    reopenNotification(note) {
      infringReopenNotification(this, note);
    },

    dismissNotificationBubble() {
      infringDismissNotificationBubble(this);
    },

    saveAgentChatPreview(agentId, messages) {
      infringSaveAgentChatPreview(this, agentId, messages);
    },

    getAgentChatPreview(agentId) {
      return infringGetAgentChatPreview(this, agentId);
    },

    coerceAgentTimestamp(value) {
      return infringCoerceAgentTimestamp(value);
    },

    agentLastActivityTs(agent) {
      return infringAgentLastActivityTs(this, agent);
    },

    agentStatusFreshness(agent) {
      return infringAgentStatusFreshness(agent);
    },

    agentStatusState(agent) {
      return infringAgentStatusState(agent);
    },

    agentStatusLabel(agent) {
      return infringAgentStatusLabel(agent);
    },

    setAgentLiveActivity(agentId, state) {
      infringSetAgentLiveActivity(this, agentId, state);
    },

    clearAgentLiveActivity(agentId) {
      infringClearAgentLiveActivity(this, agentId);
    },

    isAgentLiveBusy(agent) {
      return infringIsAgentLiveBusy(this, agent);
    },

    formatNotificationTime(ts) {
      return infringFormatNotificationTime(ts);
    },

    clearApiKey() {
      infringClearApiKey();
    }
  };
  var appStoreBridge = infringShellAppStoreBridge();
  if (appStoreBridge && typeof appStoreBridge.registerAlpineStore === 'function') {
    appStoreBridge.registerAlpineStore(Alpine, 'app', appStoreDefinition);
  } else {
    var alpineRuntime = Alpine;
    if (alpineRuntime && typeof alpineRuntime.store === 'function') {
      alpineRuntime.store('app', appStoreDefinition);
      window.InfringApp = alpineRuntime.store('app');
    } else {
      window.InfringApp = appStoreDefinition;
    }
  }
});

// Main app component
function app() {
  return {
    page: 'agents',
    themeMode: localStorage.getItem('infring-theme-mode') || 'system',
    overlayGlassTemplate: 'simple-glass',
    uiBackgroundTemplate: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readDisplayBackground === 'function') return service.readDisplayBackground();
      var mode = 'light-wood';
      try {
        var rawDisplaySettings = localStorage.getItem('infring-display-settings') || '';
        var displaySettings = rawDisplaySettings ? JSON.parse(rawDisplaySettings) : {};
        mode = String(displaySettings && displaySettings.background ? displaySettings.background : mode);
        if (mode === 'sand') {
          mode = 'light-wood';
          displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
          displaySettings.background = mode;
          localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
        }
        if (!rawDisplaySettings || !displaySettings.background) {
          displaySettings = displaySettings && typeof displaySettings === 'object' ? displaySettings : {};
          displaySettings.background = mode;
          localStorage.setItem('infring-display-settings', JSON.stringify(displaySettings));
        }
      } catch (_) {}
      if (mode === 'unsplash-paper') mode = 'light-wood';
      if (mode !== 'default-grid' && mode !== 'light-wood' && mode !== 'sand') mode = 'light-wood';
      try {
        document.documentElement.setAttribute('data-ui-background-template', mode);
      } catch (_) {}
      return mode;
    })(),
    theme: (() => {
      var mode = localStorage.getItem('infring-theme-mode') || 'system';
      if (mode === 'system') return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      return mode;
    })(),
    sidebarCollapsed: localStorage.getItem('infring-sidebar') === 'collapsed',
    mobileMenuOpen: false,
    chatSidebarMode: 'default',
    chatSidebarQuery: '',
    chatSidebarSearchResults: [],
    chatSidebarSearchLoading: false,
    chatSidebarSearchError: '',
    chatSidebarSearchSeq: 0,
    _chatSidebarSearchTimer: 0,
    agentChatsSectionCollapsed: false,
    chatSidebarSortMode: (() => {
      try {
        var saved = String(localStorage.getItem('infring-chat-sidebar-sort-mode') || '').trim().toLowerCase();
        return saved === 'topology' ? 'topology' : 'age';
      } catch(_) {
        return 'age';
      }
    })(),
    chatSidebarTopologyOrder: (() => {
      try {
        var raw = localStorage.getItem('infring-chat-sidebar-topology-order');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return [];
        return parsed.map(function(id) { return String(id || '').trim(); }).filter(Boolean);
      } catch(_) {
        return [];
      }
    })(),
    chatSidebarDragAgentId: '',

    chatSidebarDropTargetId: '',
    chatSidebarDropAfter: false,
    chatSidebarVisibleBase: 7,
    chatSidebarVisibleStep: 5,
    chatSidebarVisibleCount: 7,
    dashboardPopup: {
      id: '',
      active: false,
      source: '',
      title: '',
      body: '',
      meta_origin: '',
      meta_time: '',
      unread: false,
      left: 0,
      top: 0,
      side: 'bottom',
      inline_away: 'right',
      block_away: 'bottom',
      compact: false
    },
    confirmArchiveAgentId: '',
    sidebarSpawningAgent: false,
    connected: false,
    wsConnected: false,
    connectionState: 'connecting',
    connectionIndicatorState: 'connecting',
    healthSummary: null,
    healthSummaryError: '',
    version: (window.__INFRING_APP_VERSION || '0.0.0'),
    agentCount: 0,
    bootSelectionApplied: false,
    clockTick: Date.now(),
    _dashboardClockTimer: 0,
    _dashboardStatusTimer: 0,
    _dashboardVisibilityHandler: null,
    _themeSwitchReset: 0,
    _lastConnectionIndicatorAt: 0,
    _connectionIndicatorTimer: null,
    _pendingConnectionIndicatorState: '',
    _healthSummaryLoadedAt: 0,
    _healthSummaryLoading: null,
    _healthSummaryLoadSeq: 0,
    _pollStatusInFlight: null,
    _pollStatusQueued: false,
    sidebarHasOverflowAbove: false,
    sidebarHasOverflowBelow: false,
    chatSidebarHasOverflowAbove: false,
    chatSidebarHasOverflowBelow: false,
    _sidebarScrollIndicatorRaf: 0,
    _chatSidebarFlipDurationMs: 240,
    _chatSidebarFlipRaf: 0,
    _chatSidebarLastSnapshot: null,
    _dragSurfaceLockTransformMs: 500,
    _dragSurfaceVisualStates: {},
    chatSidebarDragActive: false,
    chatSidebarDragLeft: 0,
    chatSidebarDragTop: 0,
    _chatSidebarDragRowsCache: null,
    _chatSidebarDragRenderMaxRows: 10,
    _chatSidebarDragRenderRowHeight: 56,
    chatSidebarPlacementX: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-x'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0;
    })(),
    chatSidebarPlacementY: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-y'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0.5;
    })(),
    chatSidebarPlacementTopPx: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-sidebar-placement-top-px'));
        if (Number.isFinite(raw)) return raw;
      } catch(_) {}
      return Number.NaN;
    })(),
    chatSidebarWallLock: (() => {
      try {
        var raw = String(
          localStorage.getItem('infring-chat-sidebar-wall-lock')
          || localStorage.getItem('infring-chat-sidebar-smash-wall')
          || ''
        ).trim().toLowerCase();
        if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
      } catch(_) {}
      return '';
    })(),
    _chatSidebarMoveDurationMs: 280,
    _chatSidebarPointerActive: false,
    _chatSidebarPointerMoved: false,
    _chatSidebarPointerStartX: 0,
    _chatSidebarPointerStartY: 0,
    _chatSidebarPointerOriginLeft: 0,
    _chatSidebarPointerOriginTop: 0,
    _chatSidebarPointerLastX: 0,
    _chatSidebarPointerLastY: 0,
    _chatSidebarPointerLastAt: 0,
    _chatSidebarPointerVelocity: 0,
    _chatSidebarPointerMoveHandler: null,
    _chatSidebarPointerUpHandler: null,
    _sidebarToggleSuppressUntil: 0,
    chatMapDragActive: false,
    chatMapDragLeft: 0,
    chatMapDragTop: 0,
    chatMapPlacementX: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-x'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 1;
    })(),
    chatMapPlacementY: (() => {
      try {
        var raw = Number(localStorage.getItem('infring-chat-map-placement-y'));
        if (Number.isFinite(raw)) return Math.max(0, Math.min(1, raw));
      } catch(_) {}
      return 0.38;
    })(),
    chatMapWallLock: (() => {
      try {
        var raw = String(
          localStorage.getItem('infring-chat-map-wall-lock')
          || localStorage.getItem('infring-chat-map-smash-wall')
          || ''
        ).trim().toLowerCase();
        if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
      } catch(_) {}
      return '';
    })(),
    _chatMapMoveDurationMs: 280,
    _chatMapPointerActive: false,
    _chatMapPointerMoved: false,
    _chatMapPointerStartX: 0,
    _chatMapPointerStartY: 0,
    _chatMapPointerOriginLeft: 0,
    _chatMapPointerOriginTop: 0,
    _chatMapPointerLastX: 0,
    _chatMapPointerLastY: 0,
    _chatMapPointerLastAt: 0,
    _chatMapPointerVelocity: 0,
    _chatMapPointerMoveHandler: null,
    _chatMapPointerUpHandler: null,
    bootSplashVisible: true,
    _bootSplashStartedAt: Date.now(),
    _bootSplashMinMs: 850,
    _bootSplashMaxMs: 5000,
    _bootSplashHideTimer: 0,
    _bootSplashMaxTimer: 0,
    bootProgressPercent: 6,
    bootProgressEvent: 'splash_visible',
    _bootProgressUpdatedAt: Date.now(),
    _taskbarRefreshOverlayTimer: 0,
    _taskbarRefreshReloadTimer: 0,
    taskbarHeroMenuOpen: false,
    taskbarTextMenuOpen: '',
    helpManualWindowOpen: false,
    reportIssueWindowOpen: false,
    reportIssueDraft: '',
    popupWindowPlacements: {
      manual: { left: null, top: null },
      report: { left: null, top: null }
    },
    popupWindowWallLocks: {
      manual: (() => {
        try {
          var raw = String(
            localStorage.getItem('infring-popup-window-manual-wall-lock')
            || localStorage.getItem('infring-popup-window-manual-smash-wall')
            || ''
          ).trim().toLowerCase();
          if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
        } catch(_) {}
        return '';
      })(),
      report: (() => {
        try {
          var raw = String(
            localStorage.getItem('infring-popup-window-report-wall-lock')
            || localStorage.getItem('infring-popup-window-report-smash-wall')
            || ''
          ).trim().toLowerCase();
          if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
        } catch(_) {}
        return '';
      })()
    },
    popupWindowDragActive: false,
    popupWindowDragKind: '',
    popupWindowDragLeft: 0,
    popupWindowDragTop: 0,
    popupWindowDragWallLock: '',
    _popupWindowMoveDurationMs: 260,
    _popupWindowPointerActive: false,
    _popupWindowPointerMoved: false,
    _popupWindowPointerStartX: 0,
    _popupWindowPointerStartY: 0,
    _popupWindowPointerOriginLeft: 0,
    _popupWindowPointerOriginTop: 0,
    _popupWindowPointerLastX: 0,
    _popupWindowPointerLastY: 0,
    _popupWindowPointerLastAt: 0,
    _popupWindowPointerVelocity: 0,
    _popupWindowPointerMoveHandler: null,
    _popupWindowPointerUpHandler: null,
    taskbarHeroActionPending: '',
    taskbarDockEdge: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().taskbar.edge;
      try {
        var raw = String(localStorage.getItem('infring-taskbar-dock-edge') || '').trim().toLowerCase();
        if (raw === 'bottom') return 'bottom';
      } catch(_) {}
      return 'top';
    })(),
    taskbarDockDragActive: false,
    taskbarDockDragY: 0,
    _taskbarDockPointerActive: false,
    _taskbarDockPointerMoved: false,
    _taskbarDockPointerStartX: 0,
    _taskbarDockPointerStartY: 0,
    _taskbarDockOriginY: 0,
    _taskbarDockPointerMoveHandler: null,
    _taskbarDockPointerUpHandler: null,
    taskbarReorderLeft: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readTaskbarOrder === 'function') return service.readTaskbarOrder('left');
      var defaults = ['nav_cluster'];
      try {
        var raw = localStorage.getItem('infring-taskbar-order-left');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i += 1) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j += 1) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    taskbarReorderRight: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readTaskbarOrder === 'function') return service.readTaskbarOrder('right');
      var defaults = ['connectivity', 'theme', 'notifications', 'search', 'auth'];
      try {
        var raw = localStorage.getItem('infring-taskbar-order-right');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i += 1) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j += 1) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    taskbarDragGroup: '',
    taskbarDragItem: '',
    taskbarDragStartOrder: [],
    _taskbarDragHoldTimer: 0,
    _taskbarDragHoldGroup: '',
    _taskbarDragHoldItem: '',
    _taskbarDragArmedGroup: '',
    _taskbarDragArmedItem: '',
    navBackStack: [],
    navForwardStack: [],
    _navCurrentPage: '',
    _navHistoryAction: '',
    _navHistoryCap: 48,

    appsIconBottomRowFill(index) {
      var idx = Number(index);
      if (!Number.isFinite(idx) || idx < 0) idx = 0;
      idx = Math.floor(idx);
      var colors = Array.isArray(this.appsIconBottomRowColors) ? this.appsIconBottomRowColors : [];
      return String(colors[idx] || '#22c55e');
    },

    chatSidebarFlipDurationMs() {
      return infringChatSidebarFlipDurationMs(this);
    },

    readChatSidebarSnapshot() {
      return infringReadChatSidebarSnapshot(this);
    },

    animateChatSidebarFromSnapshot(snapshot) {
      infringAnimateChatSidebarFromSnapshot(this, snapshot);
    },

    maybeAnimateChatSidebarRows() {
      infringMaybeAnimateChatSidebarRows(this);
    },

    cleanupBottomDockDragGhost() {
      infringCleanupBottomDockDragGhost(this);
    },

    setBottomDockGhostTarget(x, y) {
      infringSetBottomDockGhostTarget(this, x, y);
    },

    dragbarService() {
      var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
      return services && services.dragbar ? services.dragbar : null;
    },

    taskbarDockService() {
      var services = typeof window !== 'undefined' ? window.InfringSharedShellServices : null;
      return services && services.taskbarDock ? services.taskbarDock : null;
    },

    dragSurfaceMoveDurationMs(rawValue, fallbackMs) {
      return infringDragSurfaceMoveDurationMs(this, rawValue, fallbackMs);
    },

    readBottomDockScale(el) {
      return infringReadBottomDockScale(el);
    },

    bootProgressClamped(rawPercent) {
      return infringBootProgressClamped(rawPercent);
    },

    resetBootProgress() {
      infringResetBootProgress(this);
    },

    bootProgressFromBootStage(rawStage) {
      return infringBootProgressFromBootStage(rawStage);
    },

    setBootProgressPercent(rawPercent, opts) {
      infringSetBootProgressPercent(this, rawPercent, opts);
    },

    setBootProgressEvent(eventName, meta) {
      infringSetBootProgressEvent(this, eventName, meta);
    },
    normalizeConnectionIndicatorState(state) {
      return infringNormalizeConnectionIndicatorState(state);
    },

    queueConnectionIndicatorState(state) {
      infringQueueConnectionIndicatorState(this, state);
    },

    _computeScrollHintState(el) {
      return infringComputeScrollHintState(el);
    },

    bottomDockOrder: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readDockOrder === 'function') return service.readDockOrder();
      var defaults = ['chat', 'overview', 'agents', 'scheduler', 'skills', 'runtime', 'settings'];
      try {
        var raw = localStorage.getItem('infring-bottom-dock-order');
        var parsed = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(parsed)) return defaults.slice();
        var seen = {};
        var ordered = [];
        for (var i = 0; i < parsed.length; i++) {
          var id = String(parsed[i] || '').trim();
          if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
          seen[id] = true;
          ordered.push(id);
        }
        for (var j = 0; j < defaults.length; j++) {
          var fallbackId = defaults[j];
          if (seen[fallbackId]) continue;
          seen[fallbackId] = true;
          ordered.push(fallbackId);
        }
        return ordered;
      } catch(_) {
        return defaults.slice();
      }
    })(),
    bottomDockTileConfig: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.dockTileConfig === 'function') return service.dockTileConfig();
      return {
      chat: { icon: 'messages', tone: 'message', tooltip: 'Messages', label: 'Messages' },
      overview: { icon: 'home', tone: 'bright', tooltip: 'Home', label: 'Home' },
      agents: { icon: 'agents', tone: 'bright', tooltip: 'Agents', label: 'Agents' },
      scheduler: { icon: 'automation', tone: 'muted', tooltip: 'Automation', label: 'Automation', animation: ['automation-gears', 1200] },
      skills: { icon: 'apps', tone: 'default', tooltip: 'Apps', label: 'Apps' },
      runtime: { icon: 'system', tone: 'bright', tooltip: 'System', label: 'System', animation: ['system-terminal', 2000] },
      settings: { icon: 'settings', tone: 'muted', tooltip: 'Settings', label: 'Settings', animation: ['spin', 4000] }
      };
    })(),
    appsIconBottomRowColors: (() => {
      var palette = ['#14b8a6', '#06b6d4', '#38bdf8', '#22c55e', '#f59e0b', '#ef4444', '#a855f7', '#f43f5e', '#64748b'];
      var out = [];
      for (var i = 0; i < 3; i += 1) {
        out.push(palette[Math.floor(Math.random() * palette.length)]);
      }
      return out;
    })(),
    bottomDockDragId: '',
    bottomDockDragStartOrder: [],
    bottomDockDragCommitted: false,
    bottomDockHoverId: '',
    bottomDockHoverWeightById: {},
    bottomDockPointerX: 0,
    bottomDockPointerY: 0,
    bottomDockPreviewText: '',
    bottomDockPreviewMorphFromText: '',
    bottomDockPreviewHoverKey: '',
    bottomDockPreviewX: 0,
    bottomDockPreviewY: 0,
    bottomDockPreviewWidth: 0,
    bottomDockPreviewVisible: false,
    bottomDockPreviewLabelMorphing: false,
    bottomDockPreviewLabelFxReady: true,
    _bottomDockPreviewHideTimer: 0,
    _bottomDockPreviewReflowRaf: 0,
    _bottomDockPreviewReflowFrames: 0,
    _bottomDockPreviewWidthRaf: 0,
    _bottomDockPreviewLabelFxRaf: 0,
    _bottomDockPreviewLabelFxTimer: 0,
    _bottomDockPreviewLabelMorphTimer: 0,
    bottomDockClickAnimId: '',
    _bottomDockDragGhostEl: null,
    _bottomDockClickAnimTimer: 0,
    _bottomDockClickAnimDurationMs: 980,
    _bottomDockSuppressClickUntil: 0,
    _bottomDockPointerActive: false,
    _bottomDockPointerMoved: false,
    _bottomDockPointerCandidateId: '',
    _bottomDockPointerStartX: 0,
    _bottomDockPointerStartY: 0,
    _bottomDockPointerLastX: 0,
    _bottomDockPointerLastY: 0,
    _bottomDockPointerGrabOffsetX: 16,
    _bottomDockPointerGrabOffsetY: 16,
    _bottomDockDragGhostWidth: 32,
    _bottomDockDragGhostHeight: 32,
    _bottomDockPointerMoveHandler: null,
    _bottomDockPointerUpHandler: null,
    _bottomDockGhostTargetX: 0,
    _bottomDockGhostTargetY: 0,
    _bottomDockGhostCurrentX: 0,
    _bottomDockGhostCurrentY: 0,
    _bottomDockGhostRaf: 0,
    _bottomDockGhostCleanupTimer: 0,
    _bottomDockMoveDurationMs: 360,
    _bottomDockExpandedScale: 1.54,
    bottomDockRotationDeg: Number.NaN,
    _bottomDockRevealTargetDuringSettle: false,
    _bottomDockDragBoundaries: [],
    _bottomDockLastInsertionIndex: -1,
    _bottomDockReorderLockUntil: 0,
    bottomDockPlacementId: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().dock.placement;
      try {
        var raw = String(localStorage.getItem('infring-bottom-dock-placement') || '').trim().toLowerCase();
        var allowed = {
          left: true,
          center: true,
          right: true,
          'top-left': true,
          'top-center': true,
          'top-right': true,
          'left-top': true,
          'left-bottom': true,
          'right-top': true,
          'right-bottom': true
        };
        if (allowed[raw]) return raw;
        if (raw === 'left-center') return 'left-top';
        if (raw === 'right-center') return 'right-top';
      } catch(_) {}
      return 'center';
    })(),
    bottomDockSnapPoints: [
      { id: 'left', x: 0.16, y: 0.995, side: 'bottom' },
      { id: 'center', x: 0.50, y: 0.995, side: 'bottom' },
      { id: 'right', x: 0.84, y: 0.995, side: 'bottom' },
      { id: 'top-left', x: 0.16, y: 0.005, side: 'top' },
      { id: 'top-center', x: 0.50, y: 0.005, side: 'top' },
      { id: 'top-right', x: 0.84, y: 0.005, side: 'top' },
      { id: 'left-top', x: 0.005, y: (1 / 3), side: 'left' },
      { id: 'left-bottom', x: 0.005, y: (2 / 3), side: 'left' },
      { id: 'right-top', x: 0.995, y: (1 / 3), side: 'right' },
      { id: 'right-bottom', x: 0.995, y: (2 / 3), side: 'right' }
    ],
    bottomDockContainerDragActive: false,
    bottomDockContainerSettling: false,
    bottomDockContainerDragX: 0,
    bottomDockContainerDragY: 0,
    bottomDockContainerWallLock: (() => {
      var service = infringTaskbarDockService();
      if (service && typeof service.readLayoutConfig === 'function') return service.readLayoutConfig().dock.wallLock;
      try {
        var raw = String(
          localStorage.getItem('infring-bottom-dock-wall-lock')
          || localStorage.getItem('infring-bottom-dock-smash-wall')
          || ''
        ).trim().toLowerCase();
        if (raw === 'left' || raw === 'right' || raw === 'top' || raw === 'bottom') return raw;
      } catch(_) {}
      return '';
    })(),
    _bottomDockContainerDragWallLock: '',
    _bottomDockContainerPointerActive: false,
    _bottomDockContainerPointerMoved: false,
    _bottomDockContainerPointerStartX: 0,
    _bottomDockContainerPointerStartY: 0,
    _bottomDockContainerPointerLastX: 0,
    _bottomDockContainerPointerLastY: 0,
    _bottomDockContainerOriginX: 0,
    _bottomDockContainerOriginY: 0,
    _bottomDockContainerPointerMoveHandler: null,
    _bottomDockContainerPointerUpHandler: null,
    _bottomDockContainerSettleTimer: 0,

    bottomDockMoveDurationMs() {
      return infringBottomDockMoveDurationMs(this);
    },

    bottomDockExpandedScale() {
      return infringBottomDockExpandedScale(this);
    },

    bottomDockReadViewportSize() {
      return infringBottomDockReadViewportSize();
    },

    bottomDockReadBaseSize() {
      return infringBottomDockReadBaseSize();
    },

    bottomDockNormalizeSide(side) {
      return infringBottomDockNormalizeSide(side);
    },

    bottomDockIsVerticalSide(side) {
      return infringBottomDockIsVerticalSide(side);
    },

    bottomDockRotationDegForSide(side) {
      return infringBottomDockRotationDegForSide(side);
    },

    bottomDockIconRotationDegForSide(side) {
      return infringBottomDockIconRotationDegForSide(side);
    },

    bottomDockUpDegForSide(side) {
      return infringBottomDockUpDegForSide(side);
    },

    bottomDockOrientation(sideHint) {
      return infringBottomDockOrientation(this, sideHint);
    },

    bottomDockOppositeSide(sideHint) {
      return infringBottomDockOppositeSide(sideHint);
    },

    bottomDockWallSide() {
      return infringBottomDockWallSide(this);
    },

    bottomDockOpenSide() {
      return infringBottomDockOpenSide(this);
    },

    bottomDockRotationDegResolved(sideHint) {
      return infringBottomDockRotationDegResolved(this, sideHint);
    },

    bottomDockScreenDeltaToLocal(dx, dy, sideHint) {
      return infringBottomDockScreenDeltaToLocal(this, dx, dy, sideHint);
    },

    bottomDockCanonicalRotationCandidatesForSide(side) {
      return infringBottomDockCanonicalRotationCandidatesForSide(side);
    },

    bottomDockNormalizeRotationDeg(value) {
      return infringBottomDockNormalizeRotationDeg(value);
    },

    bottomDockResolveShortestRotationDeg(currentDeg, targetDeg) {
      return infringBottomDockResolveShortestRotationDeg(currentDeg, targetDeg);
    },

    bottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY) {
      return infringBottomDockPreferredRotationDirectionForAnchor(anchorX, anchorY);
    },

    bottomDockResolveDirectionalRotationDeg(currentDeg, targetDeg, direction) {
      return infringBottomDockResolveDirectionalRotationDeg(currentDeg, targetDeg, direction);
    },

    bottomDockResolveRotationForSide(side, anchorX, anchorY) {
      return infringBottomDockResolveRotationForSide(this, side, anchorX, anchorY);
    },

    bottomDockSnapDefinitions() {
      return infringBottomDockSnapDefinitions(this);
    },

    bottomDockSnapDefinitionById(id) {
      return infringBottomDockSnapDefinitionById(this, id);
    },

    bottomDockSideForSnapId(id) {
      return infringBottomDockSideForSnapId(this, id);
    },

    bottomDockActiveSnapId() {
      return infringBottomDockActiveSnapId(this);
    },

    bottomDockActiveSide() {
      return infringBottomDockActiveSide(this);
    },

    bottomDockWallLockNormalized() {
      return infringBottomDockWallLockNormalized(this);
    },

    bottomDockTaskbarContained() {
      return infringBottomDockTaskbarContained(this);
    },

    bottomDockHoverExpansionDisabled() {
      return infringBottomDockHoverExpansionDisabled(this);
    },

    bottomDockTaskbarContainedAnchorX(sideHint) {
      return infringBottomDockTaskbarContainedAnchorX(this, sideHint);
    },

    bottomDockTaskbarContainedMetrics() {
      return infringBottomDockTaskbarContainedMetrics(this);
    },

    bottomDockSetWallLock(wallRaw) {
      var wall = this.dragSurfaceNormalizeWall(wallRaw);
      this.bottomDockContainerWallLock = wall;
      try {
        if (wall) localStorage.setItem('infring-bottom-dock-wall-lock', wall);
        else localStorage.removeItem('infring-bottom-dock-wall-lock');
        localStorage.removeItem('infring-bottom-dock-smash-wall');
        infringUpdateShellLayoutConfig(function(config) { config.dock.wallLock = wall; });
      } catch(_) {}
      return wall;
    },

    bottomDockBoundsScaleForSide(sideHint) {
      return infringBottomDockBoundsScaleForSide(this, sideHint);
    },

    bottomDockVisualSizeForSide(sideHint) {
      return infringBottomDockVisualSizeForSide(this, sideHint);
    },

    bottomDockHardBoundsForSide(sideHint) {
      return infringBottomDockHardBoundsForSide(this, sideHint);
    },

    bottomDockTopLeftFromAnchor(anchorX, anchorY, sideHint) {
      return infringBottomDockTopLeftFromAnchor(this, anchorX, anchorY, sideHint);
    },

    bottomDockAnchorFromTopLeft(leftRaw, topRaw, sideHint) {
      return infringBottomDockAnchorFromTopLeft(this, leftRaw, topRaw, sideHint);
    },

    bottomDockLocalWallForRotation(wallRaw, rotationDegRaw) {
      return infringBottomDockLocalWallForRotation(this, wallRaw, rotationDegRaw);
    },

    bottomDockLockRadiusCssVars(wallRaw, rotationDegRaw) {
      return infringBottomDockLockRadiusCssVars(this, wallRaw, rotationDegRaw);
    },

    bottomDockClampDragAnchor(anchorX, anchorY) {
      return infringBottomDockClampDragAnchor(anchorX, anchorY);
    },

    bottomDockClampAnchor(anchorX, anchorY, sideOverride) {
      void sideOverride;
      return infringBottomDockClampDragAnchor(anchorX, anchorY);
    },

    bottomDockAnchorForSnapId(id) {
      return infringBottomDockAnchorForSnapId(this, id);
    },

    bottomDockNearestSnapId(anchorX, anchorY) {
      return infringBottomDockNearestSnapId(this, anchorX, anchorY);
    },

    persistBottomDockPlacement() {
      var key = String(this.bottomDockPlacementId || '').trim().toLowerCase();
      var snap = this.bottomDockSnapDefinitionById(key);
      this.bottomDockPlacementId = String(snap && snap.id || 'center');
      try {
        localStorage.setItem('infring-bottom-dock-placement', this.bottomDockPlacementId);
        infringUpdateShellLayoutConfig(function(config) { config.dock.placement = this.bottomDockPlacementId; }.bind(this));
      } catch(_) {}
    },

    syncDragWallCapHostNode(node, wallRaw) {
      infringSyncDragWallCapHostNode(this, node, wallRaw);
    },

    syncDragWallCaps() {
      infringSyncDragWallCaps(this);
    },

    bottomDockContainerStyle() {
      return infringBottomDockContainerStyle(this);
    },

    bindBottomDockContainerPointerListeners() {
      infringBindBottomDockContainerPointerListeners(this);
    },

    unbindBottomDockContainerPointerListeners() {
      infringUnbindBottomDockContainerPointerListeners(this);
    },

    startBottomDockContainerPointerDrag(ev) {
      infringStartBottomDockContainerPointerDrag(this, ev);
    },

    handleBottomDockContainerPointerMove(ev) {
      infringHandleBottomDockContainerPointerMove(this, ev);
    },

    endBottomDockContainerPointerDrag() {
      infringEndBottomDockContainerPointerDrag(this);
    },

    settleBottomDockDragGhost(dragId, done) {
      infringSettleBottomDockDragGhost(this, dragId, done);
    },

    taskbarDockEdgeNormalized(raw) {
      return infringTaskbarDockEdgeNormalized(this, raw);
    },

    taskbarPersistDockEdge() {
      infringTaskbarPersistDockEdge(this);
    },

    taskbarReadHeight() {
      return infringTaskbarReadHeight();
    },

    taskbarReadViewportHeight() {
      return infringTaskbarReadViewportHeight();
    },

    chatOverlayViewportWidth() {
      return infringChatOverlayViewportWidth();
    },

    taskbarAnchorForDockEdge(edgeRaw) {
      return infringTaskbarAnchorForDockEdge(this, edgeRaw);
    },

    taskbarClampDragY(yRaw) {
      return infringTaskbarClampDragY(this, yRaw);
    },

    taskbarNearestDockEdge(yRaw) {
      return infringTaskbarNearestDockEdge(this, yRaw);
    },

    taskbarContainerStyle() {
      return infringTaskbarContainerStyle(this);
    },

    shouldIgnoreTaskbarDockDragTarget(target) {
      return infringShouldIgnoreTaskbarDockDragTarget(this, target);
    },

    bindTaskbarDockPointerListeners() {
      infringBindTaskbarDockPointerListeners(this);
    },

    unbindTaskbarDockPointerListeners() {
      infringUnbindTaskbarDockPointerListeners(this);
    },

    startTaskbarDockPointerDrag(ev) {
      infringStartTaskbarDockPointerDrag(this, ev);
    },

    handleTaskbarDockPointerMove(ev) {
      infringHandleTaskbarDockPointerMove(this, ev);
    },

    endTaskbarDockPointerDrag() {
      infringEndTaskbarDockPointerDrag(this);
    },

    overlayWallGapPx() {
      return infringOverlayWallGapPx();
    },

    chatOverlayVerticalBounds() {
      return infringChatOverlayVerticalBounds(this);
    },

    dragSurfaceHardBounds(widthRaw, heightRaw, ignoreTaskbarBoundaryRaw) {
      return infringDragSurfaceHardBounds(this, widthRaw, heightRaw, ignoreTaskbarBoundaryRaw);
    },

    dragSurfaceSoftBounds(widthRaw, heightRaw) {
      return infringDragSurfaceSoftBounds(this, widthRaw, heightRaw);
    },

    dragSurfaceClampWithBounds(bounds, leftRaw, topRaw) {
      return infringDragSurfaceClampWithBounds(this, bounds, leftRaw, topRaw);
    },

    dragSurfaceNearestWall(bounds, leftRaw, topRaw) {
      return infringDragSurfaceNearestWall(this, bounds, leftRaw, topRaw);
    },

    dragSurfaceNormalizeWall(wallRaw) {
      return infringDragSurfaceNormalizeWall(this, wallRaw);
    },

    dragSurfaceApplyWallLock(bounds, leftRaw, topRaw, wallRaw) {
      return infringDragSurfaceApplyWallLock(this, bounds, leftRaw, topRaw, wallRaw);
    },

    dragSurfaceDistanceFromWall(bounds, leftRaw, topRaw, wallRaw) {
      return infringDragSurfaceDistanceFromWall(this, bounds, leftRaw, topRaw, wallRaw);
    },

    dragSurfaceWallLockOvershoot(bounds, leftRaw, topRaw, wallRaw) {
      return infringDragSurfaceWallLockOvershoot(this, bounds, leftRaw, topRaw, wallRaw);
    },

    dragSurfaceCenteredPoint(bounds) {
      return infringDragSurfaceCenteredPoint(this, bounds);
    },

    dragSurfaceWallLockContactThreshold() {
      return infringDragSurfaceWallLockContactThreshold(this);
    },
    dragSurfaceWallLockDistanceThreshold() {
      return infringDragSurfaceWallLockDistanceThreshold(this);
    },
    dragSurfaceWallUnlockDistanceThreshold() {
      return infringDragSurfaceWallUnlockDistanceThreshold(this);
    },
    dragSurfaceWallLockOvershootThreshold() {
      return infringDragSurfaceWallLockOvershootThreshold(this);
    },
    dragSurfaceResolveWallLock(bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw) {
      return infringDragSurfaceResolveWallLock(this, bounds, candidateLeft, candidateTop, nearest, motionDxRaw, motionDyRaw);
    },

    dragSurfaceRadiusByWall(wallRaw) {
      return infringDragSurfaceRadiusByWall(this, wallRaw);
    },

    dragSurfaceLockTransformTimeMs(rawValue) {
      return infringDragSurfaceLockTransformTimeMs(this, rawValue);
    },

    dragSurfaceLockBorderFadeDurationMs(transformMsRaw) {
      return infringDragSurfaceLockBorderFadeDurationMs(this, transformMsRaw);
    },

    dragSurfaceVisualStateStore() {
      return infringDragSurfaceVisualStateStore(this);
    },

    dragSurfaceLockVisualCssVars(surfaceKeyRaw, wallRaw, optionsRaw) {
      return infringDragSurfaceLockVisualCssVars(this, surfaceKeyRaw, wallRaw, optionsRaw);
    },

    dragSurfaceLockRadiusCssVars(wallRaw) {
      return infringDragSurfaceLockRadiusCssVars(this, wallRaw);
    },

    readChatMapElement() {
      return infringReadChatMapElement();
    },

    readChatMapHeight() {
      return infringReadChatMapHeight(this);
    },

    chatMapPlacementEnabled() {
      return infringChatMapPlacementEnabled(this);
    },

    chatMapClampTop(topRaw) {
      return infringChatMapClampTop(this, topRaw);
    },

    chatMapPersistPlacementFromTop(topRaw) {
      infringChatMapPersistPlacementFromTop(this, topRaw);
    },

    shouldIgnoreChatMapDragTarget(target) {
      return infringShouldIgnoreChatMapDragTarget(this, target);
    },

    bindChatMapPointerListeners() {
      infringBindChatMapPointerListeners(this);
    },

    unbindChatMapPointerListeners() {
      infringUnbindChatMapPointerListeners(this);
    },

    taskbarReorderDefaults(group) {
      return infringTaskbarReorderDefaults(this, group);
    },
    taskbarReorderStorageKey(group) {
      return infringTaskbarReorderStorageKey(this, group);
    },
    taskbarReorderOrderForGroup(group) {
      return infringTaskbarReorderOrderForGroup(this, group);
    },
    setTaskbarReorderOrderForGroup(group, nextOrder) {
      infringSetTaskbarReorderOrderForGroup(this, group, nextOrder);
    },
    normalizeTaskbarReorder(group, rawOrder) {
      return infringNormalizeTaskbarReorder(this, group, rawOrder);
    },
    persistTaskbarReorder(group) {
      infringPersistTaskbarReorder(this, group);
    },
    taskbarReorderOrderIndex(group, item) {
      return infringTaskbarReorderOrderIndex(this, group, item);
    },
    taskbarReorderItemStyle(group, item) {
      return infringTaskbarReorderItemStyle(this, group, item);
    },
    taskbarReorderItemRects(group) {
      return infringTaskbarReorderItemRects(group);
    },
    animateTaskbarReorderFromRects(group, beforeRects) {
      infringAnimateTaskbarReorderFromRects(group, beforeRects);
    },
    applyTaskbarReorder(group, dragItem, targetItem, preferAfter, animate) {
      return infringApplyTaskbarReorder(this, group, dragItem, targetItem, preferAfter, animate);
    },
    handleTaskbarReorderPointerDown(group, ev) {
      infringHandleTaskbarReorderPointerDown(this, group, ev);
    },
    cancelTaskbarDragHold() {
      infringCancelTaskbarDragHold(this);
    },
    forceTaskbarMoveDragEffect(ev) {
      infringForceTaskbarMoveDragEffect(ev);
    },
    setTaskbarDragBodyActive(active) {
      infringSetTaskbarDragBodyActive(active);
    },
    handleTaskbarReorderDragStart(group, ev) {
      infringHandleTaskbarReorderDragStart(this, group, ev);
    },
    handleTaskbarReorderDragMove(ev) {
      infringHandleTaskbarReorderDragMove(this, ev);
    },
    handleTaskbarReorderDragEnter(group, ev) {
      infringHandleTaskbarReorderDragEnter(this, group, ev);
    },
    handleTaskbarReorderDragOver(group, ev) {
      infringHandleTaskbarReorderDragOver(this, group, ev);
    },
    clearTaskbarReorderDraggingClass() {
      infringClearTaskbarReorderDraggingClass();
    },
    handleTaskbarReorderDrop(group, ev) {
      infringHandleTaskbarReorderDrop(this, group, ev);
    },
    handleTaskbarDragEnd() {
      infringHandleTaskbarDragEnd(this);
    },
    chatSidebarSnapDefinitions() {
      return infringChatSidebarSnapDefinitions();
    },
    chatSidebarSnapDefinitionById(id) {
      return infringChatSidebarSnapDefinitionById(this, id);
    },
    chatSidebarAnchorForSnapId(id) {
      return infringChatSidebarAnchorForSnapId(this, id);
    },
    chatSidebarNearestSnapId(leftRaw, topRaw) {
      return infringChatSidebarNearestSnapId(this, leftRaw, topRaw);
    },
    chatSidebarResolvedLeftFromRatio() {
      return infringChatSidebarResolvedLeftFromRatio(this);
    },
    chatSidebarResolvedTopFromRatio() {
      return infringChatSidebarResolvedTopFromRatio(this);
    },
    chatSidebarActiveSnapId() {
      return infringChatSidebarActiveSnapId(this);
    },
    chatSidebarPersistSnapId(id) {
      infringChatSidebarPersistSnapId(this, id);
    },
    readChatMapWidth() {
      return infringReadChatMapWidth(this);
    },
    chatMapSnapDefinitions() {
      return infringChatMapSnapDefinitions();
    },
    chatMapSnapDefinitionById(id) {
      return infringChatMapSnapDefinitionById(this, id);
    },
    chatMapAnchorForSnapId(id) {
      return infringChatMapAnchorForSnapId(this, id);
    },
    chatMapNearestSnapId(leftRaw, topRaw) {
      return infringChatMapNearestSnapId(this, leftRaw, topRaw);
    },
    chatMapResolvedLeftFromRatio() {
      return infringChatMapResolvedLeftFromRatio(this);
    },
    chatMapResolvedTopFromRatio() {
      return infringChatMapResolvedTopFromRatio(this);
    },
    chatMapActiveSnapId() {
      return infringChatMapActiveSnapId(this);
    },
    chatMapPersistSnapId(id) {
      infringChatMapPersistSnapId(this, id);
    },
    chatMapClampLeft(leftRaw) {
      return infringChatMapClampLeft(this, leftRaw);
    },
    chatMapHardBounds() {
      return infringChatMapHardBounds(this);
    },
    chatMapWallLockNormalized() {
      return infringChatMapWallLockNormalized(this);
    },
    chatMapSetWallLock(wallRaw) {
      return infringChatMapSetWallLock(this, wallRaw);
    },
    chatMapResolvedLeft() {
      return infringChatMapResolvedLeft(this);
    },
    chatMapResolvedTop() {
      return infringChatMapResolvedTop(this);
    },
    chatMapPersistPlacementFromLeft(leftRaw) {
      infringChatMapPersistPlacementFromLeft(this, leftRaw);
    },
    chatMapContainerStyle() {
      return infringChatMapContainerStyle(this);
    },
    startChatMapPointerDrag(ev) {
      infringStartChatMapPointerDrag(this, ev);
    },
    handleChatMapPointerMove(ev) {
      infringHandleChatMapPointerMove(this, ev);
    },
    endChatMapPointerDrag() {
      infringEndChatMapPointerDrag(this);
    },

    readChatSidebarElement() {
      return infringReadChatSidebarElement();
    },
    readChatSidebarHeight() {
      return infringReadChatSidebarHeight(this);
    },
    readChatSidebarWidth() {
      return infringReadChatSidebarWidth(this);
    },
    readChatSidebarPulltabWidth() {
      return infringReadChatSidebarPulltabWidth();
    },
    chatSidebarClampLeft(leftRaw) {
      return infringChatSidebarClampLeft(this, leftRaw);
    },
    chatSidebarHardBounds() {
      return infringChatSidebarHardBounds(this);
    },
    chatSidebarWallLockNormalized() {
      return infringChatSidebarWallLockNormalized(this);
    },
    chatSidebarSetWallLock(wallRaw) {
      return infringChatSidebarSetWallLock(this, wallRaw);
    },
    chatSidebarResolvedLeft() {
      return infringChatSidebarResolvedLeft(this);
    },
    chatSidebarPersistPlacementFromLeft(leftRaw) {
      infringChatSidebarPersistPlacementFromLeft(this, leftRaw);
    },
    chatSidebarClampTop(topRaw) {
      return infringChatSidebarClampTop(this, topRaw);
    },
    chatSidebarResolvedTop() {
      return infringChatSidebarResolvedTop(this);
    },
    chatSidebarPersistPlacementFromTop(topRaw) {
      infringChatSidebarPersistPlacementFromTop(this, topRaw);
    },
    chatSidebarContainerStyle() {
      return infringChatSidebarContainerStyle(this);
    },
    chatSidebarNavShellStyle() {
      return infringChatSidebarNavShellStyle(this);
    },
    chatSidebarNavStyle() {
      return infringChatSidebarNavStyle(this);
    },
    chatSidebarPulltabStyle() {
      return infringChatSidebarPulltabStyle(this);
    },
    shouldIgnoreChatSidebarDragTarget(target) {
      return infringShouldIgnoreChatSidebarDragTarget(this, target);
    },

    bindChatSidebarPointerListeners() {
      infringBindChatSidebarPointerListeners(this);
    },

    unbindChatSidebarPointerListeners() {
      infringUnbindChatSidebarPointerListeners(this);
    },

    startChatSidebarPointerDrag(ev) {
      infringStartChatSidebarPointerDrag(this, ev);
    },

    handleChatSidebarPointerMove(ev) {
      infringHandleChatSidebarPointerMove(this, ev);
    },

    endChatSidebarPointerDrag() {
      infringEndChatSidebarPointerDrag(this);
    },

    shouldSuppressSidebarToggle() {
      return infringShouldSuppressSidebarToggle(this);
    },

    popupWindowStorageKey(kind, axis) {
      return infringPopupWindowStorageKey(kind, axis);
    },
    popupWindowWallLockStorageKey(kind) {
      return infringPopupWindowWallLockStorageKey(kind);
    },
    popupWindowWallLock(kind) {
      return infringPopupWindowWallLock(kind);
    },
    popupWindowSetWallLock(kind, wallRaw) {
      return infringPopupWindowSetWallLock(this, kind, wallRaw);
    },

    popupWindowOpenState(kind) {
      return infringPopupWindowOpenState(this, kind);
    },

    popupWindowSetOpenState(kind, open) {
      infringPopupWindowSetOpenState(this, kind, open);
    },

    readPopupWindowElement(kind) {
      return infringReadPopupWindowElement(kind);
    },

    popupWindowDefaultSize(kind) {
      return infringPopupWindowDefaultSize(kind);
    },

    readPopupWindowSize(kind) {
      return infringReadPopupWindowSize(this, kind);
    },

    popupWindowBounds(kind, widthRaw, heightRaw) {
      return infringPopupWindowBounds(this, kind, widthRaw, heightRaw);
    },

    popupWindowClampPlacement(kind, leftRaw, topRaw) {
      return infringPopupWindowClampPlacement(this, kind, leftRaw, topRaw);
    },
    popupWindowHardBounds(kind) {
      return infringPopupWindowHardBounds(this, kind);
    },

    popupWindowEnsurePlacement(kind, forceCenter) {
      return infringPopupWindowEnsurePlacement(this, kind, forceCenter);
    },

    popupWindowPersistPlacement(kind, leftRaw, topRaw) {
      infringPopupWindowPersistPlacement(this, kind, leftRaw, topRaw);
    },

    popupWindowResolvedLeft(kind) {
      return infringPopupWindowResolvedLeft(this, kind);
    },

    popupWindowResolvedTop(kind) {
      return infringPopupWindowResolvedTop(this, kind);
    },

    popupWindowStyle(kind) {
      return infringPopupWindowStyle(this, kind);
    },

    openPopupWindow(kind) {
      infringOpenPopupWindow(this, kind);
    },

    closePopupWindow(kind) {
      infringClosePopupWindow(this, kind);
    },

    bindPopupWindowPointerListeners() {
      infringBindPopupWindowPointerListeners(this);
    },

    unbindPopupWindowPointerListeners() {
      infringUnbindPopupWindowPointerListeners(this);
    },

    startPopupWindowPointerDrag(kind, ev) {
      infringStartPopupWindowPointerDrag(this, kind, ev);
    },

    handlePopupWindowPointerMove(ev) {
      infringHandlePopupWindowPointerMove(this, ev);
    },

    endPopupWindowPointerDrag() {
      infringEndPopupWindowPointerDrag(this);
    },

    bottomDockDefaultOrder() {
      return infringBottomDockDefaultOrder(this);
    },

    bottomDockTileConfigById(id) {
      return infringBottomDockTileConfigById(this, id);
    },

    bottomDockTileData(id, field, fallback) {
      return infringBottomDockTileData(this, id, field, fallback);
    },

    bottomDockTileAnimationName(id) {
      return infringBottomDockTileAnimationName(this, id);
    },

    bottomDockTileAnimationDurationAttr(id) {
      return infringBottomDockTileAnimationDurationAttr(this, id);
    },

    bottomDockSlotStyle(id) {
      return infringBottomDockSlotStyle(this, id);
    },

    bottomDockTileStyle(id) {
      return infringBottomDockTileStyle(this, id);
    },

    normalizeBottomDockOrder(rawOrder) {
      var service = this.taskbarDockService();
      if (service && typeof service.normalizeOrder === 'function') return service.normalizeOrder(rawOrder, this.bottomDockDefaultOrder());
      var defaults = this.bottomDockDefaultOrder();
      var source = Array.isArray(rawOrder) ? rawOrder : [];
      var seen = {};
      var ordered = [];
      for (var i = 0; i < source.length; i++) {
        var id = String(source[i] || '').trim();
        if (!id || seen[id] || defaults.indexOf(id) < 0) continue;
        seen[id] = true;
        ordered.push(id);
      }
      for (var j = 0; j < defaults.length; j++) {
        var fallbackId = defaults[j];
        if (seen[fallbackId]) continue;
        seen[fallbackId] = true;
        ordered.push(fallbackId);
      }
      return ordered;
    },

    persistBottomDockOrder() {
      this.bottomDockOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      try {
        var service = this.taskbarDockService();
        if (service && typeof service.persistDockOrder === 'function') this.bottomDockOrder = service.persistDockOrder(this.bottomDockOrder, this.bottomDockTileConfig);
        else localStorage.setItem('infring-bottom-dock-order', JSON.stringify(this.bottomDockOrder));
      } catch(_) {}
      infringUpdateShellLayoutConfig(function(config) {
        config.dock.order = this.bottomDockOrder.slice();
      }.bind(this));
    },

    bottomDockOrderIndex(id) {
      var key = String(id || '').trim();
      if (!key) return 999;
      var service = this.taskbarDockService();
      if (service && typeof service.orderIndex === 'function') {
        return service.orderIndex(key, this.bottomDockOrder, this.bottomDockDefaultOrder());
      }
      var order = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var idx = order.indexOf(key);
      if (idx >= 0) return idx;
      var fallback = this.bottomDockDefaultOrder().indexOf(key);
      return fallback >= 0 ? fallback : 999;
    },

    bottomDockAxisBasis(sideHint) {
      var rotationDeg = this.bottomDockRotationDegResolved(sideHint);
      var theta = (Number(rotationDeg || 0) * Math.PI) / 180;
      var ux = Math.cos(theta);
      var uy = Math.sin(theta);
      if (Math.abs(ux) < 0.0001) ux = 0;
      if (Math.abs(uy) < 0.0001) uy = 0;
      return { ux: ux, uy: uy, vx: -uy, vy: ux };
    },

    bottomDockProjectPointToAxis(x, y, basis) {
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var ux = Number(axis.ux || 0);
      var uy = Number(axis.uy || 0);
      var vx = Number(axis.vx || (-uy));
      var vy = Number(axis.vy || ux);
      var px = Number(x || 0);
      var py = Number(y || 0);
      return {
        primary: (px * ux) + (py * uy),
        secondary: (px * vx) + (py * vy)
      };
    },

    bottomDockAxisHalfExtent(width, height, basis) {
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var w = Number(width || 0);
      var h = Number(height || 0);
      if (!Number.isFinite(w) || w < 0) w = 0;
      if (!Number.isFinite(h) || h < 0) h = 0;
      var ux = Math.abs(Number(axis.ux || 0));
      var uy = Math.abs(Number(axis.uy || 0));
      var vx = Math.abs(Number(axis.vx || 0));
      var vy = Math.abs(Number(axis.vy || 0));
      return {
        primary: ((ux * w) + (uy * h)) / 2,
        secondary: ((vx * w) + (vy * h)) / 2
      };
    },

    bottomDockProjectedRectBounds(rect, basis) {
      if (!rect) return null;
      var axis = basis && typeof basis === 'object'
        ? basis
        : this.bottomDockAxisBasis();
      var left = Number(rect.left || 0);
      var top = Number(rect.top || 0);
      var right = Number(rect.right || left);
      var bottom = Number(rect.bottom || top);
      var p1 = this.bottomDockProjectPointToAxis(left, top, axis);
      var p2 = this.bottomDockProjectPointToAxis(right, top, axis);
      var p3 = this.bottomDockProjectPointToAxis(left, bottom, axis);
      var p4 = this.bottomDockProjectPointToAxis(right, bottom, axis);
      var primaryMin = Math.min(p1.primary, p2.primary, p3.primary, p4.primary);
      var primaryMax = Math.max(p1.primary, p2.primary, p3.primary, p4.primary);
      var secondaryMin = Math.min(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
      var secondaryMax = Math.max(p1.secondary, p2.secondary, p3.secondary, p4.secondary);
      return {
        primaryMin: primaryMin,
        primaryMax: primaryMax,
        secondaryMin: secondaryMin,
        secondaryMax: secondaryMax
      };
    },

    bottomDockButtonRects() {
      var out = {};
      var root = document.querySelector('.bottom-dock');
      if (!root) return out;
      var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
      for (var i = 0; i < nodes.length; i++) {
        var node = nodes[i];
        if (!node) continue;
        var id = String(node.getAttribute('data-dock-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var width = Number(rect.width || 0);
        var height = Number(rect.height || 0);
        var left = Number(rect.left || 0);
        var top = Number(rect.top || 0);
        out[id] = {
          left: left,
          top: top,
          width: width,
          height: height,
          cx: left + (width / 2),
          cy: top + (height / 2)
        };
      }
      return out;
    },

    animateBottomDockFromRects(beforeRects) {
      if (!beforeRects || typeof beforeRects !== 'object') return;
      if (typeof requestAnimationFrame !== 'function') return;
      var durationMs = this.bottomDockMoveDurationMs();
      var self = this;
      requestAnimationFrame(function() {
        var root = document.querySelector('.bottom-dock');
        if (!root) return;
        var rootScale = self.readBottomDockScale(root);
        if (!Number.isFinite(rootScale) || rootScale <= 0.01) rootScale = 1;
        var side = self.bottomDockActiveSide();
        var nodes = root.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i++) {
          var node = nodes[i];
          if (!node || node.classList.contains('dragging')) continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || !Object.prototype.hasOwnProperty.call(beforeRects, id)) continue;
          var from = beforeRects[id] || {};
          var rect = node.getBoundingClientRect();
          var fromCx = Number(from.cx);
          var fromCy = Number(from.cy);
          if (!Number.isFinite(fromCx)) fromCx = Number(from.left || 0) + (Number(from.width || 0) / 2);
          if (!Number.isFinite(fromCy)) fromCy = Number(from.top || 0) + (Number(from.height || 0) / 2);
          var toCx = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
          var toCy = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
          var screenDx = Number(fromCx || 0) - Number(toCx || 0);
          var screenDy = Number(fromCy || 0) - Number(toCy || 0);
          if (Math.abs(screenDx) < 0.5 && Math.abs(screenDy) < 0.5) continue;
          var localDelta = self.bottomDockScreenDeltaToLocal(screenDx, screenDy, side);
          var tx = Number(localDelta.x || 0) / rootScale;
          var ty = Number(localDelta.y || 0) / rootScale;
          if (Math.abs(tx) < 0.25 && Math.abs(ty) < 0.25) continue;
          node.style.setProperty('--dock-reorder-transition', '0ms');
          node.style.setProperty('--dock-reorder-translate-x', Math.round(tx) + 'px');
          node.style.setProperty('--dock-reorder-translate-y', Math.round(ty) + 'px');
          void node.offsetHeight;
          node.style.setProperty('--dock-reorder-transition', Math.max(0, Math.round(durationMs)) + 'ms');
          node.style.setProperty('--dock-reorder-translate-x', '0px');
          node.style.setProperty('--dock-reorder-translate-y', '0px');
          (function(el) {
            window.setTimeout(function() {
              if (
                !el.classList.contains('dragging') &&
                !el.classList.contains('hovered') &&
                !el.classList.contains('neighbor-hover') &&
                !el.classList.contains('second-neighbor-hover')
              ) {
                el.style.removeProperty('--dock-reorder-translate-x');
                el.style.removeProperty('--dock-reorder-translate-y');
              }
              el.style.removeProperty('--dock-reorder-transition');
            }, durationMs + 30);
          })(node);
        }
      });
    },

    setBottomDockHover(id, ev) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      this.bottomDockHoverId = key;
      if (ev) {
        var evX = Number(ev.clientX || 0);
        var evY = Number(ev.clientY || 0);
        if (Number.isFinite(evX) && evX > 0) this.bottomDockPointerX = evX;
        if (Number.isFinite(evY) && evY > 0) this.bottomDockPointerY = evY;
      }
      if (this._bottomDockPreviewHideTimer) {
        try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        this._bottomDockPreviewHideTimer = 0;
      }
      if (!Number.isFinite(this.bottomDockPointerX) || this.bottomDockPointerX <= 0) {
        try {
          var slot = document.querySelector('.bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
          if (slot && typeof slot.getBoundingClientRect === 'function') {
            var slotRect = slot.getBoundingClientRect();
            this.bottomDockPointerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
            this.bottomDockPointerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
          }
        } catch(_) {}
      }
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    clearBottomDockHover(id) {
      if (id) return;
      this.bottomDockHoverId = '';
      if (!this.bottomDockHoverId) {
        this.bottomDockHoverWeightById = {};
        this.bottomDockPointerX = 0;
        this.bottomDockPointerY = 0;
        this.cancelBottomDockPreviewReflow();
        var self = this;
        if (this._bottomDockPreviewHideTimer) {
          try { clearTimeout(this._bottomDockPreviewHideTimer); } catch(_) {}
        }
        this._bottomDockPreviewHideTimer = window.setTimeout(function() {
          self._bottomDockPreviewHideTimer = 0;
          if (!String(self.bottomDockHoverId || '').trim()) {
            self.bottomDockPreviewVisible = false;
            self.bottomDockPreviewText = '';
            self.bottomDockPreviewMorphFromText = '';
            self.bottomDockPreviewLabelMorphing = false;
            self.bottomDockPreviewWidth = 0;
          }
        }, 40);
        return;
      }
      this.syncBottomDockPreview();
    },

    readBottomDockSlotCenters() {
      var out = [];
      if (typeof document === 'undefined') return out;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.querySelectorAll !== 'function') return out;
      var nodes = root.querySelectorAll('.dock-tile-slot[data-dock-slot-id]');
      for (var i = 0; i < nodes.length; i += 1) {
        var node = nodes[i];
        if (!node || typeof node.getAttribute !== 'function' || typeof node.getBoundingClientRect !== 'function') continue;
        var id = String(node.getAttribute('data-dock-slot-id') || '').trim();
        if (!id) continue;
        var rect = node.getBoundingClientRect();
        var centerX = Number(rect.left || 0) + (Number(rect.width || 0) / 2);
        var centerY = Number(rect.top || 0) + (Number(rect.height || 0) / 2);
        if (!Number.isFinite(centerX) || !Number.isFinite(centerY)) continue;
        out.push({ id: id, centerX: centerX, centerY: centerY });
      }
      return out;
    },

    bottomDockWeightForDistance(distancePx) {
      var d = Math.abs(Number(distancePx || 0));
      if (!Number.isFinite(d)) return 0;
      var sigma = 52;
      var exponent = -((d * d) / (2 * sigma * sigma));
      var weight = Math.exp(exponent);
      if (!Number.isFinite(weight) || weight < 0.008) return 0;
      if (weight > 1) return 1;
      return weight;
    },

    refreshBottomDockHoverWeights() {
      var side = this.bottomDockActiveSide();
      var vertical = this.bottomDockIsVerticalSide(side);
      var primaryPointer = vertical
        ? Number(this.bottomDockPointerY || 0)
        : Number(this.bottomDockPointerX || 0);
      if (!Number.isFinite(primaryPointer) || primaryPointer <= 0) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var centers = this.readBottomDockSlotCenters();
      if (!centers.length) {
        this.bottomDockHoverWeightById = {};
        return;
      }
      var nearestId = '';
      var nearestDistance = Number.POSITIVE_INFINITY;
      var weights = {};
      for (var i = 0; i < centers.length; i += 1) {
        var item = centers[i];
        if (!item || !item.id) continue;
        var anchor = vertical ? Number(item.centerY || 0) : Number(item.centerX || 0);
        var dist = Math.abs(primaryPointer - anchor);
        if (!Number.isFinite(dist)) continue;
        if (dist < nearestDistance) {
          nearestDistance = dist;
          nearestId = item.id;
        }
        weights[item.id] = this.bottomDockWeightForDistance(dist);
      }
      this.bottomDockHoverWeightById = weights;
      if (nearestId) this.bottomDockHoverId = nearestId;
    },

    updateBottomDockPointer(ev) {
      if (!ev) return;
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(ev.clientX || 0);
      var y = Number(ev.clientY || 0);
      if (!Number.isFinite(x) || x <= 0) return;
      this.bottomDockPointerX = x;
      if (Number.isFinite(y) && y > 0) this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
    },

    reviveBottomDockHoverFromPoint(clientX, clientY) {
      if (String(this.bottomDockDragId || '').trim()) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var x = Number(clientX || 0);
      var y = Number(clientY || 0);
      if (!Number.isFinite(x) || !Number.isFinite(y) || x <= 0 || y <= 0) return;
      var root = document.querySelector('.bottom-dock');
      if (!root || typeof root.getBoundingClientRect !== 'function') return;
      var rect = root.getBoundingClientRect();
      var withinX = x >= (Number(rect.left || 0) - 16) && x <= (Number(rect.right || 0) + 16);
      var withinY = y >= (Number(rect.top || 0) - 18) && y <= (Number(rect.bottom || 0) + 18);
      if (!withinX || !withinY) return;
      this.bottomDockPointerX = x;
      this.bottomDockPointerY = y;
      this.refreshBottomDockHoverWeights();
      this.syncBottomDockPreview();
      this.scheduleBottomDockPreviewReflow();
    },

    scheduleBottomDockPreviewReflow() {
      this.cancelBottomDockPreviewReflow();
      var self = this;
      this._bottomDockPreviewReflowFrames = 10;
      var step = function() {
        if (!String(self.bottomDockHoverId || '').trim()) {
          self._bottomDockPreviewReflowRaf = 0;
          self._bottomDockPreviewReflowFrames = 0;
          return;
        }
        self.syncBottomDockPreview();
        self._bottomDockPreviewReflowFrames = Math.max(0, Number(self._bottomDockPreviewReflowFrames || 0) - 1);
        if (self._bottomDockPreviewReflowFrames <= 0) {
          self._bottomDockPreviewReflowRaf = 0;
          return;
        }
        self._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
      };
      this._bottomDockPreviewReflowRaf = requestAnimationFrame(step);
    },

    cancelBottomDockPreviewReflow() {
      if (this._bottomDockPreviewReflowRaf && typeof cancelAnimationFrame === 'function') {
        try { cancelAnimationFrame(this._bottomDockPreviewReflowRaf); } catch(_) {}
      }
      this._bottomDockPreviewReflowRaf = 0;
      this._bottomDockPreviewReflowFrames = 0;
    },

    syncBottomDockPreview() {
      var key = String(this.bottomDockHoverId || '').trim();
      if (!key) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var text = this.bottomDockTileData(key, 'tooltip', '');
      var label = String(text || '').trim();
      if (!label) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var root = document.querySelector('.bottom-dock');
      var slot = document.querySelector('.bottom-dock .dock-tile-slot[data-dock-slot-id="' + key + '"]');
      if (!root || !slot) {
        this.bottomDockPreviewVisible = false;
        this.bottomDockPreviewText = '';
        this.bottomDockPreviewMorphFromText = '';
        this.bottomDockPreviewHoverKey = '';
        this.bottomDockPreviewLabelMorphing = false;
        this.bottomDockPreviewWidth = 0;
        this.bottomDockPreviewLabelFxReady = true;
        return;
      }
      var centerX = 0;
      var centerY = 0;
      var anchorY = 0;
      var anchorX = 0;
      var wallSide = this.bottomDockWallSide();
      var openSide = this.bottomDockOpenSide();
      var vertical = this.bottomDockIsVerticalSide(wallSide);
      var dockRect = (typeof root.getBoundingClientRect === 'function')
        ? root.getBoundingClientRect()
        : null;
      if (typeof slot.getBoundingClientRect === 'function' && dockRect) {
        var slotRect = slot.getBoundingClientRect();
        centerX = Number(slotRect.left || 0) + (Number(slotRect.width || 0) / 2);
        centerY = Number(slotRect.top || 0) + (Number(slotRect.height || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(dockRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(dockRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(dockRect.left || 0) - 8;
        } else {
          anchorX = Number(dockRect.right || 0) + 8;
        }
      } else if (slot.offsetParent === root) {
        var rootRect = root.getBoundingClientRect();
        centerX = Number(rootRect.left || 0) + Number(slot.offsetLeft || 0) + (Number(slot.offsetWidth || 0) / 2);
        centerY = Number(rootRect.top || 0) + Number(slot.offsetTop || 0) + (Number(slot.offsetHeight || 0) / 2);
        if (openSide === 'top') {
          anchorY = Number(rootRect.top || 0) - 8;
        } else if (openSide === 'bottom') {
          anchorY = Number(rootRect.bottom || 0) + 8;
        } else if (openSide === 'left') {
          anchorX = Number(rootRect.left || 0) - 8;
        } else {
          anchorX = Number(rootRect.right || 0) + 8;
        }
      }
      var pointerX = Number(this.bottomDockPointerX || 0);
      var pointerY = Number(this.bottomDockPointerY || 0);
      if (!vertical && Number.isFinite(pointerX) && pointerX > 0) {
        if (dockRect) {
          var minX = Number(dockRect.left || 0);
          var maxX = Number(dockRect.right || 0);
          if (Number.isFinite(minX) && Number.isFinite(maxX) && maxX > minX) {
            pointerX = Math.max(minX, Math.min(maxX, pointerX));
          }
        }
        centerX = pointerX;
      }
      if (vertical && Number.isFinite(pointerY) && pointerY > 0) {
        if (dockRect) {
          var minY = Number(dockRect.top || 0);
          var maxY = Number(dockRect.bottom || 0);
          if (Number.isFinite(minY) && Number.isFinite(maxY) && maxY > minY) {
            pointerY = Math.max(minY, Math.min(maxY, pointerY));
          }
        }
        centerY = pointerY;
      }
      if (!Number.isFinite(centerX)) centerX = 0;
      if (!Number.isFinite(centerY)) centerY = 0;
      if (!Number.isFinite(anchorX)) anchorX = 0;
      if (!Number.isFinite(anchorY)) anchorY = 0;
      this.bottomDockPreviewX = vertical ? anchorX : centerX;
      this.bottomDockPreviewY = vertical ? centerY : anchorY;
      this.bottomDockPreviewHoverKey = key;
      this.bottomDockPreviewVisible = true;
      this.bottomDockPreviewText = label;
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.bottomDockPreviewLabelFxReady = true;
    },

    bindBottomDockPointerListeners() {
      if (this._bottomDockPointerMoveHandler || this._bottomDockPointerUpHandler) return;
      var self = this;
      this._bottomDockPointerMoveHandler = function(ev) { self.handleBottomDockPointerMove(ev); };
      this._bottomDockPointerUpHandler = function(ev) { self.endBottomDockPointerDrag(ev); };
      window.addEventListener('pointermove', this._bottomDockPointerMoveHandler, true);
      window.addEventListener('pointerup', this._bottomDockPointerUpHandler, true);
      window.addEventListener('pointercancel', this._bottomDockPointerUpHandler, true);
      window.addEventListener('mousemove', this._bottomDockPointerMoveHandler, true);
      window.addEventListener('mouseup', this._bottomDockPointerUpHandler, true);
    },

    unbindBottomDockPointerListeners() {
      if (this._bottomDockPointerMoveHandler) {
        try { window.removeEventListener('pointermove', this._bottomDockPointerMoveHandler, true); } catch(_) {}
        try { window.removeEventListener('mousemove', this._bottomDockPointerMoveHandler, true); } catch(_) {}
      }
      if (this._bottomDockPointerUpHandler) {
        try { window.removeEventListener('pointerup', this._bottomDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('pointercancel', this._bottomDockPointerUpHandler, true); } catch(_) {}
        try { window.removeEventListener('mouseup', this._bottomDockPointerUpHandler, true); } catch(_) {}
      }
      this._bottomDockPointerMoveHandler = null;
      this._bottomDockPointerUpHandler = null;
    },

    startBottomDockPointerDrag(id, ev) {
      if (!ev || Number(ev.button) !== 0) return;
      if (this.bottomDockContainerDragActive || this._bottomDockContainerPointerActive) return;
      var key = String(id || '').trim();
      if (!key) return;
      var hostEl = ev && ev.currentTarget ? ev.currentTarget : null;
      if (hostEl && typeof hostEl.getBoundingClientRect === 'function') {
        try {
          var rect = hostEl.getBoundingClientRect();
          var width = Number(rect.width || 32);
          var height = Number(rect.height || 32);
          var baseWidth = Number(hostEl && hostEl.offsetWidth ? hostEl.offsetWidth : width || 32);
          var baseHeight = Number(hostEl && hostEl.offsetHeight ? hostEl.offsetHeight : height || 32);
          if (!Number.isFinite(width) || width <= 0) width = 32;
          if (!Number.isFinite(height) || height <= 0) height = 32;
          if (!Number.isFinite(baseWidth) || baseWidth <= 0) baseWidth = width;
          if (!Number.isFinite(baseHeight) || baseHeight <= 0) baseHeight = height;
          var expandedScale = this.bottomDockExpandedScale();
          var expandedWidth = baseWidth * expandedScale;
          var expandedHeight = baseHeight * expandedScale;
          this._bottomDockDragGhostWidth = Math.max(20, Math.min(112, Math.max(width, expandedWidth)));
          this._bottomDockDragGhostHeight = Math.max(20, Math.min(112, Math.max(height, expandedHeight)));
          var offsetX = Number(ev.clientX || 0) - Number(rect.left || 0);
          var offsetY = Number(ev.clientY || 0) - Number(rect.top || 0);
          var relX = Number.isFinite(offsetX) && width > 0 ? (offsetX / width) : 0.5;
          var relY = Number.isFinite(offsetY) && height > 0 ? (offsetY / height) : 0.5;
          relX = Math.max(0, Math.min(1, relX));
          relY = Math.max(0, Math.min(1, relY));
          this._bottomDockPointerGrabOffsetX = relX * this._bottomDockDragGhostWidth;
          this._bottomDockPointerGrabOffsetY = relY * this._bottomDockDragGhostHeight;
        } catch(_) {
          this._bottomDockPointerGrabOffsetX = 16;
          this._bottomDockPointerGrabOffsetY = 16;
          this._bottomDockDragGhostWidth = 32;
          this._bottomDockDragGhostHeight = 32;
        }
      } else {
        this._bottomDockPointerGrabOffsetX = 16;
        this._bottomDockPointerGrabOffsetY = 16;
        this._bottomDockDragGhostWidth = 32;
        this._bottomDockDragGhostHeight = 32;
      }
      try {
        if (hostEl && typeof hostEl.setPointerCapture === 'function' && Number.isFinite(ev.pointerId)) {
          hostEl.setPointerCapture(ev.pointerId);
        }
      } catch(_) {}
      this._bottomDockPointerActive = true;
      this._bottomDockPointerMoved = false;
      this._bottomDockPointerCandidateId = key;
      this._bottomDockPointerStartX = Number(ev.clientX || 0);
      this._bottomDockPointerStartY = Number(ev.clientY || 0);
      this._bottomDockPointerLastX = Number(ev.clientX || 0);
      this._bottomDockPointerLastY = Number(ev.clientY || 0);
      this._bottomDockReorderLockUntil = 0;
      this.bindBottomDockPointerListeners();
    },

    activateBottomDockPointerDrag(ev) {
      if (this._bottomDockPointerMoved) return;
      var dragId = String(this._bottomDockPointerCandidateId || '').trim();
      if (!dragId) return;
      this._bottomDockPointerMoved = true;
      this.bottomDockHoverId = '';
      this.bottomDockHoverWeightById = {};
      this.bottomDockPointerX = 0;
      this.bottomDockPointerY = 0;
      this.bottomDockPreviewVisible = false;
      this.bottomDockPreviewText = '';
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.cancelBottomDockPreviewReflow();
      this._bottomDockRevealTargetDuringSettle = false;
      this.bottomDockDragId = dragId;
      this.bottomDockDragCommitted = false;
      this.bottomDockDragStartOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      this.cleanupBottomDockDragGhost();
      this.captureBottomDockDragBoundaries(dragId);
      var originNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + dragId + '"]');
      if (!originNode || !document || !document.body) return;
      var dockEl = document.querySelector('.bottom-dock');
      if (dockEl && dockEl.style && typeof dockEl.style.setProperty === 'function') {
        dockEl.style.setProperty('--bottom-dock-drag-scale', String(this.readBottomDockScale(dockEl)));
      }
      var ghost = document.createElement('div');
      ghost.className = 'bottom-dock-drag-ghost bottom-dock-btn dock-tile';
      var tone = '';
      var iconKind = '';
      try {
        tone = String(originNode.getAttribute('data-dock-tone') || '').trim();
        iconKind = String(originNode.getAttribute('data-dock-icon') || '').trim();
      } catch(_) {
        tone = '';
        iconKind = '';
      }
      if (tone) ghost.setAttribute('data-dock-tone', tone);
      if (iconKind) ghost.setAttribute('data-dock-icon', iconKind);
      if (originNode.classList && typeof originNode.classList.contains === 'function') {
        if (originNode.classList.contains('active')) ghost.classList.add('active');
      }
      ghost.setAttribute('aria-hidden', 'true');
      ghost.innerHTML = String(originNode.innerHTML || '');
      ghost.style.position = 'fixed';
      ghost.style.width = Math.round(Number(this._bottomDockDragGhostWidth || 32)) + 'px';
      ghost.style.height = Math.round(Number(this._bottomDockDragGhostHeight || 32)) + 'px';
      ghost.style.borderRadius = Math.round((Number(this._bottomDockDragGhostWidth || 32) / 32) * 11) + 'px';
      ghost.style.setProperty(
        '--dock-ghost-scale',
        String(Math.max(0.8, Math.min(4, Number(this._bottomDockDragGhostWidth || 32) / 32)))
      );
      var ghostUpDeg = Number(this.bottomDockUpDegForSide(this.bottomDockActiveSide()) || 0);
      var ghostTileRotation = Math.round(ghostUpDeg) + 'deg';
      var ghostIconRotation = '0deg';
      ghost.style.setProperty('--bottom-dock-tile-rotation-deg', ghostTileRotation);
      ghost.style.setProperty('--bottom-dock-icon-rotation-deg', ghostIconRotation);
      var ghostX = Number(ev.clientX || 0) - Number(this._bottomDockPointerGrabOffsetX || 16);
      var ghostY = Number(ev.clientY || 0) - Number(this._bottomDockPointerGrabOffsetY || 16);
      this._bottomDockGhostCurrentX = ghostX;
      this._bottomDockGhostCurrentY = ghostY;
      ghost.style.left = Math.round(ghostX) + 'px';
      ghost.style.top = Math.round(ghostY) + 'px';
      ghost.style.margin = '0';
      ghost.style.pointerEvents = 'none';
      ghost.style.opacity = '1';
      document.body.appendChild(ghost);
      this._bottomDockDragGhostEl = ghost;
      this.setBottomDockGhostTarget(ghostX, ghostY);
    },

    handleBottomDockPointerMove(ev) {
      if (!this._bottomDockPointerActive) return;
      this._bottomDockPointerLastX = Number(ev.clientX || 0);
      this._bottomDockPointerLastY = Number(ev.clientY || 0);
      var movedX = Math.abs(Number(ev.clientX || 0) - Number(this._bottomDockPointerStartX || 0));
      var movedY = Math.abs(Number(ev.clientY || 0) - Number(this._bottomDockPointerStartY || 0));
      if (!this._bottomDockPointerMoved) {
        if (movedX < 5 && movedY < 5) return;
        this.activateBottomDockPointerDrag(ev);
      }
      if (!this._bottomDockPointerMoved) return;
      if (ev && typeof ev.preventDefault === 'function' && ev.cancelable) ev.preventDefault();
      var ghost = this._bottomDockDragGhostEl;
      if (ghost) {
        this.setBottomDockGhostTarget(
          Number(ev.clientX || 0) - Number(this._bottomDockPointerGrabOffsetX || 16),
          Number(ev.clientY || 0) - Number(this._bottomDockPointerGrabOffsetY || 16)
        );
      }
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var insertionIndex = this.bottomDockInsertionIndexFromPointer(dragId, ev);
      if (Number.isFinite(insertionIndex)) {
        var normalizedIndex = Math.max(0, Math.round(Number(insertionIndex || 0)));
        var nowMs = Date.now();
        var lockUntil = Number(this._bottomDockReorderLockUntil || 0);
        if (
          normalizedIndex !== Number(this._bottomDockLastInsertionIndex || -1) &&
          (!Number.isFinite(lockUntil) || lockUntil <= nowMs)
        ) {
          var changed = this.applyBottomDockReorderByIndex(dragId, normalizedIndex, true);
          this._bottomDockLastInsertionIndex = normalizedIndex;
          if (changed) {
            var moveDuration = this.bottomDockMoveDurationMs();
            var lockMs = Math.max(220, Math.min(420, Math.round(moveDuration * 0.55)));
            this._bottomDockReorderLockUntil = nowMs + lockMs;
          }
        }
        return;
      }
      var targetId = '';
      var targetEl = null;
      try {
        var pointerEl = typeof document !== 'undefined' && typeof document.elementFromPoint === 'function'
          ? document.elementFromPoint(Number(ev.clientX || 0), Number(ev.clientY || 0))
          : null;
        targetEl = pointerEl && typeof pointerEl.closest === 'function'
          ? pointerEl.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId && targetId !== dragId) {
        this._bottomDockLastInsertionIndex = -1;
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDragOver(targetId, ev, preferAfter);
        return;
      }
      if (!this.bottomDockShouldAppendFromPointer(dragId, ev)) return;
      var appendTargetId = this.bottomDockAppendTargetId(dragId);
      if (!appendTargetId) return;
      this._bottomDockLastInsertionIndex = -1;
      this.handleBottomDockDragOver(appendTargetId, ev, true);
    },

    endBottomDockPointerDrag() {
      if (!this._bottomDockPointerActive) return;
      this._bottomDockPointerActive = false;
      this.unbindBottomDockPointerListeners();
      if (!this._bottomDockPointerMoved) {
        this._bottomDockPointerCandidateId = '';
        return;
      }
      var dragId = String(this.bottomDockDragId || this._bottomDockPointerCandidateId || '').trim();
      if (dragId) {
        var finalPointerEvent = {
          clientX: Number(this._bottomDockPointerLastX || 0),
          clientY: Number(this._bottomDockPointerLastY || 0)
        };
        var finalInsertionIndex = this.bottomDockInsertionIndexFromPointer(dragId, finalPointerEvent);
        if (Number.isFinite(finalInsertionIndex)) {
          this.applyBottomDockReorderByIndex(dragId, finalInsertionIndex, false);
        } else if (this.bottomDockShouldAppendFromPointer(dragId, finalPointerEvent)) {
          var appendTargetId = this.bottomDockAppendTargetId(dragId);
          if (appendTargetId) {
            this.handleBottomDockDragOver(appendTargetId, finalPointerEvent, true);
          }
        }
      }
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
      if (JSON.stringify(current) !== JSON.stringify(start)) {
        this.bottomDockOrder = current;
        this.persistBottomDockOrder();
        this.bottomDockDragCommitted = true;
      }
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      var self = this;
      var finalizeDrag = function() {
        var dockEl = document.querySelector('.bottom-dock');
        if (dockEl && dockEl.style && typeof dockEl.style.removeProperty === 'function') {
          dockEl.style.removeProperty('--bottom-dock-drag-scale');
        }
        var dropX = Number(self._bottomDockPointerLastX || 0);
        var dropY = Number(self._bottomDockPointerLastY || 0);
        self.bottomDockDragId = '';
        self.bottomDockHoverId = '';
        self.bottomDockDragStartOrder = [];
        self._bottomDockPointerGrabOffsetX = 16;
        self._bottomDockPointerGrabOffsetY = 16;
        self._bottomDockDragGhostWidth = 32;
        self._bottomDockDragGhostHeight = 32;
        self._bottomDockPointerCandidateId = '';
        self._bottomDockPointerMoved = false;
        self._bottomDockDragBoundaries = [];
        self._bottomDockLastInsertionIndex = -1;
        self.reviveBottomDockHoverFromPoint(dropX, dropY);
        self._bottomDockPointerLastX = 0;
        self._bottomDockPointerLastY = 0;
      };
      this.settleBottomDockDragGhost(dragId, finalizeDrag);
    },

    shouldSuppressBottomDockClick() {
      var until = Number(this._bottomDockSuppressClickUntil || 0);
      return Number.isFinite(until) && until > Date.now();
    },

    clearBottomDockClickAnimation() {
      if (this._bottomDockClickAnimTimer) {
        try { clearTimeout(this._bottomDockClickAnimTimer); } catch(_) {}
      }
      this._bottomDockClickAnimTimer = 0;
      this.bottomDockClickAnimId = '';
    },

    triggerBottomDockClickAnimation(id, durationOverrideMs) {
      var key = String(id || '').trim();
      if (!key || typeof window === 'undefined' || typeof window.setTimeout !== 'function') return;
      this.clearBottomDockClickAnimation();
      this.bottomDockClickAnimId = key;
      var self = this;
      var durationMs = Number(durationOverrideMs);
      if (!Number.isFinite(durationMs) || durationMs < 120) {
        durationMs = Number(self._bottomDockClickAnimDurationMs || 980);
      }
      if (!Number.isFinite(durationMs) || durationMs < 120) durationMs = 980;
      if (typeof document !== 'undefined') {
        try {
          var tileNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
          if (tileNode && tileNode.style && typeof tileNode.style.setProperty === 'function') {
            tileNode.style.setProperty('--dock-click-duration', Math.round(durationMs) + 'ms');
          }
        } catch(_) {}
      }
      self._bottomDockClickAnimTimer = window.setTimeout(function() {
        if (typeof document !== 'undefined') {
          try {
            var activeNode = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
            if (activeNode && activeNode.style && typeof activeNode.style.removeProperty === 'function') {
              activeNode.style.removeProperty('--dock-click-duration');
            }
          } catch(_) {}
        }
        self._bottomDockClickAnimTimer = 0;
        self.bottomDockClickAnimId = '';
      }, durationMs);
    },

    bottomDockIsClickAnimating(id) {
      var key = String(id || '').trim();
      if (!key) return false;
      return String(this.bottomDockClickAnimId || '').trim() === key;
    },

    handleBottomDockTileClick(id, targetPage, ev) {
      if (this.shouldSuppressBottomDockClick()) return;
      var key = String(id || '').trim();
      var pageKey = String(targetPage || '').trim();
      var clickAnimation = '';
      var clickDurationMs = 0;
      try {
        var triggerEl = ev && ev.currentTarget ? ev.currentTarget : null;
        clickAnimation = String(
          triggerEl && typeof triggerEl.getAttribute === 'function'
            ? (triggerEl.getAttribute('data-dock-click-animation') || '')
            : ''
        ).trim();
        clickDurationMs = Number(
          triggerEl && typeof triggerEl.getAttribute === 'function'
            ? (triggerEl.getAttribute('data-dock-click-duration-ms') || '')
            : ''
        );
      } catch(_) {
        clickAnimation = '';
        clickDurationMs = 0;
      }
      if (!Number.isFinite(clickDurationMs) || clickDurationMs < 120) clickDurationMs = 0;
      if (key && clickAnimation && clickAnimation !== 'none') {
        this.triggerBottomDockClickAnimation(key, clickDurationMs);
      }
      if (pageKey) this.navigate(pageKey);
    },

    normalizeSidebarPopupText(rawText) {
      var text = String(rawText || '').trim();
      if (!text) return '';
      if (this.isSidebarPopupPlaceholderText(text)) return '';
      return text;
    },

    isSidebarPopupPlaceholderText(text) {
      var normalized = String(text || '').trim().toLowerCase();
      return normalized === 'no messages yet'
        || normalized === 'system events and terminal output'
        || normalized === 'no matching text'
        || normalized === 'agent';
    },

    sidebarPopupMetaOrigin(preview, fallbackLabel) {
      var role = String(preview && preview.role || '').trim().toLowerCase();
      if (role === 'user') return 'User';
      if (role === 'assistant' || role === 'agent') return 'Agent';
      if (role) return role.charAt(0).toUpperCase() + role.slice(1);
      return String(fallbackLabel || 'Sidebar').trim() || 'Sidebar';
    },

    hideDashboardPopupBySource(source) {
      var expected = String(source || '').trim();
      if (!expected) return;
      var popup = this.dashboardPopup || {};
      var currentSource = String(popup.source || '').trim();
      if (currentSource !== expected) return;
      this.hideDashboardPopup(String(popup.id || '').trim());
    },

    showCollapsedSidebarAgentPopup(agent, ev) {
      if (!this.sidebarCollapsed || !agent) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var rawId = String(agent.id || '').trim();
      var rawIdLower = rawId.toLowerCase();
      var isSystemThread = (typeof this.isSystemSidebarThread === 'function')
        ? this.isSystemSidebarThread(agent)
        : (agent.is_system_thread === true || rawIdLower === 'system');
      if (isSystemThread || rawIdLower === 'settings') {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var preview = this.chatSidebarPreview(agent) || {};
      var previewText = this.normalizeSidebarPopupText(preview.text || '');
      var title = String(agent.name || rawId).trim();
      if (!rawId || !title || !previewText) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      this.showDashboardPopup('sidebar-agent:' + rawId, title, ev, {
        source: 'sidebar',
        side: 'right',
        body: previewText,
        meta_origin: this.sidebarPopupMetaOrigin(preview, 'Agent'),
        meta_time: typeof this.formatChatSidebarTime === 'function'
          ? String(this.formatChatSidebarTime(preview.ts) || '').trim()
          : '',
        unread: !!preview.unread_response
      });
    },

    showCollapsedSidebarNavPopup(label, ev) {
      if (!this.sidebarCollapsed) {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      var navLabel = String(label || '').trim();
      var navLabelLower = navLabel.toLowerCase();
      if (!navLabel || navLabelLower === 'system' || navLabelLower === 'settings') {
        this.hideDashboardPopupBySource('sidebar');
        return;
      }
      this.showDashboardPopup('sidebar-nav:' + navLabelLower.replace(/[^a-z0-9_-]+/g, '-'), navLabel, ev, {
        source: 'sidebar',
        side: 'right',
        meta_origin: 'Sidebar'
      });
    },

    dashboardPopupService() {
      var root = typeof window !== 'undefined' ? window : {};
      var services = root && root.InfringSharedShellServices;
      return services && services.popup ? services.popup : null;
    },

    clearDashboardPopupState() {
      var service = this.dashboardPopupService();
      this.dashboardPopup = service && typeof service.emptyState === 'function'
        ? service.emptyState()
        : {
          id: '',
          active: false,
          source: '',
          title: '',
          body: '',
          meta_origin: '',
          meta_time: '',
          unread: false,
          left: 0,
          top: 0,
          side: 'bottom',
          inline_away: 'right',
          block_away: 'bottom',
          compact: false
        };
    },

    normalizeDashboardPopupSide(sideValue, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.normalizeSide === 'function') {
        return service.normalizeSide(sideValue, fallbackSide);
      }
      var fallback = String(fallbackSide || 'bottom').trim().toLowerCase();
      if (fallback !== 'top' && fallback !== 'left' && fallback !== 'right') fallback = 'bottom';
      var side = String(sideValue || fallback).trim().toLowerCase();
      if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
      return side;
    },

    dashboardOppositeSide(sideValue) {
      var service = this.dashboardPopupService();
      if (service && typeof service.oppositeSide === 'function') {
        return service.oppositeSide(sideValue);
      }
      var side = this.normalizeDashboardPopupSide(sideValue, 'bottom');
      if (side === 'top') return 'bottom';
      if (side === 'left') return 'right';
      if (side === 'right') return 'left';
      return 'top';
    },

    dashboardPopupWallAffinity(rect) {
      var service = this.dashboardPopupService();
      if (service && typeof service.wallAffinity === 'function') {
        return service.wallAffinity(rect);
      }
      if (!rect || typeof window === 'undefined') return null;
      var viewportWidth = Number(window.innerWidth || 0);
      var viewportHeight = Number(window.innerHeight || 0);
      if (!Number.isFinite(viewportWidth) || viewportWidth <= 0) viewportWidth = 1;
      if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) viewportHeight = 1;
      var left = Number(rect.left || 0);
      var right = Number(rect.right || 0);
      var top = Number(rect.top || 0);
      var bottom = Number(rect.bottom || 0);
      if (!Number.isFinite(left) || !Number.isFinite(right) || !Number.isFinite(top) || !Number.isFinite(bottom)) {
        return null;
      }
      var width = Math.max(1, Math.abs(right - left));
      var height = Math.max(1, Math.abs(bottom - top));
      var distanceToLeft = Math.max(0, left);
      var distanceToRight = Math.max(0, viewportWidth - right);
      var distanceToTop = Math.max(0, top);
      var distanceToBottom = Math.max(0, viewportHeight - bottom);
      var proximityScore = function(distance) {
        var normalized = Number(distance || 0);
        if (!Number.isFinite(normalized) || normalized < 0) normalized = 0;
        return 1 / (1 + normalized);
      };
      return {
        scores: {
          top: width * proximityScore(distanceToTop),
          bottom: width * proximityScore(distanceToBottom),
          left: height * proximityScore(distanceToLeft),
          right: height * proximityScore(distanceToRight)
        },
        distances: {
          top: distanceToTop,
          bottom: distanceToBottom,
          left: distanceToLeft,
          right: distanceToRight
        }
      };
    },

    dashboardPopupWallAnchorNode(node) {
      if (!node || typeof node.closest !== 'function') return null;
      try {
        return node.closest(
          '[data-popup-wall-anchor], .global-taskbar, .sidebar, .bottom-dock, .doc-window, .chat-window'
        );
      } catch(_) {
        return null;
      }
    },

    dashboardPopupWallRectForNode(node) {
      var anchor = this.dashboardPopupWallAnchorNode(node);
      if (!anchor || typeof anchor.getBoundingClientRect !== 'function') return null;
      try {
        return anchor.getBoundingClientRect();
      } catch(_) {
        return null;
      }
    },

    dashboardPopupUsableAnchorRect(node) {
      if (!node || typeof node.getBoundingClientRect !== 'function') return null;
      var rect = null;
      try {
        rect = node.getBoundingClientRect();
      } catch(_) {
        rect = null;
      }
      var width = rect ? Math.abs(Number(rect.right || 0) - Number(rect.left || 0)) : 0;
      var height = rect ? Math.abs(Number(rect.bottom || 0) - Number(rect.top || 0)) : 0;
      if (rect && width > 0 && height > 0) return rect;
      if (node && typeof node.closest === 'function') {
        try {
          var fallback = node.closest('[data-popup-origin-anchor], .composer-menu-pill, .composer-input-pill, .taskbar-text-menu-anchor, .taskbar-hero-menu-anchor, .notif-wrap');
          if (fallback && fallback !== node && typeof fallback.getBoundingClientRect === 'function') {
            rect = fallback.getBoundingClientRect();
            width = rect ? Math.abs(Number(rect.right || 0) - Number(rect.left || 0)) : 0;
            height = rect ? Math.abs(Number(rect.bottom || 0) - Number(rect.top || 0)) : 0;
            if (rect && width > 0 && height > 0) return rect;
          }
        } catch(_) {}
      }
      return null;
    },

    dashboardPopupSideAwayFromNearestWall(rect, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.sideAwayFromNearestWall === 'function') {
        return service.sideAwayFromNearestWall(rect, fallbackSide);
      }
      var fallback = this.normalizeDashboardPopupSide('', fallbackSide);
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.scores || !affinity.distances) return fallback;
      var scores = affinity.scores;
      var distances = affinity.distances;
      var walls = ['top', 'bottom', 'left', 'right'];
      var fallbackWall = this.dashboardOppositeSide(fallback);
      var winner = walls[0];
      var winnerScore = Number(scores[winner] || 0);
      var epsilon = 0.000001;
      var i;
      for (i = 1; i < walls.length; i += 1) {
        var wall = walls[i];
        var score = Number(scores[wall] || 0);
        if (score > winnerScore + epsilon) {
          winner = wall;
          winnerScore = score;
          continue;
        }
        if (Math.abs(score - winnerScore) <= epsilon) {
          if (wall === fallbackWall && winner !== fallbackWall) {
            winner = wall;
            winnerScore = score;
            continue;
          }
          var wallDistance = Number(distances[wall] || 0);
          var winnerDistance = Number(distances[winner] || 0);
          if (wallDistance < winnerDistance) {
            winner = wall;
            winnerScore = score;
          }
        }
      }
      return this.dashboardOppositeSide(winner);
    },

    dashboardPopupHorizontalAwayFromNearestWall(rect, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.horizontalAwayFromNearestWall === 'function') {
        return service.horizontalAwayFromNearestWall(rect, fallbackSide);
      }
      var fallback = String(fallbackSide || 'right').trim().toLowerCase();
      if (fallback !== 'left') fallback = 'right';
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.distances) return fallback;
      var distances = affinity.distances;
      var nearest = Number(distances.left || 0) <= Number(distances.right || 0)
        ? 'left'
        : 'right';
      return nearest === 'left' ? 'right' : 'left';
    },

    dashboardPopupVerticalAwayFromNearestWall(rect, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.verticalAwayFromNearestWall === 'function') {
        return service.verticalAwayFromNearestWall(rect, fallbackSide);
      }
      var fallback = String(fallbackSide || 'bottom').trim().toLowerCase();
      if (fallback !== 'top') fallback = 'bottom';
      var affinity = this.dashboardPopupWallAffinity(rect);
      if (!affinity || !affinity.distances) return fallback;
      var distances = affinity.distances;
      var nearest = Number(distances.top || 0) <= Number(distances.bottom || 0)
        ? 'top'
        : 'bottom';
      return nearest === 'top' ? 'bottom' : 'top';
    },

    dashboardPopupAxisAwareSideAway(rect, fallbackSide) {
      var service = this.dashboardPopupService();
      if (service && typeof service.axisAwareSideAway === 'function') {
        return service.axisAwareSideAway(rect, fallbackSide);
      }
      var fallback = this.normalizeDashboardPopupSide('', fallbackSide || 'bottom');
      if (fallback === 'left' || fallback === 'right') {
        return this.dashboardPopupHorizontalAwayFromNearestWall(rect, fallback);
      }
      return this.dashboardPopupVerticalAwayFromNearestWall(rect, fallback);
    },

    taskbarAnchoredDropdownClass(anchorNode, fallbackSide, layoutKey) {
      var fallback = this.normalizeDashboardPopupSide('', fallbackSide || 'bottom');
      var anchorRect = anchorNode && typeof anchorNode.getBoundingClientRect === 'function'
        ? this.dashboardPopupUsableAnchorRect(anchorNode)
        : null;
      var service = this.dashboardPopupService();
      if (service && typeof service.dropdownClass === 'function') {
        return service.dropdownClass(anchorRect, fallback, layoutKey);
      }
      String(layoutKey == null ? '' : layoutKey);
      var side = fallback;
      var inlineAway = 'right';
      var blockAway = 'bottom';
      if (anchorRect) {
        side = this.dashboardPopupAxisAwareSideAway(anchorRect, fallback);
        inlineAway = this.dashboardPopupHorizontalAwayFromNearestWall(anchorRect, 'right');
        blockAway = this.dashboardPopupVerticalAwayFromNearestWall(anchorRect, 'bottom');
      }
      return {
        'taskbar-anchored-dropdown': true,
        'is-side-top': side === 'top',
        'is-side-bottom': side === 'bottom',
        'is-side-left': side === 'left',
        'is-side-right': side === 'right',
        'is-inline-away-left': inlineAway === 'left',
        'is-inline-away-right': inlineAway === 'right',
        'is-block-away-top': blockAway === 'top',
        'is-block-away-bottom': blockAway === 'bottom'
      };
    },

    dashboardPopupAnchorPoint(ev, sideOverride) {
      var preferredSide = this.normalizeDashboardPopupSide(sideOverride, 'bottom');
      var node = ev && ev.currentTarget ? ev.currentTarget : null;
      if (!node && ev && ev.target && typeof ev.target.closest === 'function') {
        try {
          node = ev.target.closest('button,[role="button"],.taskbar-reorder-item');
        } catch(_) {
          node = null;
        }
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') {
        return { left: 0, top: 0, side: preferredSide, inline_away: 'right', block_away: 'bottom' };
      }
      var rect = node.getBoundingClientRect();
      var service = this.dashboardPopupService();
      if (service && typeof service.anchorPoint === 'function') {
        return service.anchorPoint(rect, preferredSide);
      }
      var side = this.dashboardPopupAxisAwareSideAway(rect, preferredSide);
      var inlineAway = this.dashboardPopupHorizontalAwayFromNearestWall(rect, 'right');
      var blockAway = this.dashboardPopupVerticalAwayFromNearestWall(rect, 'bottom');
      var left = Math.round(Number(rect.left || 0));
      var top = Math.round(Number(rect.bottom || 0));
      if (side === 'top') {
        left = inlineAway === 'left'
          ? Math.round(Number(rect.right || 0))
          : Math.round(Number(rect.left || 0));
        top = Math.round(Number(rect.top || 0));
      } else if (side === 'bottom') {
        left = inlineAway === 'left'
          ? Math.round(Number(rect.right || 0))
          : Math.round(Number(rect.left || 0));
        top = Math.round(Number(rect.bottom || 0));
      } else if (side === 'left') {
        left = Math.round(Number(rect.left || 0));
        top = blockAway === 'top'
          ? Math.round(Number(rect.bottom || 0))
          : Math.round(Number(rect.top || 0));
      } else if (side === 'right') {
        left = Math.round(Number(rect.right || 0));
        top = blockAway === 'top'
          ? Math.round(Number(rect.bottom || 0))
          : Math.round(Number(rect.top || 0));
      }
      return {
        left: left,
        top: top,
        side: side,
        inline_away: inlineAway === 'left' ? 'left' : 'right',
        block_away: blockAway === 'top' ? 'top' : 'bottom'
      };
    },

    showDashboardPopup(id, label, ev, overrides) {
      var popupId = String(id || '').trim();
      var title = String(label || '').trim();
      if (!popupId || !title) {
        this.hideDashboardPopup();
        return;
      }
      var eventType = String((ev && ev.type) || '').toLowerCase();
      if (
        eventType === 'mouseleave' ||
        eventType === 'pointerleave' ||
        eventType === 'blur' ||
        eventType === 'focusout'
      ) {
        this.hideDashboardPopup(popupId);
        return;
      }
      if (ev && ev.isTrusted === false) return;
      var config = overrides && typeof overrides === 'object' ? overrides : {};
      var anchor = this.dashboardPopupAnchorPoint(ev, config.side);
      var service = this.dashboardPopupService();
      this.dashboardPopup = service && typeof service.openState === 'function'
        ? service.openState(popupId, title, config, anchor)
        : {
          id: popupId,
          active: true,
          source: String(config.source || '').trim(),
          title: title,
          body: String(config.body || '').trim(),
          meta_origin: String(config.meta_origin || 'Taskbar').trim(),
          meta_time: String(config.meta_time || '').trim(),
          unread: !!config.unread,
          left: anchor.left,
          top: anchor.top,
          side: anchor.side,
          inline_away: anchor.inline_away === 'left' ? 'left' : 'right',
          block_away: anchor.block_away === 'top' ? 'top' : 'bottom',
          compact: false
        };
    },

    showTaskbarNavPopup(label, ev) {
      var navLabel = String(label || '').trim();
      if (!navLabel) {
        this.hideDashboardPopup();
        return;
      }
      var navKey = navLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-');
      var body = navKey === 'back'
        ? (this.canNavigateBack() ? 'Go to the previous page in this session' : 'No earlier page in this session')
        : (this.canNavigateForward() ? 'Go to the next page in this session' : 'No later page in this session');
      this.showDashboardPopup('taskbar-nav:' + navKey, navLabel, ev, {
        source: 'taskbar',
        side: 'bottom',
        compact: false,
        body: body,
        meta_origin: 'Chat nav'
      });
    },

    showTaskbarUtilityPopup(label, body, ev) {
      var utilityLabel = String(label || '').trim();
      if (!utilityLabel) {
        this.hideDashboardPopup();
        return;
      }
      this.showDashboardPopup(
        'taskbar-utility:' + utilityLabel.toLowerCase().replace(/[^a-z0-9_-]+/g, '-'),
        utilityLabel,
        ev,
        {
          source: 'taskbar',
          side: 'bottom',
          compact: false,
          body: String(body || '').trim(),
          meta_origin: 'Taskbar'
        }
      );
    },

    hideDashboardPopup(rawId) {
      var service = this.dashboardPopupService();
      if (service && typeof service.closeState === 'function') {
        this.dashboardPopup = service.closeState(this.dashboardPopup, rawId);
        return;
      }
      var popupId = String(rawId || '').trim();
      var currentId = String(this.dashboardPopup && this.dashboardPopup.id || '').trim();
      if (popupId && currentId && popupId !== currentId) return;
      this.clearDashboardPopupState();
    },

    bottomDockIsDraggingVisual(id) {
      var key = String(id || '').trim();
      if (!key) return false;
      if (this._bottomDockRevealTargetDuringSettle) return false;
      return String(this.bottomDockDragId || '').trim() === key;
    },

    bottomDockIsNeighbor(id) {
      var hoverId = String(this.bottomDockHoverId || '').trim();
      var key = String(id || '').trim();
      if (!hoverId || !key || hoverId === key) return false;
      return Math.abs(this.bottomDockOrderIndex(hoverId) - this.bottomDockOrderIndex(key)) === 1;
    },

    bottomDockIsSecondNeighbor(id) {
      var hoverId = String(this.bottomDockHoverId || '').trim();
      var key = String(id || '').trim();
      if (!hoverId || !key || hoverId === key) return false;
      return Math.abs(this.bottomDockOrderIndex(hoverId) - this.bottomDockOrderIndex(key)) === 2;
    },

    bottomDockHoverWeight(id) {
      var key = String(id || '').trim();
      if (!key) return 0;
      var weights = this.bottomDockHoverWeightById && typeof this.bottomDockHoverWeightById === 'object'
        ? this.bottomDockHoverWeightById
        : null;
      if (weights && Object.prototype.hasOwnProperty.call(weights, key)) {
        var exact = Number(weights[key] || 0);
        if (Number.isFinite(exact)) return Math.max(0, Math.min(1, exact));
      }
      if (key === String(this.bottomDockHoverId || '').trim()) return 1;
      if (this.bottomDockIsNeighbor(key)) return 0.33;
      if (this.bottomDockIsSecondNeighbor(key)) return 0.11;
      return 0;
    },

    startBottomDockDrag(id, ev) {
      var key = String(id || '').trim();
      if (!key) return;
      this.cleanupBottomDockDragGhost();
      this.bottomDockHoverId = '';
      this.bottomDockHoverWeightById = {};
      this.bottomDockPointerX = 0;
      this.bottomDockPointerY = 0;
      this.bottomDockPreviewVisible = false;
      this.bottomDockPreviewText = '';
      this.bottomDockPreviewMorphFromText = '';
      this.bottomDockPreviewLabelMorphing = false;
      this.bottomDockPreviewWidth = 0;
      this.cancelBottomDockPreviewReflow();
      this.bottomDockDragId = key;
      this.bottomDockDragCommitted = false;
      this.bottomDockDragStartOrder = this.normalizeBottomDockOrder(this.bottomDockOrder);
      this._bottomDockReorderLockUntil = 0;
      this.captureBottomDockDragBoundaries(key);
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.effectAllowed = 'move'; } catch(_) {}
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
        try {
          var dragNode = ev.currentTarget;
          if (dragNode && typeof ev.dataTransfer.setDragImage === 'function') {
            var rect = dragNode.getBoundingClientRect();
            var ghost = dragNode.cloneNode(true);
            if (ghost && document && document.body) {
              ghost.classList.add('bottom-dock-drag-ghost');
              ghost.style.position = 'fixed';
              ghost.style.left = '-9999px';
              ghost.style.top = '-9999px';
              ghost.style.margin = '0';
              ghost.style.transform = 'none';
              ghost.style.pointerEvents = 'none';
              ghost.style.opacity = '1';
              document.body.appendChild(ghost);
              this._bottomDockDragGhostEl = ghost;
              ev.dataTransfer.setDragImage(
                ghost,
                Math.max(0, Math.round(Number(rect.width || 0) / 2)),
                Math.max(0, Math.round(Number(rect.height || 0) / 2))
              );
            } else {
              ev.dataTransfer.setDragImage(
                dragNode,
                Math.max(0, Math.round(Number(rect.width || 0) / 2)),
                Math.max(0, Math.round(Number(rect.height || 0) / 2))
              );
            }
          }
        } catch(_) {}
        try { ev.dataTransfer.setData('application/x-infring-dock', key); } catch(_) {}
        try { ev.dataTransfer.setData('text/plain', key); } catch(_) {}
      }
    },

    bottomDockShouldInsertAfter(targetId, ev, targetEl) {
      var key = String(targetId || '').trim();
      if (!key) return false;
      if (!ev) return false;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
      var node = targetEl || null;
      if (!node && typeof document !== 'undefined') {
        try {
          node = document.querySelector('.bottom-dock-btn[data-dock-id="' + key + '"]');
        } catch(_) {
          node = null;
        }
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') return false;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) return false;
      if (!Number.isFinite(height) || height <= 0) return false;
      var basis = this.bottomDockAxisBasis();
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      var centerProj = this.bottomDockProjectPointToAxis(centerX, centerY, basis);
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var half = this.bottomDockAxisHalfExtent(width, height, basis).primary;
      if (!Number.isFinite(half) || half <= 0) half = Math.max(width, height) / 2;
      if (!Number.isFinite(half) || half <= 0) return false;
      var ratio = (pointerProj.primary - (centerProj.primary - half)) / (half * 2);
      return ratio >= 0.5;
    },

    captureBottomDockDragBoundaries(dragId) {
      var key = String(dragId || '').trim();
      if (!key || typeof document === 'undefined') {
        this._bottomDockDragBoundaries = [];
        this._bottomDockLastInsertionIndex = -1;
        return [];
      }
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock) {
        this._bottomDockDragBoundaries = [];
        this._bottomDockLastInsertionIndex = -1;
        return [];
      }
      var centers = [];
      var basis = this.bottomDockAxisBasis();
      try {
        var nodes = dock.querySelectorAll('.bottom-dock-btn[data-dock-id]');
        for (var i = 0; i < nodes.length; i += 1) {
          var node = nodes[i];
          if (!node || typeof node.getAttribute !== 'function') continue;
          var id = String(node.getAttribute('data-dock-id') || '').trim();
          if (!id || id === key || typeof node.getBoundingClientRect !== 'function') continue;
          var rect = node.getBoundingClientRect();
          var width = Number(rect.width || 0);
          var height = Number(rect.height || 0);
          if (!Number.isFinite(width) || width <= 0) continue;
          if (!Number.isFinite(height) || height <= 0) continue;
          var centerX = Number(rect.left || 0) + (width / 2);
          var centerY = Number(rect.top || 0) + (height / 2);
          centers.push(this.bottomDockProjectPointToAxis(centerX, centerY, basis).primary);
        }
      } catch(_) {}
      centers.sort(function(a, b) { return a - b; });
      this._bottomDockDragBoundaries = centers;
      this._bottomDockLastInsertionIndex = -1;
      return centers;
    },

    bottomDockAppendTargetId(dragId) {
      var key = String(dragId || '').trim();
      if (!key) return '';
      var order = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var filtered = [];
      for (var i = 0; i < order.length; i += 1) {
        var id = String(order[i] || '').trim();
        if (!id || id === key) continue;
        filtered.push(id);
      }
      if (!filtered.length) return '';
      return String(filtered[filtered.length - 1] || '').trim();
    },

    bottomDockShouldAppendFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev || typeof document === 'undefined') return false;
      var clientX = Number(ev.clientX || 0);
      var clientY = Number(ev.clientY || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return false;
      var appendTargetId = this.bottomDockAppendTargetId(key);
      if (!appendTargetId) return false;
      var node = null;
      try {
        node = document.querySelector('.bottom-dock-btn[data-dock-id="' + appendTargetId + '"]');
      } catch(_) {
        node = null;
      }
      if (!node || typeof node.getBoundingClientRect !== 'function') return false;
      var rect = node.getBoundingClientRect();
      var width = Number(rect.width || 0);
      var height = Number(rect.height || 0);
      if (!Number.isFinite(width) || width <= 0) return false;
      if (!Number.isFinite(height) || height <= 0) return false;
      var basis = this.bottomDockAxisBasis();
      var centerX = Number(rect.left || 0) + (width / 2);
      var centerY = Number(rect.top || 0) + (height / 2);
      var centerProj = this.bottomDockProjectPointToAxis(centerX, centerY, basis);
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var extent = this.bottomDockAxisHalfExtent(width, height, basis);
      var halfPrimary = Number(extent.primary || 0);
      var halfSecondary = Number(extent.secondary || 0);
      if (!Number.isFinite(halfPrimary) || halfPrimary <= 0) halfPrimary = Math.max(width, height) / 2;
      if (!Number.isFinite(halfSecondary) || halfSecondary <= 0) halfSecondary = Math.min(width, height) / 2;
      var secondaryPad = Math.max(18, halfSecondary * 0.75);
      if (Math.abs(pointerProj.secondary - centerProj.secondary) > (halfSecondary + secondaryPad)) return false;
      var threshold = centerProj.primary + halfPrimary - Math.min(18, halfPrimary * 0.7);
      return pointerProj.primary >= threshold;
    },

    bottomDockInsertionIndexFromCoords(dragId, clientXRaw, clientYRaw) {
      var key = String(dragId || '').trim();
      if (!key || typeof document === 'undefined') return null;
      var clientX = Number(clientXRaw || 0);
      var clientY = Number(clientYRaw || 0);
      if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) return null;
      var dock = null;
      try {
        dock = document.querySelector('.bottom-dock');
      } catch(_) {
        dock = null;
      }
      if (!dock || typeof dock.getBoundingClientRect !== 'function') return null;
      var dockRect = dock.getBoundingClientRect();
      var basis = this.bottomDockAxisBasis();
      var pointerProj = this.bottomDockProjectPointToAxis(clientX, clientY, basis);
      var dockBounds = this.bottomDockProjectedRectBounds(dockRect, basis);
      if (!dockBounds) return null;
      if (
        pointerProj.secondary < (Number(dockBounds.secondaryMin || 0) - 24) ||
        pointerProj.secondary > (Number(dockBounds.secondaryMax || 0) + 24)
      ) return null;
      var centers = this.captureBottomDockDragBoundaries(key);
      if (centers.length === 0) return null;
      var insertionIndex = 0;
      for (var c = 0; c < centers.length; c += 1) {
        if (pointerProj.primary >= centers[c]) insertionIndex += 1;
      }
      insertionIndex = Math.max(0, Math.min(centers.length, insertionIndex));
      return insertionIndex;
    },

    bottomDockGhostCenterPoint() {
      var x = Number(this._bottomDockGhostTargetX || this._bottomDockGhostCurrentX || 0);
      var y = Number(this._bottomDockGhostTargetY || this._bottomDockGhostCurrentY || 0);
      var width = Number(this._bottomDockDragGhostWidth || 0);
      var height = Number(this._bottomDockDragGhostHeight || 0);
      if (!Number.isFinite(width) || width <= 0) width = 32;
      if (!Number.isFinite(height) || height <= 0) height = 32;
      return {
        x: x + (width / 2),
        y: y + (height / 2)
      };
    },

    bottomDockInsertionIndexFromPointer(dragId, ev) {
      var key = String(dragId || '').trim();
      if (!key || !ev) return null;
      var center = this.bottomDockGhostCenterPoint();
      return this.bottomDockInsertionIndexFromCoords(key, center.x, center.y);
    },

    applyBottomDockReorderByIndex(dragId, insertionIndex, animate) {
      var key = String(dragId || '').trim();
      if (!key) return false;
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var fromIndex = current.indexOf(key);
      if (fromIndex < 0) return false;
      var next = current.slice();
      next.splice(fromIndex, 1);
      var idx = Number(insertionIndex);
      if (!Number.isFinite(idx)) return false;
      idx = Math.max(0, Math.min(next.length, Math.round(idx)));
      next.splice(idx, 0, key);
      if (JSON.stringify(next) === JSON.stringify(current)) return false;
      var doAnimate = Boolean(animate);
      var beforeRects = doAnimate ? this.bottomDockButtonRects() : null;
      this.bottomDockOrder = next;
      if (doAnimate && beforeRects) this.animateBottomDockFromRects(beforeRects);
      return true;
    },
    persistBottomDockOrderIfChangedFromDragStart() {
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
      if (JSON.stringify(current) !== JSON.stringify(start)) {
        this.bottomDockOrder = current;
        this.persistBottomDockOrder();
        this.bottomDockDragCommitted = true;
      }
    },
    completeBottomDockDropCleanup(ev) {
      this.bottomDockDragId = '';
      this.bottomDockDragStartOrder = [];
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
      this.reviveBottomDockHoverFromPoint(
        Number(ev && ev.clientX || 0),
        Number(ev && ev.clientY || 0)
      );
      if (ev && typeof ev.preventDefault === 'function') ev.preventDefault();
    },

    handleBottomDockContainerDragOver(ev) {
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
      }
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var targetId = '';
      var targetEl = null;
      try {
        targetEl = ev && ev.target && typeof ev.target.closest === 'function'
          ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId && targetId !== dragId) {
        this._bottomDockLastInsertionIndex = -1;
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDragOver(targetId, ev, preferAfter);
        return;
      }
      if (!this.bottomDockShouldAppendFromPointer(dragId, ev)) return;
      var appendTargetId = this.bottomDockAppendTargetId(dragId);
      if (!appendTargetId) return;
      this._bottomDockLastInsertionIndex = -1;
      this.handleBottomDockDragOver(appendTargetId, ev, true);
    },

    handleBottomDockContainerDrop(ev) {
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!dragId) return;
      var targetId = '';
      var targetEl = null;
      try {
        targetEl = ev && ev.target && typeof ev.target.closest === 'function'
          ? ev.target.closest('.bottom-dock-btn[data-dock-id]')
          : null;
        targetId = targetEl ? String(targetEl.getAttribute('data-dock-id') || '').trim() : '';
      } catch(_) {}
      if (targetId) {
        var preferAfter = this.bottomDockShouldInsertAfter(targetId, ev, targetEl);
        this.handleBottomDockDrop(targetId, ev, preferAfter);
        return;
      }
      if (this.bottomDockShouldAppendFromPointer(dragId, ev)) {
        var appendTargetId = this.bottomDockAppendTargetId(dragId);
        if (appendTargetId) {
          this.handleBottomDockDrop(appendTargetId, ev, true);
          return;
        }
      }
      this.persistBottomDockOrderIfChangedFromDragStart();
      this.completeBottomDockDropCleanup(ev);
    },

    handleBottomDockDragOver(id, ev, preferAfter) {
      var targetId = String(id || '').trim();
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!targetId || !dragId || targetId === dragId) return;
      var nowMs = Date.now();
      var lockUntil = Number(this._bottomDockReorderLockUntil || 0);
      if (Number.isFinite(lockUntil) && lockUntil > nowMs) return;
      if (ev && ev.dataTransfer) {
        try { ev.dataTransfer.dropEffect = 'move'; } catch(_) {}
      }
      var placeAfter = Boolean(preferAfter);
      var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var next = current.slice();
      var fromIndex = next.indexOf(dragId);
      var toIndex = next.indexOf(targetId);
      if (fromIndex < 0 || toIndex < 0 || fromIndex === toIndex) return;
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (placeAfter) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      if (JSON.stringify(next) === JSON.stringify(current)) return;
      var beforeRects = this.bottomDockButtonRects();
      this.bottomDockOrder = next;
      this.animateBottomDockFromRects(beforeRects);
      var moveDuration = this.bottomDockMoveDurationMs();
      var lockMs = Math.max(320, Math.min(520, Math.round(moveDuration + 60)));
      this._bottomDockReorderLockUntil = nowMs + lockMs;
    },

    handleBottomDockDrop(id, ev, preferAfter) {
      var targetId = String(id || '').trim();
      var dragId = String(this.bottomDockDragId || '').trim();
      if (!targetId || !dragId) {
        this._bottomDockSuppressClickUntil = Date.now() + 220;
        this.cleanupBottomDockDragGhost();
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this.bottomDockDragCommitted = false;
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        return;
      }
      if (targetId === dragId) {
        this.persistBottomDockOrderIfChangedFromDragStart();
        this.completeBottomDockDropCleanup(ev);
        return;
      }
      var next = this.normalizeBottomDockOrder(this.bottomDockOrder);
      var fromIndex = next.indexOf(dragId);
      var toIndex = next.indexOf(targetId);
      var placeAfter = Boolean(preferAfter);
      if (fromIndex < 0 || toIndex < 0) {
        this.bottomDockDragId = '';
        this.bottomDockDragStartOrder = [];
        this.bottomDockDragCommitted = false;
        this.reviveBottomDockHoverFromPoint(
          Number(ev && ev.clientX || 0),
          Number(ev && ev.clientY || 0)
        );
        return;
      }
      next.splice(fromIndex, 1);
      if (fromIndex < toIndex) toIndex -= 1;
      if (placeAfter) toIndex += 1;
      if (toIndex < 0) toIndex = 0;
      if (toIndex > next.length) toIndex = next.length;
      next.splice(toIndex, 0, dragId);
      this.bottomDockOrder = next;
      this.persistBottomDockOrder();
      this.bottomDockDragCommitted = true;
      this.completeBottomDockDropCleanup(ev);
    },

    endBottomDockDrag() {
      if (!this.bottomDockDragCommitted && Array.isArray(this.bottomDockDragStartOrder) && this.bottomDockDragStartOrder.length) {
        var current = this.normalizeBottomDockOrder(this.bottomDockOrder);
        var start = this.normalizeBottomDockOrder(this.bottomDockDragStartOrder);
        if (JSON.stringify(current) !== JSON.stringify(start)) {
          this.bottomDockOrder = current;
          this.persistBottomDockOrder();
          this.bottomDockDragCommitted = true;
        } else {
          var beforeRects = this.bottomDockButtonRects();
          this.bottomDockOrder = start;
          this.animateBottomDockFromRects(beforeRects);
        }
      }
      this.bottomDockDragId = '';
      this.bottomDockHoverId = '';
      this.bottomDockDragStartOrder = [];
      this.bottomDockDragCommitted = false;
      this._bottomDockSuppressClickUntil = Date.now() + 220;
      this.cleanupBottomDockDragGhost();
    },

    dashboardPopupOrigin(overrides) {
      var service = this.dashboardPopupService();
      if (service && typeof service.origin === 'function') {
        return service.origin(overrides);
      }
      return Object.assign({
        source: '',
        active: false,
        ready: false,
        side: 'top',
        inline_away: 'right',
        block_away: 'bottom',
        left: 0,
        top: 0,
        compact: false,
        title: '',
        body: '',
        meta_origin: '',
        meta_time: '',
        unread: false
      }, overrides || {});
    },

    bottomDockPopupOrigin() {
      var label = String(this.bottomDockPreviewText || '').trim();
      var left = Math.round(Number(this.bottomDockPreviewX || 0));
      var top = Math.round(Number(this.bottomDockPreviewY || 0));
      if (!this.bottomDockPreviewVisible || !label) return this.dashboardPopupOrigin();
      return this.dashboardPopupOrigin({
        source: 'bottom_dock',
        active: true,
        ready: left > 0 && top > 0,
        side: this.bottomDockOpenSide(),
        inline_away: 'center',
        block_away: 'center',
        left: left,
        top: top,
        compact: false,
        title: label
      });
    },

    dashboardPopupStateOrigin() {
      var service = this.dashboardPopupService();
      if (service && typeof service.stateOrigin === 'function') {
        return service.stateOrigin(this.dashboardPopup);
      }
      var popup = this.dashboardPopup || {};
      var title = String(popup.title || '').trim();
      var body = String(popup.body || '').trim();
      var left = Math.round(Number(popup.left || 0));
      var top = Math.round(Number(popup.top || 0));
      var side = String(popup.side || 'bottom').trim().toLowerCase();
      var inlineAway = String(popup.inline_away || 'right').trim().toLowerCase();
      var blockAway = String(popup.block_away || 'bottom').trim().toLowerCase();
      if (side !== 'top' && side !== 'left' && side !== 'right') side = 'bottom';
      if (inlineAway !== 'left' && inlineAway !== 'right') inlineAway = 'center';
      if (blockAway !== 'top' && blockAway !== 'bottom') blockAway = 'center';
      if (!popup.active || !title) return this.dashboardPopupOrigin();
      return this.dashboardPopupOrigin({
        source: String(popup.source || 'ui').trim(),
        active: true,
        ready: left > 0 && top > 0,
        side: side,
        inline_away: inlineAway,
        block_away: blockAway,
        left: left,
        top: top,
        compact: false,
        title: title,
        body: body,
        meta_origin: String(popup.meta_origin || '').trim(),
        meta_time: String(popup.meta_time || '').trim(),
        unread: !!popup.unread
      });
    },

    activeDashboardPopupOrigin() {
      var sharedPopup = this.dashboardPopupStateOrigin();
      if (sharedPopup.active && sharedPopup.ready) return sharedPopup;
      var dockPopup = this.bottomDockPopupOrigin();
      if (dockPopup.active && dockPopup.ready) return dockPopup;
      return this.dashboardPopupOrigin();
    },

    isDashboardPopupVisible() {
      var popup = this.activeDashboardPopupOrigin();
      return !!(popup.active && popup.ready && popup.title);
    },

    dashboardPopupOverlayClass() {
      var popup = this.activeDashboardPopupOrigin();
      var service = this.dashboardPopupService();
      if (service && typeof service.overlayClass === 'function') {
        return service.overlayClass(popup, 'fogged-glass');
      }
      return {
        'is-visible': !!(popup.active && popup.ready && popup.title),
        'is-side-top': popup.side === 'top',
        'is-side-bottom': popup.side === 'bottom',
        'is-side-left': popup.side === 'left',
        'is-side-right': popup.side === 'right',
        'is-inline-away-left': popup.inline_away === 'left',
        'is-inline-away-right': popup.inline_away === 'right',
        'is-inline-away-center': popup.inline_away !== 'left' && popup.inline_away !== 'right',
        'is-block-away-top': popup.block_away === 'top',
        'is-block-away-bottom': popup.block_away === 'bottom',
        'is-block-away-center': popup.block_away !== 'top' && popup.block_away !== 'bottom',
        'is-unread': !!popup.unread
      };
    },

    dashboardPopupOverlayStyle() {
      var popup = this.activeDashboardPopupOrigin();
      var service = this.dashboardPopupService();
      if (service && typeof service.overlayStyle === 'function') {
        return service.overlayStyle(popup);
      }
      if (!popup.active || !popup.ready) return 'left:-9999px;top:-9999px;';
      return 'left:' + Math.round(Number(popup.left || 0)) + 'px;top:' + Math.round(Number(popup.top || 0)) + 'px;';
    },

    updateSidebarScrollIndicators() {
      var refs = this.$refs || {};
      var navState = this._computeScrollHintState(refs.sidebarNav);
      this.sidebarHasOverflowAbove = !!navState.above;
      this.sidebarHasOverflowBelow = !!navState.below;
      var chatState = this._computeScrollHintState(refs.chatSidebarList);
      this.chatSidebarHasOverflowAbove = !!chatState.above;
      this.chatSidebarHasOverflowBelow = !!chatState.below;
    },
    scheduleSidebarScrollIndicators() {
      if (this._sidebarScrollIndicatorRaf) return;
      var self = this;
      this._sidebarScrollIndicatorRaf = requestAnimationFrame(function() {
        self._sidebarScrollIndicatorRaf = 0;
        self.updateSidebarScrollIndicators();
        if (typeof self.maybeAnimateChatSidebarRows === 'function') {
          self.maybeAnimateChatSidebarRows();
        }
      });
    },
    shellAppStoreBridge() {
      return infringShellAppStoreBridge();
    },
    notifyShellAppStore(reason) {
      var bridge = this.shellAppStoreBridge();
      if (bridge && typeof bridge.notify === 'function') bridge.notify(reason || 'shell_root_changed');
    },
    getAppStore() {
      var bridge = this.shellAppStoreBridge();
      if (bridge && typeof bridge.current === 'function') {
        var bridgedStore = bridge.current();
        if (bridgedStore && typeof bridgedStore === 'object') return bridgedStore;
      }
      return (typeof window !== 'undefined' && window.InfringApp && typeof window.InfringApp === 'object')
        ? window.InfringApp
        : null;
    },
    get agents() {
      var store = this.getAppStore();
      return store && Array.isArray(store.agents) ? store.agents : [];
    },
    isSystemSidebarThread(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread === true) return true;
      var id = String(agent.id || '').trim().toLowerCase();
      if (id === 'system') return true;
      var role = String(agent.role || '').trim().toLowerCase();
      return role === 'system';
    },
    isSidebarArchivedAgent(agent) {
      if (!agent || typeof agent !== 'object') return false;
      var store = this.getAppStore();
      if (store && typeof store.isArchivedLikeAgent === 'function') return store.isArchivedLikeAgent(agent);
      if (Object.prototype.hasOwnProperty.call(agent, 'sidebar_archived')) return !!agent.sidebar_archived;
      return !!agent.archived;
    },
    isReservedSystemEmoji(rawEmoji) {
      var normalized = String(rawEmoji || '').replace(/\uFE0F/g, '').trim();
      return normalized === '⚙';
    },
    sanitizeSidebarAgentRow(agent) {
      if (!agent || typeof agent !== 'object') return agent;
      var row = Object.assign({}, agent);
      var identity = Object.assign({}, (row.identity && typeof row.identity === 'object') ? row.identity : {});
      if (this.isSystemSidebarThread(row)) {
        row.id = 'system';
        row.name = 'System';
        row.is_system_thread = true;
        row.role = 'system';
        identity.emoji = '\u2699\ufe0f';
        row.identity = identity;
        return row;
      }
      if (this.isReservedSystemEmoji(identity.emoji)) {
        identity.emoji = '';
      }
      row.identity = identity;
      return row;
    },
    persistChatSidebarTopologyOrder() {
      var seen = {};
      var out = [];
      (this.chatSidebarTopologyOrder || []).forEach(function(id) {
        var key = String(id || '').trim();
        if (!key || seen[key]) return;
        seen[key] = true;
        out.push(key);
      });
      this.chatSidebarTopologyOrder = out;
      try {
        localStorage.setItem('infring-chat-sidebar-topology-order', JSON.stringify(out));
      } catch(_) {}
    },
    chatSidebarCanReorderTopology() {
      return String(this.chatSidebarSortMode || '').toLowerCase() === 'topology';
    },
    startChatSidebarTopologyDrag(agent, ev) {
      if (!this.chatSidebarCanReorderTopology() || !agent || !agent.id) return;
      this.syncChatSidebarTopologyOrderFromAgents();
      this.chatSidebarDragAgentId = String(agent.id);
      this.chatSidebarDropTargetId = '';
      this.chatSidebarDropAfter = false;
      if (ev && ev.dataTransfer) {
        ev.dataTransfer.effectAllowed = 'move';
        ev.dataTransfer.setData('text/plain', this.chatSidebarDragAgentId);
      }
    },
    handleChatSidebarTopologyDragOver(agent, ev) {
      if (!this.chatSidebarCanReorderTopology() || !this.chatSidebarDragAgentId || !agent || !agent.id) return;
      if (ev) {
        ev.preventDefault();
        if (ev.dataTransfer) ev.dataTransfer.dropEffect = 'move';
      }
      var targetId = String(agent.id);
      var dropAfter = false;
      if (ev && ev.currentTarget && typeof ev.clientY === 'number' && typeof ev.currentTarget.getBoundingClientRect === 'function') {
        var rect = ev.currentTarget.getBoundingClientRect();
        dropAfter = ev.clientY > (rect.top + (rect.height / 2));
      }
      this.chatSidebarDropAfter = !!dropAfter;
      this.chatSidebarDropTargetId = targetId === this.chatSidebarDragAgentId ? '' : targetId;
    },
    handleChatSidebarTopologyDrop(agent, ev) {
      if (ev) ev.preventDefault();
      if (!this.chatSidebarCanReorderTopology() || !agent || !agent.id) return this.endChatSidebarTopologyDrag();
      var dragId = String(this.chatSidebarDragAgentId || '').trim();
      if (!dragId && ev && ev.dataTransfer) dragId = String(ev.dataTransfer.getData('text/plain') || '').trim();
      var targetId = String(agent.id).trim();
      if (!dragId || !targetId || dragId === targetId) return this.endChatSidebarTopologyDrag();
      this.syncChatSidebarTopologyOrderFromAgents();
      var order = (this.chatSidebarTopologyOrder || []).slice();
      var fromIndex = order.indexOf(dragId);
      var targetIndex = order.indexOf(targetId);
      if (fromIndex < 0 || targetIndex < 0) return this.endChatSidebarTopologyDrag();
      var dropAfter = false;
      if (ev && ev.currentTarget && typeof ev.clientY === 'number' && typeof ev.currentTarget.getBoundingClientRect === 'function') {
        var rect = ev.currentTarget.getBoundingClientRect();
        dropAfter = ev.clientY > (rect.top + (rect.height / 2));
      }
      order.splice(fromIndex, 1);
      if (fromIndex < targetIndex) targetIndex -= 1;
      if (dropAfter) targetIndex += 1;
      if (targetIndex < 0) targetIndex = 0;
      if (targetIndex > order.length) targetIndex = order.length;
      order.splice(targetIndex, 0, dragId);
      this.chatSidebarTopologyOrder = order;
      this.persistChatSidebarTopologyOrder();
      this.endChatSidebarTopologyDrag();
      this.scheduleSidebarScrollIndicators();
    },
    endChatSidebarTopologyDrag() {
      this.chatSidebarDragAgentId = '';
      this.chatSidebarDropTargetId = '';
      this.chatSidebarDropAfter = false;
    },
    get chatSidebarAgents() {
      var list = (this.agents || []).slice();
      var self = this;
      var pendingFreshId = String((this.getAppStore() && this.getAppStore().pendingFreshAgentId) || '').trim();
      list = list.filter(function(agent) {
        if (!agent || !agent.id) return false;
        if (pendingFreshId && String(agent.id || '') === pendingFreshId) return false;
        if (self.isSidebarArchivedAgent(agent)) return false;
        return true;
      });
      list.sort(function(a, b) {
        return self.chatSidebarSortComparator(a, b);
      });
      if (this.chatSidebarCanReorderTopology() && Array.isArray(this.chatSidebarTopologyOrder) && this.chatSidebarTopologyOrder.length) {
        var rank = {};
        this.chatSidebarTopologyOrder.forEach(function(id, idx) {
          var key = String(id || '').trim();
          if (!key || rank[key] != null) return;
          rank[key] = idx;
        });
        list.sort(function(a, b) {
          var aId = String((a && a.id) || '');
          var bId = String((b && b.id) || '');
          var hasA = Object.prototype.hasOwnProperty.call(rank, aId);
          var hasB = Object.prototype.hasOwnProperty.call(rank, bId);
          if (hasA && hasB && rank[aId] !== rank[bId]) return rank[aId] - rank[bId];
          if (hasA && !hasB) return -1;
          if (!hasA && hasB) return 1;
          return self.chatSidebarSortComparator(a, b);
        });
      }
      return list.map(function(agent) {
        return self.sanitizeSidebarAgentRow(agent);
      });
    },
    get chatSidebarRows() {
      if (this.chatSidebarDragActive && Array.isArray(this._chatSidebarDragRowsCache)) {
        return this._chatSidebarDragRowsCache;
      }
      var query = String(this.chatSidebarQuery || '').trim();
      var rows;
      if (!query) rows = this.chatSidebarAgents || [];
      else if (Array.isArray(this.chatSidebarSearchResults) && this.chatSidebarSearchResults.length) rows = this.chatSidebarSearchResults;
      else rows = [];
      if (this.chatSidebarDragActive) {
        this._chatSidebarDragRowsCache = Array.isArray(rows) ? rows.slice() : [];
      } else {
        this._chatSidebarDragRowsCache = null;
      }
      return rows;
    },
    chatSidebarDragRenderWindow(rows) {
      var sourceRows = Array.isArray(rows) ? rows : [];
      var total = sourceRows.length;
      var maxRows = Math.max(1, Math.floor(Number(this._chatSidebarDragRenderMaxRows || 10)));
      if (!this.chatSidebarDragActive || total <= maxRows) {
        return { virtualized: false, start: 0, end: total, padTop: 0, padBottom: 0 };
      }
      var refs = this.$refs || {};
      var nav = refs.sidebarNav || null;
      var rowHeight = Math.max(1, Math.floor(Number(this._chatSidebarDragRenderRowHeight || 56)));
      var scrollTop = nav ? Math.max(0, Number(nav.scrollTop || 0)) : 0;
      var start = Math.max(0, Math.floor(scrollTop / rowHeight));
      if (start > (total - maxRows)) start = Math.max(0, total - maxRows);
      var end = Math.min(total, start + maxRows);
      return {
        virtualized: true,
        start: start,
        end: end,
        padTop: start * rowHeight,
        padBottom: Math.max(0, (total - end) * rowHeight)
      };
    },
    get chatSidebarVirtualized() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).virtualized;
    },
    get chatSidebarVirtualPadTop() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).padTop;
    },
    get chatSidebarVirtualPadBottom() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      return this.chatSidebarDragRenderWindow(rows).padBottom;
    },
    get chatSidebarVisibleRows() {
      var rows = Array.isArray(this.chatSidebarRows) ? this.chatSidebarRows : [];
      var window = this.chatSidebarDragRenderWindow(rows);
      if (!window.virtualized) return rows;
      return rows.slice(window.start, window.end);
    },
    chatSidebarHasMoreRows() { return false; },
    showMoreChatSidebarRows() { this.scheduleSidebarScrollIndicators(); },
    init() {
      var self = this;
      var appStoreBridge = typeof this.shellAppStoreBridge === 'function' ? this.shellAppStoreBridge() : null;
      if (appStoreBridge && typeof appStoreBridge.registerShellRoot === 'function') {
        appStoreBridge.registerShellRoot(this);
      }
      this._bootSplashStartedAt = Date.now();
      this.bootSplashVisible = true;
      this.applyOverlayGlassTemplate('simple-glass', true);
      if (typeof this.resetBootProgress === 'function') this.resetBootProgress();
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('splash_visible');
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      if (this._bootSplashMaxTimer) {
        clearTimeout(this._bootSplashMaxTimer);
        this._bootSplashMaxTimer = 0;
      }
      this._bootSplashMaxTimer = window.setTimeout(function() {
        self.releaseBootSplash(true);
      }, Number(this._bootSplashMaxMs || 5000));
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
      this.syncAgentChatsSectionForPage = function() {
        this.agentChatsSectionCollapsed = false;
      };
      this.toggleAgentChatsSection = function() {
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

      this.pollStatus();
      var initStore = this.getAppStore();
      if (initStore && typeof initStore.checkOnboarding === 'function') initStore.checkOnboarding();
      if (initStore && typeof initStore.checkAuth === 'function') initStore.checkAuth();
      if (!this._dashboardClockTimer) this._dashboardClockTimer = setInterval(function() { self.clockTick = Date.now(); }, 1000);
      if (!this._dashboardStatusTimer) this._dashboardStatusTimer = setInterval(function() {
        if (document && document.hidden) return;
        self.pollStatus();
      }, 10000);
      if (!this._dashboardVisibilityHandler && document) {
        this._dashboardVisibilityHandler = function() { if (!document.hidden) self.pollStatus(); };
        document.addEventListener('visibilitychange', this._dashboardVisibilityHandler);
      }
      window.addEventListener('resize', function() {
        self.scheduleSidebarScrollIndicators();
      });
      this.$nextTick(function() {
        self.scheduleSidebarScrollIndicators();
      });
    },
    releaseBootSplash(force) {
      if (!this.bootSplashVisible) return;
      var now = Date.now();
      var elapsed = Math.max(0, now - Number(this._bootSplashStartedAt || now));
      var minRemain = Math.max(0, Number(this._bootSplashMinMs || 0) - elapsed);
      var store = this.getAppStore();
      var ready = !!force || !store || store.booting === false;
      if (!ready) return;
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('releasing', { bootStage: store && store.bootStage });
      if (this._bootSplashHideTimer) {
        clearTimeout(this._bootSplashHideTimer);
        this._bootSplashHideTimer = 0;
      }
      var self = this;
      var progressNow = typeof this.bootProgressClamped === 'function'
        ? this.bootProgressClamped(this.bootProgressPercent)
        : Math.max(0, Math.min(100, Number(this.bootProgressPercent || 0)));
      var completionAnimationDelayMs = progressNow < 100 ? 500 : 0;
      var hideDelayMs = Math.max(minRemain, completionAnimationDelayMs);
      if (typeof this.setBootProgressEvent === 'function') this.setBootProgressEvent('complete', { bootStage: store && store.bootStage });
      if (hideDelayMs <= 0) {
        this.bootSplashVisible = false;
        if (this._bootSplashMaxTimer) {
          clearTimeout(this._bootSplashMaxTimer);
          this._bootSplashMaxTimer = 0;
        }
        return;
      }
      this._bootSplashHideTimer = window.setTimeout(function() {
        self.bootSplashVisible = false;
        self._bootSplashHideTimer = 0;
        if (self._bootSplashMaxTimer) {
          clearTimeout(self._bootSplashMaxTimer);
          self._bootSplashMaxTimer = 0;
        }
      }, hideDelayMs);
    },
    normalizeNavigablePage(pageId) {
      var raw = String(pageId || '').trim().toLowerCase();
      if (!raw) return 'chat';
      var aliases = {
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
      return aliases[raw] || raw;
    },
    isKnownNavigablePage(pageId) {
      var normalized = this.normalizeNavigablePage(pageId);
      return ['chat','agents','sessions','approvals','comms','workflows','scheduler','channels','eyes','skills','hands','overview','analytics','logs','runtime','settings','wizard']
        .indexOf(normalized) >= 0;
    },
    syncPageHistory(nextPage) {
      var next = this.normalizeNavigablePage(nextPage);
      if (!this.isKnownNavigablePage(next)) return;
      var current = this.normalizeNavigablePage(this._navCurrentPage || this.page || '');
      var action = String(this._navHistoryAction || '').trim().toLowerCase();
      var back = Array.isArray(this.navBackStack) ? this.navBackStack.slice() : [];
      var forward = Array.isArray(this.navForwardStack) ? this.navForwardStack.slice() : [];
      var cap = Number(this._navHistoryCap || 48);
      if (!Number.isFinite(cap) || cap < 8) cap = 48;
      var trim = function(list) {
        return list.length > cap ? list.slice(list.length - cap) : list;
      };
      if (!current || !this.isKnownNavigablePage(current)) {
        this._navCurrentPage = next;
        this._navHistoryAction = '';
        return;
      }
      if (next === current) {
        this._navCurrentPage = next;
        this._navHistoryAction = '';
        return;
      }
      if (action === 'back') {
        if (forward.length === 0 || forward[forward.length - 1] !== current) forward.push(current);
      } else if (action === 'forward') {
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
      } else if (back.length > 0 && back[back.length - 1] === next) {
        back.pop();
        if (forward.length === 0 || forward[forward.length - 1] !== current) forward.push(current);
      } else if (forward.length > 0 && forward[forward.length - 1] === next) {
        forward.pop();
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
      } else {
        if (back.length === 0 || back[back.length - 1] !== current) back.push(current);
        forward = [];
      }
      this.navBackStack = trim(back);
      this.navForwardStack = trim(forward);
      this._navCurrentPage = next;
      this._navHistoryAction = '';
    },
    canNavigateBack() {
      return Array.isArray(this.navBackStack) && this.navBackStack.length > 0;
    },
    canNavigateForward() {
      return Array.isArray(this.navForwardStack) && this.navForwardStack.length > 0;
    },
    navigateBackPage() {
      if (!this.canNavigateBack()) return;
      var back = this.navBackStack.slice();
      var target = this.normalizeNavigablePage(back.pop());
      this.navBackStack = back;
      this._navHistoryAction = 'back';
      if (!target || target === this.normalizeNavigablePage(this.page)) {
        this._navHistoryAction = '';
        return;
      }
      this.navigate(target);
    },
    navigateForwardPage() {
      if (!this.canNavigateForward()) return;
      var forward = this.navForwardStack.slice();
      var target = this.normalizeNavigablePage(forward.pop());
      this.navForwardStack = forward;
      this._navHistoryAction = 'forward';
      if (!target || target === this.normalizeNavigablePage(this.page)) {
        this._navHistoryAction = '';
        return;
      }
      this.navigate(target);
    },
    navigate(p) {
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      if (String(p || '') !== 'chat') {
        var store = this.getAppStore();
        var pendingId = String((store && store.pendingFreshAgentId) || '').trim();
        var activeId = String((store && store.activeAgentId) || '').trim();
        if (pendingId) {
          if (store) {
            store.pendingFreshAgentId = null;
            store.pendingAgent = null;
            if (pendingId === activeId) {
              if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
              else store.activeAgentId = null;
            }
          }
          this.chatSidebarTopologyOrder = (this.chatSidebarTopologyOrder || []).filter(function(id) {
            return String(id || '').trim() !== pendingId;
          });
          this.persistChatSidebarTopologyOrder();
          InfringAPI.del('/api/agents/' + encodeURIComponent(pendingId)).catch(function() {});
          if (store && typeof store.refreshAgents === 'function') setTimeout(function() { store.refreshAgents({ force: true }).catch(function() {}); }, 0);
        }
      }
      this.page = p;
      if (typeof this.syncAgentChatsSectionForPage === 'function') {
        this.syncAgentChatsSectionForPage(p);
      }
      if (typeof this.notifyShellAppStore === 'function') this.notifyShellAppStore('navigate');
      window.location.hash = p;

      this.mobileMenuOpen = false;
    },
    setTheme(mode) {
      this.beginInstantThemeFlip();
      this.themeMode = mode;
      localStorage.setItem('infring-theme-mode', mode);
      if (mode === 'system') {
        this.theme = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
      } else {
        this.theme = mode;
      }
    },
    isChatSidebarSearchActive() {
      return String(this.chatSidebarQuery || '').trim().length > 0;
    },
    clearChatSidebarSearch() {
      if (this._chatSidebarSearchTimer) { clearTimeout(this._chatSidebarSearchTimer); this._chatSidebarSearchTimer = 0; }
      this.chatSidebarSearchSeq = Number(this.chatSidebarSearchSeq || 0) + 1;
      this.chatSidebarSearchLoading = false;
      this.chatSidebarSearchError = '';
      this.chatSidebarSearchResults = [];
      this.scheduleSidebarScrollIndicators();
    },
    onChatSidebarQueryInput(value) {
      this.chatSidebarQuery = String(value || '');
      this.chatSidebarVisibleCount = Math.max(1, Math.floor(Number(this.chatSidebarVisibleBase || 7)));
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) {
        this.clearChatSidebarSearch();
        return;
      }
      this.scheduleChatSidebarSearch();
    },
    scheduleChatSidebarSearch() {
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) { this.clearChatSidebarSearch(); return; }
      if (this._chatSidebarSearchTimer) { clearTimeout(this._chatSidebarSearchTimer); this._chatSidebarSearchTimer = 0; }
      var self = this;
      var seq = Number(this.chatSidebarSearchSeq || 0) + 1;
      this.chatSidebarSearchSeq = seq;
      this.chatSidebarSearchLoading = true;
      this.chatSidebarSearchError = '';
      this._chatSidebarSearchTimer = setTimeout(function() { self._chatSidebarSearchTimer = 0; self.runChatSidebarSearch(seq); }, 140);
    },
    async runChatSidebarSearch(seq) {
      var token = Number(seq || 0);
      var currentToken = Number(this.chatSidebarSearchSeq || 0);
      if (token !== currentToken) return;
      var query = String(this.chatSidebarQuery || '').trim();
      if (!query) {
        this.clearChatSidebarSearch();
        return;
      }
      try {
        var path = '/api/search/conversations?q=' + encodeURIComponent(query) + '&limit=80';
        var payload = await InfringAPI.get(path);
        if (token !== Number(this.chatSidebarSearchSeq || 0)) return;
        var self = this;
        var serverRows = payload && Array.isArray(payload.sidebar_rows) ? payload.sidebar_rows : null;
        if (serverRows && serverRows.length) {
          this.chatSidebarSearchResults = serverRows.filter(function(agent) {
            return !self.isSidebarArchivedAgent(agent);
          }).map(function(agent) {
            return self.sanitizeSidebarAgentRow(agent);
          });
          this.chatSidebarSearchError = '';
          return;
        }
        var quickRows = payload && Array.isArray(payload.quick_actions) ? payload.quick_actions : [];
        this.chatSidebarSearchResults = quickRows.filter(function(agent) {
          return !self.isSidebarArchivedAgent(agent);
        }).map(function(agent) {
          return self.sanitizeSidebarAgentRow(agent);
        });
        this.chatSidebarSearchError = '';
      } catch (e) {
        if (token !== Number(this.chatSidebarSearchSeq || 0)) return;
        this.chatSidebarSearchResults = [];
        this.chatSidebarSearchError = String(e && e.message ? e.message : 'search_failed');
      } finally {
        if (token === Number(this.chatSidebarSearchSeq || 0)) {
          this.chatSidebarSearchLoading = false;
        }
        this.scheduleSidebarScrollIndicators();
      }
    },
    overlayGlassTemplateNormalized(modeRaw) {
      var mode = String(modeRaw || '').trim().toLowerCase();
      if (mode === 'simple-glass') return 'simple-glass';
      if (mode === 'fogged-glass') return 'fogged-glass';
      if (mode === 'warped-glass' || mode === 'magnified-glass') return 'warped-glass';
      if (mode === 'liquid-glass') return 'fogged-glass';
      return 'simple-glass';
    },
    applyOverlayGlassTemplate(modeRaw, persistRaw) {
      var mode = this.overlayGlassTemplateNormalized(modeRaw);
      this.overlayGlassTemplate = mode;
      var persist = persistRaw !== false;
      if (document && document.documentElement) {
        try {
          document.documentElement.setAttribute('data-overlay-glass-template', mode);
        } catch (_) {}
      }
      if (persist) {
        try { localStorage.setItem('infring-overlay-glass-template', mode); } catch (_) {}
      }
      return mode;
    },
    uiBackgroundTemplateNormalized(modeRaw) {
      var service = this.taskbarDockService ? this.taskbarDockService() : infringTaskbarDockService();
      if (service && typeof service.normalizeBackgroundTemplate === 'function') return service.normalizeBackgroundTemplate(modeRaw);
      var mode = String(modeRaw || '').trim().toLowerCase();
      if (mode === 'unsplash-paper') return 'light-wood';
      if (mode === 'default-grid') return 'default-grid';
      if (mode === 'light-wood') return 'light-wood';
      if (mode === 'sand') return 'sand';
      return 'sand';
    },
    applyUiBackgroundTemplate(modeRaw, persistRaw) {
      var mode = this.uiBackgroundTemplateNormalized(modeRaw);
      this.uiBackgroundTemplate = mode;
      var persist = persistRaw !== false;
      if (document && document.documentElement) {
        try {
          document.documentElement.setAttribute('data-ui-background-template', mode);
        } catch (_) {}
      }
      if (persist) {
        try {
          var service = this.taskbarDockService ? this.taskbarDockService() : infringTaskbarDockService();
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
    },
    beginInstantThemeFlip() {
      var self = this;
      var body = document && document.body ? document.body : null;
      if (!body) return;
      body.classList.add('theme-switching');
      // Force style flush so no-transition styles are applied before theme variables swap.
      void body.offsetHeight;
      if (this._themeSwitchReset) {
        clearTimeout(this._themeSwitchReset);
      }
      this._themeSwitchReset = window.setTimeout(function() {
        body.classList.remove('theme-switching');
        self._themeSwitchReset = 0;
      }, 260);
    },
    toggleTheme() {
      var modes = ['light', 'system', 'dark'];
      var next = modes[(modes.indexOf(this.themeMode) + 1) % modes.length];
      this.setTheme(next);
    },
    toggleSidebar() {
      if (typeof this.shouldSuppressSidebarToggle === 'function' && this.shouldSuppressSidebarToggle()) return;
      var nextCollapsed = !this.sidebarCollapsed;
      var resolveMessagesHost = function() {
        var nodes = document.querySelectorAll('#messages');
        for (var ni = 0; ni < nodes.length; ni++) if (nodes[ni] && nodes[ni].offsetParent !== null) return nodes[ni];
        return nodes && nodes.length ? nodes[0] : null;
      };
      var captureMessageBottomAnchor = function() {
        var host = resolveMessagesHost();
        if (!host || host.offsetParent === null) return null;
        var hostRect = host.getBoundingClientRect();
        var input = document.getElementById('msg-input');
        var alignY = hostRect.bottom;
        if (input && input.offsetParent !== null) {
          var inputRect = input.getBoundingClientRect();
          if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
        }
        var rows = host.querySelectorAll('.chat-message-block[id], .chat-message-block .message[id]');
        var best = null;
        var bestDiff = Number.POSITIVE_INFINITY;
        for (var i = 0; i < rows.length; i++) {
          var row = rows[i];
          if (!row || row.offsetParent === null) continue;
          var rect = row.getBoundingClientRect();
          if (rect.bottom < (hostRect.top - 40) || rect.top > (hostRect.bottom + 40)) continue;
          var diff = Math.abs(rect.bottom - alignY);
          if (diff < bestDiff) { bestDiff = diff; best = row; }
        }
        return best && best.id ? { id: String(best.id) } : null;
      };
      if (nextCollapsed) this._sidebarChatAnchorForExpand = captureMessageBottomAnchor();
      this.sidebarCollapsed = nextCollapsed;
      localStorage.setItem('infring-sidebar', this.sidebarCollapsed ? 'collapsed' : 'expanded');
      // Always clear stale sidebar popup when toggling sidebar state.
      this.hideDashboardPopupBySource('sidebar');
      if (!nextCollapsed) {
        var anchor = (this._sidebarChatAnchorForExpand && this._sidebarChatAnchorForExpand.id)
          ? this._sidebarChatAnchorForExpand
          : captureMessageBottomAnchor();
        this._sidebarChatAnchorForExpand = null;
        var passes = 4;
        var restoreAnchor = function() {
          var host = resolveMessagesHost();
          if (!host || host.offsetParent === null || !anchor || !anchor.id) return;
          var row = document.getElementById(anchor.id);
          if (!row || !host.contains(row) || row.offsetParent === null) return;
          var hostRect = host.getBoundingClientRect();
          var input = document.getElementById('msg-input');
          var alignY = hostRect.bottom;
          if (input && input.offsetParent !== null) {
            var inputRect = input.getBoundingClientRect();
            if (inputRect.top > hostRect.top && inputRect.top < (hostRect.bottom + 140)) alignY = inputRect.top;
          }
          var alignOffset = Math.max(0, Math.min(Math.max(0, Number(host.clientHeight || 0)), Math.round(alignY - hostRect.top)));
          var rowBottom = Number(row.offsetTop || 0) + Math.max(0, Number(row.offsetHeight || 0));
          var maxTop = Math.max(0, Number(host.scrollHeight || 0) - Math.max(0, Number(host.clientHeight || 0)));
          var nextTop = Math.max(0, Math.min(maxTop, Math.round(rowBottom - alignOffset)));
          host.scrollTop = nextTop;
          if (passes-- > 1 && typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
          try { host.dispatchEvent(new Event('scroll')); } catch (_) {}
        };
        if (typeof requestAnimationFrame === 'function') requestAnimationFrame(restoreAnchor);
        else setTimeout(restoreAnchor, 0);
      }
      this.scheduleSidebarScrollIndicators();
    },
    runtimeFacadeHealthSummary() {
      var summary = this.healthSummary && typeof this.healthSummary === 'object' ? this.healthSummary : null;
      if (!summary) return null;
      var loadedAt = Number(this._healthSummaryLoadedAt || 0);
      if (loadedAt > 0 && (Date.now() - loadedAt) > 60000) return null;
      return summary;
    },
    runtimeFacadeState() {
      var store = this.getAppStore();
      var conn = this.normalizeConnectionIndicatorState(
        this.connectionIndicatorState ||
        ((store && store.connectionState) || this.connectionState || '')
      );
      if (conn === 'connecting') return 'connecting';
      if (conn === 'disconnected') return this.runtimeFacadeHealthSummary() ? 'connecting' : 'down';
      if (this.runtimeEtaSeconds() > 0) return 'active';
      return 'connected';
    },
    runtimeFacadeClass() {
      var state = this.runtimeFacadeState();
      if (state === 'connected' || state === 'active') return 'health-ok';
      if (state === 'connecting') return 'health-connecting';
      return 'health-down';
    },
    runtimeFacadeLabel() {
      var state = this.runtimeFacadeState();
      if (state === 'active') return 'Active';
      if (state === 'connected') {
        var store = this.getAppStore();
        var health = this.runtimeFacadeHealthSummary();
        var agents = ((store && store.agents && store.agents.length) || (store && store.agentCount) || this.agentCount || Number(health && health.agent_count || 0) || Number(health && health.agents && health.agents.length || 0));
        return String(agents) + ' agents';
      }
      if (state === 'connecting' && this.runtimeFacadeHealthSummary()) return 'Reconnecting...';
      if (state === 'connecting') return 'Connecting...';
      return 'Disconnected';
    },
    runtimeFacadeDisplayLabel() {
      var label = String(this.runtimeFacadeLabel() || '').trim();
      if (!label) return '';
      return label.replace(/\s+agents?$/i, '');
    },
    runtimeResponseP95Ms() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) {
        var health = this.runtimeFacadeHealthSummary();
        var durationMs = Number(health && health.durationMs);
        return Number.isFinite(durationMs) && durationMs >= 0 ? Math.round(durationMs) : null;
      }
      var facadeP95 = Number(runtime.facade_response_p95_ms);
      if (Number.isFinite(facadeP95) && facadeP95 > 0) return Math.round(facadeP95);
      var p95 = Number(runtime.receipt_latency_p95_ms);
      if (Number.isFinite(p95) && p95 > 0) return Math.round(p95);
      var p99 = Number(runtime.receipt_latency_p99_ms);
      if (Number.isFinite(p99) && p99 > 0) return Math.round(p99);
      return null;
    },
    runtimeConfidencePercent() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return this.runtimeFacadeHealthSummary() ? 92 : 80;
      var facadeConfidence = Number(runtime.facade_confidence_percent);
      if (Number.isFinite(facadeConfidence) && facadeConfidence > 0) {
        return Math.max(10, Math.min(100, Math.round(facadeConfidence)));
      }

      var score = 100;
      var queueDepth = Number(runtime.queue_depth || 0);
      var stale = Number(runtime.cockpit_stale_blocks || 0);
      var gaps = Number(runtime.health_coverage_gap_count || 0);
      var conduitSignals = Number(runtime.conduit_signals || 0);
      var targetSignals = Math.max(1, Number(runtime.target_conduit_signals || 4));
      var benchmark = String(runtime.benchmark_sanity_cockpit_status || runtime.benchmark_sanity_status || 'unknown').toLowerCase();
      var spine = Number(runtime.spine_success_rate);

      if (queueDepth > 20) score -= Math.min(20, Math.floor((queueDepth - 20) / 2));
      if (stale > 0) score -= Math.min(20, stale * 2);
      if (gaps > 0) score -= Math.min(20, gaps * 6);
      if (conduitSignals < Math.max(3, Math.floor(targetSignals * 0.5))) score -= 12;
      if (benchmark === 'warn') score -= 8;
      if (benchmark === 'fail' || benchmark === 'error') score -= 20;
      if (Number.isFinite(spine)) {
        if (spine < 0.9) score -= 15;
        if (spine < 0.6) score -= 10;
      }

      score = Math.max(10, Math.min(100, Math.round(score)));
      return score;
    },
    runtimeEtaSeconds() {
      var store = this.getAppStore();
      var runtime = store && store.runtimeSync && typeof store.runtimeSync === 'object'
        ? store.runtimeSync
        : null;
      if (!runtime) return 0;
      var facadeEta = Number(runtime.facade_eta_seconds);
      if (Number.isFinite(facadeEta) && facadeEta >= 0) {
        return Math.max(0, Math.min(300, Math.round(facadeEta)));
      }
      var queueDepth = Math.max(0, Number(runtime.queue_depth || 0));
      if (queueDepth <= 0) return 0;
      // Conservative client-side estimate for "Active" mode only.
      return Math.max(1, Math.min(300, Math.ceil(queueDepth / 8)));
    },
    runtimeFacadeDetail() {
      var state = this.runtimeFacadeState();
      var store = this.getAppStore();
      var bootStage = String((store && store.bootStage) || '').trim();
      var stageSuffix = bootStage ? (' · ' + bootStage.replace(/_/g, ' ')) : '';
      if (state === 'connecting' && this.runtimeFacadeHealthSummary()) return 'HTTP health OK · reconnecting live runtime' + stageSuffix;
      if (state === 'connecting') return 'Establishing runtime link' + stageSuffix;
      if (state === 'down') return 'Runtime unavailable' + stageSuffix;
      var response = this.runtimeResponseP95Ms();
      var confidence = this.runtimeConfidencePercent();
      var health = this.runtimeFacadeHealthSummary();
      var agents = ((store && store.agents && store.agents.length) || (store && store.agentCount) || Number(health && health.agent_count || 0) || Number(health && health.agents && health.agents.length || 0));
      var base = 'Response ' + (response != null ? (response + 'ms') : '—') + ' · Confidence ' + confidence + '%';
      if (store && store.statusDegraded) {
        return base + ' · Status degraded' + stageSuffix;
      }
      if (state === 'active') {
        var eta = this.runtimeEtaSeconds();
        return (eta > 0 ? ('ETA ~' + eta + 's · ') : '') + base;
      }
      return base + ' · ' + agents + ' agent(s)';
    },
    runtimeFacadeTitle() {
      return this.runtimeFacadeLabel();
    },
    taskbarClockParts() {
      var tick = Number(this.clockTick || Date.now());
      var dt = new Date(tick);
      if (!Number.isFinite(dt.getTime())) dt = new Date();
      var dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
      var monthNames = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
      var dayName = dayNames[dt.getDay()] || '';
      var monthName = monthNames[dt.getMonth()] || '';
      var day = dt.getDate();
      var hours24 = dt.getHours();
      var minutes = dt.getMinutes();
      var suffix = hours24 >= 12 ? 'PM' : 'AM';
      var hours12 = hours24 % 12;
      if (hours12 === 0) hours12 = 12;
      var minuteText = minutes < 10 ? ('0' + minutes) : String(minutes);
      return {
        main: dayName + ' ' + monthName + ' ' + day + ' ' + hours12 + ':' + minuteText,
        meridiem: suffix
      };
    },
    taskbarClockMainLabel() {
      return this.taskbarClockParts().main;
    },
    taskbarClockMeridiemLabel() {
      return this.taskbarClockParts().meridiem;
    },
    taskbarClockLabel() {
      var parts = this.taskbarClockParts();
      return parts.main + ' ' + parts.meridiem;
    },
    toggleAgentChatsSidebar() {
      if (this.sidebarCollapsed) {
        this.sidebarCollapsed = false;
        localStorage.setItem('infring-sidebar', 'expanded');
      }
      this.hideDashboardPopupBySource('sidebar');
      this.scheduleSidebarScrollIndicators();
    },
    closeAgentChatsSidebar() {
      if (this.chatSidebarMode !== 'default') {
        this.chatSidebarMode = 'default';
        this.chatSidebarQuery = '';
        this.clearChatSidebarSearch();
      }
      this.confirmArchiveAgentId = '';
      this.scheduleSidebarScrollIndicators();
    },
    async applyBootChatSelection() {
      if (this.bootSelectionApplied) return;
      var store = this.getAppStore();
      if (!store || store.agentsLoading || !store.agentsHydrated) {
        return;
      }
      var rows = Array.isArray(store.agents) ? store.agents.slice() : [];
      if (!rows.length) {
        this.bootSelectionApplied = true;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
        else store.activeAgentId = null;
        this.navigate('chat');
        this.chatSidebarQuery = '';
        this.clearChatSidebarSearch();
        return;
      }
      var target = null;
      if (store.activeAgentId) {
        var saved = String(store.activeAgentId);
        target = rows.find(function(agent) { return agent && String(agent.id) === saved; }) || null;
      }
      if (!target) {
        rows.sort(function(a, b) {

// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
          return this.chatSidebarSortComparator(a, b);
        }.bind(this));
        target = rows.length ? rows[0] : null;
      }
      if (target && target.id) {
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(target.id);
        else store.activeAgentId = target.id;
      }
      this.bootSelectionApplied = true;
      this.navigate('chat');
      this.closeAgentChatsSidebar();
    },
    sidebarAgentSortTs(agent) {
      if (!agent) return 0;
      var serverTs = Number(agent.sidebar_sort_ts);
      if (Number.isFinite(serverTs) && serverTs > 0) return Math.round(serverTs);
      return 0;
    },
    chatSidebarTopologyKey(agent) {
      if (!agent || !agent.id) return 'z|~~~~|';
      var serverKey = String(agent.sidebar_topology_key || '').trim().toLowerCase();
      if (serverKey) return serverKey;
      return 'z|' + String(agent.id || '').trim().toLowerCase();
    },
    chatSidebarSortComparator(a, b) {
      var mode = String(this.chatSidebarSortMode || '').toLowerCase();
      if (mode === 'topology') {
        var topoA = this.chatSidebarTopologyKey(a);
        var topoB = this.chatSidebarTopologyKey(b);
        if (topoA < topoB) return -1;
        if (topoA > topoB) return 1;
      }
      var byTs = this.sidebarAgentSortTs(b) - this.sidebarAgentSortTs(a);
      if (byTs !== 0) return byTs;
      var aName = String((a && (a.name || a.id)) || '').toLowerCase();
      var bName = String((b && (b.name || b.id)) || '').toLowerCase();
      if (aName < bName) return -1;
      if (aName > bName) return 1;
      return 0;
    },
    syncChatSidebarTopologyOrderFromAgents() {
      var self = this;
      var pool = (this.agents || []).filter(function(agent) {
        if (!agent || !agent.id) return false;
        return !(typeof self.isSidebarArchivedAgent === 'function' && self.isSidebarArchivedAgent(agent));
      });
      pool.sort(function(a, b) {
        return self.chatSidebarSortComparator(a, b);
      });
      var liveIds = pool.map(function(agent) { return String(agent.id); });
      var liveSet = new Set(liveIds);
      var seen = {};
      var prior = Array.isArray(this.chatSidebarTopologyOrder) ? this.chatSidebarTopologyOrder : [];
      var next = [];
      prior.forEach(function(id) {
        var key = String(id || '').trim();
        if (!key || seen[key] || !liveSet.has(key)) return;
        seen[key] = true;
        next.push(key);
      });
      liveIds.forEach(function(id) {
        if (seen[id]) return;
        seen[id] = true;
        next.push(id);
      });
      var changed = next.length !== prior.length;
      if (!changed) changed = next.some(function(id, idx) { return id !== String(prior[idx] || ''); });
      if (changed) {
        this.chatSidebarTopologyOrder = next;
        this.persistChatSidebarTopologyOrder();
      }
    },
    setChatSidebarSortMode(mode) {
      var normalized = String(mode || '').trim().toLowerCase() === 'topology' ? 'topology' : 'age';
      this.chatSidebarSortMode = normalized;
      if (normalized === 'topology' && typeof this.syncChatSidebarTopologyOrderFromAgents === 'function') {
        this.syncChatSidebarTopologyOrderFromAgents();
      } else if (typeof this.endChatSidebarTopologyDrag === 'function') {
        this.endChatSidebarTopologyDrag();
      }
      try {
        localStorage.setItem('infring-chat-sidebar-sort-mode', normalized);
      } catch(_) {}
      this.scheduleSidebarScrollIndicators();
    },
    chatSidebarPreview(agent) {
      if (!agent) return { text: 'No messages yet', ts: 0, role: 'agent', has_tools: false, tool_state: '', tool_label: '', unread_response: false };
      if (agent.revive_recommended === true) {
        return {
          text: 'Open chat to revive',
          ts: this.sidebarAgentSortTs(agent),
          role: 'agent',
          has_tools: false,
          tool_state: '',
          tool_label: '',
          unread_response: false
        };
      }
      var isSystemThread = agent.is_system_thread === true || String(agent.id || '').toLowerCase() === 'system';
      var fallbackText = isSystemThread ? '' : 'No messages yet'; if (typeof this._isCollapsedHoverStatePlaceholderText === 'function' && this._isCollapsedHoverStatePlaceholderText(fallbackText)) fallbackText = '';
      var store = this.getAppStore();
      var preview = store && typeof store.getAgentChatPreview === 'function' ? store.getAgentChatPreview(agent.id) : null;
      var serverPreview = agent && agent.sidebar_preview && typeof agent.sidebar_preview === 'object' ? agent.sidebar_preview : null;
      if (serverPreview && typeof serverPreview === 'object') {
        var serverText = String(serverPreview.text || '').trim();
        return {
          text: serverText || fallbackText,
          ts: Number(serverPreview.ts || this.sidebarAgentSortTs(agent)) || this.sidebarAgentSortTs(agent),
          role: String(serverPreview.role || 'assistant'),
          has_tools: !!serverPreview.has_tools,
          tool_state: String(serverPreview.tool_state || ''),
          tool_label: String(serverPreview.tool_label || ''),
          unread_response: !!(preview && preview.unread_response)
        };
      }
      if (isSystemThread) {
        return {
          text: '',
          ts: preview && preview.ts ? preview.ts : this.sidebarAgentSortTs(agent),
          role: 'agent',
          has_tools: !!(preview && preview.has_tools),
          tool_state: preview && preview.tool_state ? preview.tool_state : '',
          tool_label: preview && preview.tool_label ? preview.tool_label : '',
          unread_response: !!(preview && preview.unread_response)
        };
      }
      return { text: fallbackText, ts: this.sidebarAgentSortTs(agent), role: 'agent', has_tools: false, tool_state: '', tool_label: '', unread_response: false };
    },
    sidebarDisplayEmoji(agent) {
      if (!agent) return '';
      var isSystem = this.isSystemSidebarThread && this.isSystemSidebarThread(agent);
      if (isSystem) return '\u2699\ufe0f';
      var emoji = String((agent.identity && agent.identity.emoji) || '').trim();
      if (this.isReservedSystemEmoji && this.isReservedSystemEmoji(emoji)) return '';
      return emoji;
    },
    async archiveAgentFromSidebar(agent) {
      if (!agent || !agent.id) return;
      var agentId = String(agent.id);
      if (typeof this.isSidebarArchivedAgent === 'function' && this.isSidebarArchivedAgent(agent)) return;
      this.confirmArchiveAgentId = '';
      var missingPurged = false;
      try {
        await InfringAPI.del('/api/agents/' + encodeURIComponent(agentId));
      } catch(e) {
        var msg = String(e && e.message ? e.message : '');
        if (msg.indexOf('agent_not_found') >= 0) {
          missingPurged = true;
        } else {
          InfringToast.error('Failed to archive agent: ' + (e && e.message ? e.message : 'unknown error'));
          return;
        }
      }
      this.syncChatSidebarTopologyOrderFromAgents();
      var store = this.getAppStore();
      if (store.activeAgentId === agent.id) {
        var next = this.chatSidebarAgents.length ? this.chatSidebarAgents[0] : null;
        if (next && next.id) {
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(next.id);
          else store.activeAgentId = next.id;
        } else {
          if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(null);
          else store.activeAgentId = null;
        }
      }
      await store.refreshAgents();
      if (missingPurged) {
        InfringToast.success('Removed stale agent "' + (agent.name || agent.id) + '"');
      } else {
        InfringToast.success('Archived "' + (agent.name || agent.id) + '"');
      }
      this.scheduleSidebarScrollIndicators();
    },
    async createSidebarAgentChat() {
      if (this.sidebarSpawningAgent) return;
      this.confirmArchiveAgentId = '';
      this.sidebarSpawningAgent = true;
      try {
        var res = await InfringAPI.post('/api/agents', {
          role: 'analyst'
        });
        var createdId = String((res && (res.id || res.agent_id)) || '').trim();
        if (!createdId) throw new Error('spawn_failed');
        var store = this.getAppStore();
        if (!store || typeof store.refreshAgents !== 'function') throw new Error('app_store_unavailable');
        await store.refreshAgents({ force: true });
        var authoritative = null;
        if (Array.isArray(store.agents)) {
          for (var ai = 0; ai < store.agents.length; ai++) {
            var row = store.agents[ai];
            if (row && String((row && row.id) || '') === createdId) {
              authoritative = row;
              break;
            }
          }
        }
        if (!authoritative) {
          try {
            authoritative = await InfringAPI.get('/api/agents/' + encodeURIComponent(createdId));
          } catch(_) {}
        }
        var createdSource = authoritative && typeof authoritative === 'object'
          ? Object.assign({}, res || {}, authoritative)
          : (res && typeof res === 'object' ? Object.assign({}, res) : {});
        var createdStatusState = String((createdSource && createdSource.sidebar_status_state) || '').trim().toLowerCase();
        if (createdStatusState !== 'active' && createdStatusState !== 'idle' && createdStatusState !== 'offline') {
          createdStatusState = '';
        }
        var createdStatusLabel = String((createdSource && createdSource.sidebar_status_label) || '').trim().toLowerCase();
        if (createdStatusLabel !== 'active' && createdStatusLabel !== 'idle' && createdStatusLabel !== 'offline') {
          createdStatusLabel = createdStatusState;
        }
        var createdFreshness = {
          source: String((createdSource && createdSource.sidebar_status_source) || ''),
          source_sequence: String((createdSource && createdSource.sidebar_status_source_sequence) || ''),
          age_seconds: Number((createdSource && createdSource.sidebar_status_age_seconds) || 0),
          stale: !!(createdSource && createdSource.sidebar_status_stale === true)
        };
        var created = Object.assign({}, createdSource, {
          id: createdId,
          agent_id: createdId,
          name: String((createdSource && createdSource.name) || createdId),
          role: String((createdSource && createdSource.role) || 'analyst'),
          identity: (createdSource && createdSource.identity && typeof createdSource.identity === 'object') ? createdSource.identity : {},
          avatar_url: String((createdSource && createdSource.avatar_url) || ''),
          state: String((createdSource && createdSource.state) || createdStatusLabel || createdStatusState || 'Running'),
          sidebar_status_state: createdStatusState || 'active',
          sidebar_status_label: createdStatusLabel || createdStatusState || 'active',
          sidebar_status_source: createdFreshness.source,
          sidebar_status_source_sequence: createdFreshness.source_sequence,
          sidebar_status_age_seconds: createdFreshness.age_seconds,
          sidebar_status_stale: createdFreshness.stale,
          sidebar_status_freshness: createdFreshness,
          model_name: String((createdSource && (createdSource.model_name || createdSource.runtime_model || '')) || ''),
          model_provider: String((createdSource && createdSource.model_provider) || ''),
          runtime_model: String((createdSource && createdSource.runtime_model) || ''),
          created_at: String((createdSource && createdSource.created_at) || new Date().toISOString())
        });
        this.syncChatSidebarTopologyOrderFromAgents();
        store.pendingAgent = created;
        store.pendingFreshAgentId = created.id;
        if (typeof store.setActiveAgentId === 'function') store.setActiveAgentId(created.id);
        else store.activeAgentId = created.id;
        this.navigate('chat');
        this.closeAgentChatsSidebar();
        InfringToast.success('Agent draft created. Complete initialization to launch.');
        this.scheduleSidebarScrollIndicators();
        // Keep draft agent hidden from rosters until launch completes.
      } catch(e) {
        InfringToast.error('Failed to create agent: ' + (e && e.message ? e.message : 'unknown error'));
      }
      this.sidebarSpawningAgent = false;
    },
    selectAgentChatFromSidebar(agent) {
      if (!agent || !agent.id) return;
      if (typeof this.hideDashboardPopupBySource === 'function') this.hideDashboardPopupBySource('sidebar');
      this.confirmArchiveAgentId = '';
      var quickAction = agent && agent._sidebar_quick_action && typeof agent._sidebar_quick_action === 'object' ? agent._sidebar_quick_action : null;
      if (quickAction) {
        var actionType = String(quickAction.type || '').trim().toLowerCase();
        if (actionType === 'copy_connect') {
          var checklist = 'Gateway connect checklist: open Settings, verify pairing or API token setup, and use HTTPS or localhost when device identity is required.';
          try { if (navigator && navigator.clipboard && typeof navigator.clipboard.writeText === 'function') navigator.clipboard.writeText(checklist).catch(function() {}); } catch(_) {}
          InfringToast.success('Copied connection checklist');
        }
        this.navigate(quickAction.page || 'chat');
        this.clearChatSidebarSearch();
        this.closeAgentChatsSidebar();
        this.scheduleSidebarScrollIndicators();
        return;
      }
      var store = this.getAppStore();
      var archived = typeof this.isSidebarArchivedAgent === 'function' && this.isSidebarArchivedAgent(agent);
      if (store && archived) {
        var pendingState = '';
        var rawSidebarStatusState = (typeof agent.sidebar_status_state === 'string')
          ? agent.sidebar_status_state
          : '';
        var rawSidebarStatusLabel = (typeof agent.sidebar_status_label === 'string')
          ? agent.sidebar_status_label
          : '';
        if (typeof this.agentStatusLabel === 'function') {
          pendingState = String(this.agentStatusLabel(agent) || '').trim().toLowerCase();
        }
        if (!pendingState) pendingState = 'offline';
        var pending = {
          id: String(agent.id),
          name: String(agent.name || agent.id),
          state: pendingState,
          archived: true,
          avatar_url: String(agent.avatar_url || '').trim(),
          sidebar_status_state: String(rawSidebarStatusState).trim().toLowerCase(),
          sidebar_status_label: String(rawSidebarStatusLabel).trim().toLowerCase(),
          sidebar_status_source: String(agent.sidebar_status_source || ''),
          sidebar_status_source_sequence: String(agent.sidebar_status_source_sequence || ''),
          sidebar_status_age_seconds: Number(agent.sidebar_status_age_seconds || 0),
          sidebar_status_stale: !!(agent.sidebar_status_stale === true),
          sidebar_status_freshness: agent.sidebar_status_freshness && typeof agent.sidebar_status_freshness === 'object'
            ? agent.sidebar_status_freshness
            : {
                source: String(agent.sidebar_status_source || ''),
                source_sequence: String(agent.sidebar_status_source_sequence || ''),
                age_seconds: Number(agent.sidebar_status_age_seconds || 0),
                stale: !!(agent.sidebar_status_stale === true)
              },
          identity: { emoji: String((agent.identity && agent.identity.emoji) || '') },
          role: String(agent.role || 'analyst')
        };
        store.pendingAgent = pending;
        store.pendingFreshAgentId = null;
      }
      if (store && typeof store.setActiveAgentId === 'function') store.setActiveAgentId(agent.id);
      else if (store) store.activeAgentId = agent.id;
      this.navigate('chat');
      this.closeAgentChatsSidebar();
      this.scheduleSidebarScrollIndicators();
      if (agent.revive_recommended === true) {
        var reviveId = String(agent.id || '').trim();
        if (reviveId) {
          InfringAPI.post('/api/agents/' + encodeURIComponent(reviveId) + '/revive', {
            reason: 'sidebar_contract_revival'
          }).then(function() {
            if (store && typeof store.refreshAgents === 'function') {
              store.refreshAgents({ force: true }).catch(function() {});
            }
          }).catch(function() {});
        }
      }
    },
    formatChatSidebarTime(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      var now = new Date();
      var sameDay = d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate();
      if (sameDay) return d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
      var y = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      var isYesterday = d.getFullYear() === y.getFullYear() && d.getMonth() === y.getMonth() && d.getDate() === y.getDate();
      if (isYesterday) return 'Yesterday';
      return d.toLocaleDateString([], { month: 'short', day: 'numeric' });
    },
    agentAutoTerminateEnabled(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (typeof agent.auto_terminate_allowed === 'boolean') {
        return agent.auto_terminate_allowed;
      }
      // Server contract should provide explicit policy; default fail-closed.
      return false;
    },
    agentContractRemainingMs(agent) {
      // Force recompute every second for live countdown updates.
      var _tick = Number(this.clockTick || 0);
      void _tick;
      if (!this.agentAutoTerminateEnabled(agent)) return null;
      var store = this.getAppStore();
      var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
      var ageDriftMs =
        Number.isFinite(lastRefreshAt) && lastRefreshAt > 0
          ? Math.max(0, Date.now() - lastRefreshAt)
          : 0;
      if (!agent || typeof agent !== 'object') return null;
      var directRemaining = Number(agent.contract_remaining_ms);
      if (Number.isFinite(directRemaining) && directRemaining >= 0) {
        return Math.max(0, Math.floor(directRemaining - ageDriftMs));
      }
      return null;
    },
    agentContractHasFiniteExpiry(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.revive_recommended === true) return true;
      if (typeof agent.contract_finite_expiry === 'boolean') {
        return agent.contract_finite_expiry;
      }
      var directRemaining = Number(agent.contract_remaining_ms);
      if (Number.isFinite(directRemaining) && directRemaining >= 0) return true;
      var totalMs = Number(agent.contract_total_ms);
      return Number.isFinite(totalMs) && totalMs > 0;
    },
    agentContractTerminationGraceMs() {
      return 10000;
    },
    isAgentPendingTermination(agent) {
      if (!this.agentAutoTerminateEnabled(agent)) return false;
      if (!this.agentContractHasFiniteExpiry(agent)) return false;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null || remainingMs > 0) return false;
      var store = this.getAppStore();
      var lastRefreshAt = Number((store && store._lastAgentsRefreshAt) || 0);
      if (!Number.isFinite(lastRefreshAt) || lastRefreshAt <= 0) return true;
      var refreshAgeMs = Math.max(0, Date.now() - lastRefreshAt);
      return refreshAgeMs < this.agentContractTerminationGraceMs();
    },
    shouldShowInfinityLifespan(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.revive_recommended === true) return false;
      if (typeof agent.contract_finite_expiry === 'boolean') {
        if (agent.contract_finite_expiry) return false;
        return !this.agentAutoTerminateEnabled(agent);
      }
      if (!this.agentAutoTerminateEnabled(agent)) return true;
      // Unknown contract timing should not be rendered as explicit infinity.
      return false;
    },
    shouldShowExpiryCountdown(agent) {
      if (agent && agent.revive_recommended === true) return true;
      if (!this.agentAutoTerminateEnabled(agent)) return false;
      if (!this.agentContractHasFiniteExpiry(agent)) return false;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      if (remainingMs <= 0) return this.isAgentPendingTermination(agent);
      return true;
    },
    expiryCountdownLabel(agent) {
      if (agent && agent.revive_recommended === true) return 'timed out';
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return '';

      if (remainingMs <= 0) return this.isAgentPendingTermination(agent) ? '0m' : '';
      var totalMin = Math.max(1, Math.ceil(remainingMs / 60000));
      var monthMin = 30 * 24 * 60;
      if (totalMin >= monthMin) {
        return Math.max(1, Math.ceil(totalMin / monthMin)) + 'm';
      }
      if (totalMin >= 1440) {
        return Math.max(1, Math.ceil(totalMin / 1440)) + 'd';
      }
      if (totalMin >= 60) {
        return Math.max(1, Math.ceil(totalMin / 60)) + 'h';
      }
      return totalMin + 'm';
    },

    expiryCountdownCritical(agent) {
      if (agent && agent.revive_recommended === true) return false;
      if (this.isAgentPendingTermination(agent)) return true;
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return false;
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) return false;
      var thresholdMs = Math.min(3600000, Math.max(1, Math.floor(totalMs * 0.2)));
      return remainingMs > 0 && remainingMs <= thresholdMs;
    },

    agentContractTotalMs(agent) {
      if (!agent || typeof agent !== 'object') return null;
      var durationMs = Number(agent.contract_total_ms);
      if (Number.isFinite(durationMs) && durationMs > 0) return Math.floor(durationMs);
      return null;
    },

    agentHeartStates(agent) {
      var totalHearts = 5;
      var hearts = [true, true, true, true, true];
      if (!agent || typeof agent !== 'object') return hearts;
      if (agent.is_system_thread) return hearts;
      if (agent.revive_recommended === true) return [false, false, false, false, false];
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) return [true];
      var remainingMs = this.agentContractRemainingMs(agent);
      if (remainingMs == null) return [true];
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) return [false, false, false, false, false];
      var totalMs = this.agentContractTotalMs(agent);
      if (!Number.isFinite(totalMs) || totalMs <= 0) return [true];
      var ratio = Math.max(0, Math.min(1, remainingMs / totalMs));
      var filled = Math.ceil(ratio * totalHearts);
      if (remainingMs <= 0 && this.isAgentPendingTermination(agent)) filled = 0;
      if (filled < 0) filled = 0;
      if (filled > totalHearts) filled = totalHearts;
      for (var i = 0; i < totalHearts; i++) {
        hearts[i] = i < filled;
      }
      return hearts;
    },

    agentHeartShowsInfinity(agent) {
      if (!agent || typeof agent !== 'object') return false;
      if (agent.is_system_thread) return false;
      if (agent.revive_recommended === true) return false;
      return !this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent);
    },

    agentHeartMeterLabel(agent) {
      if (!agent || typeof agent !== 'object' || agent.is_system_thread) return '';
      if (agent.revive_recommended === true) return 'Time limit: timed out';
      if (!this.agentAutoTerminateEnabled(agent) || !this.agentContractHasFiniteExpiry(agent)) {
        return 'Time limit: unlimited';
      }
      var label = this.expiryCountdownLabel(agent);
      if (label) return 'Time remaining: ' + label;
      return 'Time limit active';
    },

    showDashboardPopup(id, label, ev, overrides) {
      var popupId = String(id || '').trim();
      var title = String(label || '').trim();
      if (!popupId || !title) {
        if (typeof this.hideDashboardPopup === 'function') this.hideDashboardPopup();
        return;
      }
      var eventType = String((ev && ev.type) || '').toLowerCase();
      if (
        eventType === 'mouseleave' ||
        eventType === 'pointerleave' ||
        eventType === 'blur' ||
        eventType === 'focusout'
      ) {
        if (typeof this.hideDashboardPopup === 'function') this.hideDashboardPopup(popupId);
        return;
      }
      if (ev && ev.isTrusted === false) return;
      var config = overrides && typeof overrides === 'object' ? overrides : {};
      var anchor = typeof this.dashboardPopupAnchorPoint === 'function'
        ? this.dashboardPopupAnchorPoint(ev, config.side)
        : { left: 0, top: 0, side: String(config.side || 'bottom'), inline_away: 'right', block_away: 'bottom' };
      var service = typeof this.dashboardPopupService === 'function' ? this.dashboardPopupService() : null;
      this.dashboardPopup = service && typeof service.openState === 'function'
        ? service.openState(popupId, title, config, anchor)
        : {
          id: popupId,
          active: true,
          source: String(config.source || '').trim(),
          title: title,
          body: String(config.body || '').trim(),
          meta_origin: String(config.meta_origin || 'Taskbar').trim(),
          meta_time: String(config.meta_time || '').trim(),
          unread: !!config.unread,
          left: anchor.left,
          top: anchor.top,
          side: anchor.side,
          inline_away: anchor.inline_away === 'left' ? 'left' : 'right',
          block_away: anchor.block_away === 'top' ? 'top' : 'bottom',
          compact: false
        };
    },

    hideDashboardPopup(rawId) {
      var service = typeof this.dashboardPopupService === 'function' ? this.dashboardPopupService() : null;
      if (service && typeof service.closeState === 'function') {
        this.dashboardPopup = service.closeState(this.dashboardPopup, rawId);
        return;
      }
      var popupId = String(rawId || '').trim();
      var currentId = String((this.dashboardPopup && this.dashboardPopup.id) || '').trim();
      if (popupId && currentId && popupId !== currentId) return;
      if (typeof this.clearDashboardPopupState === 'function') {
        this.clearDashboardPopupState();
        return;
      }
      this.dashboardPopup = {
        id: '',
        active: false,
        source: '',
        title: '',
        body: '',
        meta_origin: '',
        meta_time: '',
        unread: false,
        left: 0,
        top: 0,
        side: 'bottom',
        inline_away: 'right',
        block_away: 'bottom',
        compact: false
      };
    },

    hideDashboardPopupBySource(source) {
      var popupSource = String(source || '').trim();
      if (!popupSource) return;
      var popup = this.dashboardPopup || {};
      if (String(popup.source || '').trim() !== popupSource) return;
      this.hideDashboardPopup(String(popup.id || '').trim());
    },

    closeTaskbarHeroMenu() {
      this.taskbarHeroMenuOpen = false;
    },

    closeTaskbarTextMenu() {
      this.taskbarTextMenuOpen = '';
    },

    taskbarTextMenuIsOpen(menuName) {
      var key = String(menuName || '').trim().toLowerCase();
      if (!key) return false;
      return String(this.taskbarTextMenuOpen || '').trim().toLowerCase() === key;
    },

    toggleTaskbarTextMenu(menuName) {
      var key = String(menuName || '').trim().toLowerCase();
      if (!key) {
        this.closeTaskbarTextMenu();
        return;
      }
      this.closeTaskbarHeroMenu();
      this.taskbarTextMenuOpen = this.taskbarTextMenuIsOpen(key) ? '' : key;
    },

    handleTaskbarHelpManual() {
      this.closeTaskbarTextMenu();
      this.openPopupWindow('manual');
    },
    handleTaskbarHelpReportIssue() {
      this.closeTaskbarTextMenu();
      this.openPopupWindow('report');
    },
    async submitReportIssueDraft() {
      var draft = String(this.reportIssueDraft || '').trim();
      if (!draft) {
        InfringToast.error('Please add issue details before submitting.');
        return;
      }
      var entry = {
        id: 'issue-' + String(Date.now()),
        ts: Date.now(),
        text: draft,
        page: String(this.page || '').trim(),
        agent_id: String((this.currentAgent && this.currentAgent.id) || '').trim()
      };
      try {
        var raw = localStorage.getItem('infring-issue-report-drafts');
        var list = raw ? JSON.parse(raw) : [];
        if (!Array.isArray(list)) list = [];
        list.unshift(entry);
        localStorage.setItem('infring-issue-report-drafts', JSON.stringify(list.slice(0, 25)));
      } catch(_) {}
      var title = ((draft.split(/\r?\n/).find(function(line) { return String(line || '').trim(); }) || draft).replace(/\s+/g, ' ').trim().slice(0, 120) || 'Dashboard issue report');
      var issueBody = '## User Report\n\n' + draft + '\n\n## Runtime Context\n- page: ' + (entry.page || 'unknown') + '\n- agent_id: ' + (entry.agent_id || 'none') + '\n- reported_at: ' + new Date(entry.ts || Date.now()).toISOString() + '\n- client_version: ' + String(this.version || 'unknown');
      try {
        var result = await InfringAPI.post('/api/dashboard/action', {
          action: 'dashboard.github.issue.create',
          payload: { title: title, body: issueBody, source: 'dashboard_report_popup' }
        });
        var actionResult = result && typeof result === 'object' ? (result.lane || result.payload || result) : {};
        if ((result && result.ok === false) || (actionResult && actionResult.ok === false)) {
          throw new Error(String((actionResult && (actionResult.error || actionResult.message)) || (result && (result.error || result.message)) || 'issue_submit_failed'));
        }
        var issueUrl = String((actionResult && (actionResult.html_url || actionResult.issue_url)) || '').trim();
        this.reportIssueDraft = ''; this.closePopupWindow('report');
        InfringToast.success(issueUrl ? ('Issue submitted: ' + issueUrl) : 'Issue submitted.');
      } catch (e) {
        InfringToast.error('Issue submit failed (saved locally): ' + String(e && e.message ? e.message : 'unknown error'));
      }
    },
    manualDocumentMarkdown() {
      // Canonical source: docs/workspace/manuals/infring_manual_help_tab.md
      var encoded = 'IyBJbmZyaW5nIE1hbnVhbAoKX09wZXJhdG9yLWZhY2luZyBndWlkZSBmb3IgdGhlIEhlbHAgdGFiXwoKIyMgVGFibGUgb2YgQ29udGVudHMKLSBbV2hhdCBJbmZyaW5nIElzXSgjd2hhdC1pbmZyaW5nLWlzKQotIFtJbnN0YWxsICsgU3RhcnRdKCNpbnN0YWxsLS1zdGFydCkKLSBbQ0xJIEd1aWRlXSgjY2xpLWd1aWRlKQotIFtVSSBHdWlkZV0oI3VpLWd1aWRlKQotIFtUb29scyArIEV2aWRlbmNlXSgjdG9vbHMtLWV2aWRlbmNlKQotIFtNZW1vcnkgKyBTZXNzaW9uc10oI21lbW9yeS0tc2Vzc2lvbnMpCi0gW1NhZmV0eSBNb2RlbF0oI3NhZmV0eS1tb2RlbCkKLSBbVHJvdWJsZXNob290aW5nXSgjdHJvdWJsZXNob290aW5nKQotIFtSZXBvcnRpbmcgSXNzdWVzXSgjcmVwb3J0aW5nLWlzc3VlcykKCi0tLQoKIyMgV2hhdCBJbmZyaW5nIElzCgpJbmZyaW5nIGlzIGEgbG9jYWwsIGRldGVybWluaXN0aWMsIHJlY2VpcHQtZmlyc3QgYXV0b21hdGlvbiBhbmQgb3JjaGVzdHJhdGlvbiBydW50aW1lLgoKSW4gcHJhY3RpY2FsIHRlcm1zLCB0aGF0IG1lYW5zOgotICoqQ29yZSB0cnV0aCBsaXZlcyBpbiB0aGUgUnVzdCBjb3JlLioqIENyaXRpY2FsIHBvbGljeSwgcmVjZWlwdHMsIGV4ZWN1dGlvbiwgYW5kIHNhZmV0eSBkZWNpc2lvbnMgYXJlIGF1dGhvcml0YXRpdmUgaW4gY29yZSBsYW5lcy4KLSAqKlRoZSBvcmNoZXN0cmF0aW9uIGxheWVyIGNvb3JkaW5hdGVzIHdvcmsuKiogSXQgc2hhcGVzIHJlcXVlc3RzLCBwbGFucyB3b3JrLCBoYW5kbGVzIGNsYXJpZmljYXRpb24sIGFuZCBwYWNrYWdlcyByZXN1bHRzLgotICoqVGhlIGNsaWVudC9kYXNoYm9hcmQgaXMgYSBwcmVzZW50YXRpb24gc3VyZmFjZS4qKiBJdCBpcyB0aGVyZSB0byBoZWxwIHlvdSBvcGVyYXRlIHRoZSBzeXN0ZW0sIG5vdCB0byBiZSB0aGUgc291cmNlIG9mIHRydXRoLgotICoqT3BlcmF0aW9ucyBhcmUgZXZpZGVuY2UtYmFja2VkLioqIEltcG9ydGFudCBhY3Rpb25zIGFuZCBvdXRjb21lcyBhcmUgZGVzaWduZWQgdG8gYmUgdHJhY2VhYmxlLgotICoqRmFpbHVyZSBpcyBkZXNpZ25lZCB0byBiZSBmYWlsLWNsb3NlZC4qKiBJZiBJbmZyaW5nIGlzIHVuc3VyZSBvciBhIHJlcXVpcmVkIGxhbmUgaXMgdW5hdmFpbGFibGUsIHRoZSBjb3JyZWN0IHJlc3VsdCBpcyBvZnRlbiB0byBzdG9wLCBkZWdyYWRlIHNhZmVseSwgb3IgYXNrIGZvciBjbGFyaWZpY2F0aW9uIGluc3RlYWQgb2YgZ3Vlc3NpbmcuCgojIyMgUnVudGltZSBQcm9maWxlcwoKSW5mcmluZyBzdXBwb3J0cyBtdWx0aXBsZSBydW50aW1lIHByb2ZpbGVzOgotICoqcmljaCoqIOKAlCBmdWxsIG9wZXJhdG9yIGV4cGVyaWVuY2UsIGluY2x1ZGluZyB0aGUgZ2F0ZXdheS9kYXNoYm9hcmQgc3VyZmFjZS4KLSAqKnB1cmUqKiDigJQgUnVzdC1vbmx5IHByb2ZpbGUgd2l0aCBubyByaWNoIGdhdGV3YXkgVUkgc3VyZmFjZS4KLSAqKnRpbnktbWF4Kiog4oCUIHNtYWxsZXN0IHB1cmUgcHJvZmlsZSBmb3IgY29uc3RyYWluZWQgZW52aXJvbm1lbnRzLgoKIyMjIEV4cGVyaW1lbnRhbCBTdXJmYWNlcwoKU29tZSBsYW5lcyBhcmUgZXhwbGljaXRseSBleHBlcmltZW50YWwuIEluIHBhcnRpY3VsYXIsIHRoZSBgYXNzaW1pbGF0ZWAgcnVudGltZSBzdXJmYWNlIGlzIGd1YXJkZWQgYW5kIG5vdCBwYXJ0IG9mIHRoZSBub3JtYWwgcHVibGljIHByb2R1Y3Rpb24gc3VyZmFjZS4KCiMjIyBXaGVuIHRvIHVzZSBJbmZyaW5nCgpVc2UgSW5mcmluZyB3aGVuIHlvdSB3YW50OgotIGEgbG9jYWwgb3BlcmF0b3IgcnVudGltZQotIGRldGVybWluaXN0aWMsIHBvbGljeS1nb3Zlcm5lZCBleGVjdXRpb24KLSBhIGRhc2hib2FyZCBmb3IgaW50ZXJhY3RpdmUgb3BlcmF0aW9uCi0gYSBDTEkgZm9yIHNjcmlwdGluZywgdmVyaWZpY2F0aW9uLCBhbmQgY29udHJvbGxlZCB3b3JrZmxvd3MKCi0tLQoKIyMgSW5zdGFsbCArIFN0YXJ0CgojIyMgUXVpY2sgaW5zdGFsbAoKIyMjIG1hY09TIC8gTGludXgKYGBgYmFzaApjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLWZ1bGwgaW5mcmluZyBnYXRld2F5CmBgYAoKIyMjIFdpbmRvd3MgKFBvd2VyU2hlbGwpCmBgYHBvd2Vyc2hlbGwKU2V0LUV4ZWN1dGlvblBvbGljeSAtU2NvcGUgUHJvY2VzcyAtRXhlY3V0aW9uUG9saWN5IEJ5cGFzcyAtRm9yY2UKJHRtcCA9IEpvaW4tUGF0aCAkZW52OlRFTVAgImluZnJpbmctaW5zdGFsbC5wczEiCmlybSBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHJvdGhldXNsYWJzL0luZlJpbmcvbWFpbi9pbnN0YWxsLnBzMSAtT3V0RmlsZSAkdG1wCiYgJHRtcCAtUmVwYWlyIC1GdWxsClJlbW92ZS1JdGVtICR0bXAgLUZvcmNlCkdldC1Db21tYW5kIGluZnJpbmcgLUVycm9yQWN0aW9uIFNpbGVudGx5Q29udGludWUKaW5mcmluZyBnYXRld2F5CmBgYAoKIyMjIFZlcmlmeSB0aGUgQ0xJCmBgYGJhc2gKaW5mcmluZyAtLWhlbHAKaW5mcmluZyBsaXN0CmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKYGBgCgpJZiB5b3VyIHNoZWxsIGhhcyBub3QgcmVmcmVzaGVkIGBQQVRIYCB5ZXQ6CmBgYGJhc2gKLiAiJEhPTUUvLmluZnJpbmcvZW52LnNoIgpoYXNoIC1yIDI+L2Rldi9udWxsIHx8IHRydWUKaW5mcmluZyAtLWhlbHAKYGBgCgpEaXJlY3QtcGF0aCBmYWxsYmFjazoKYGBgYmFzaAoiJEhPTUUvLmluZnJpbmcvYmluL2luZnJpbmciIC0taGVscApgYGAKClBvd2VyU2hlbGwgZmFsbGJhY2s6CmBgYHBvd2Vyc2hlbGwKJGVudjpQYXRoID0gIiRIT01FLy5pbmZyaW5nL2JpbjskZW52OlBhdGgiCmluZnJpbmcgLS1oZWxwCmBgYAoKIyMjIFN0YXJ0IHRoZSBvcGVyYXRvciBzdXJmYWNlCmBgYGJhc2gKaW5mcmluZyBnYXRld2F5CmBgYAoKVGhpcyBzdGFydHMgdGhlIHJ1bnRpbWUgYW5kIGRhc2hib2FyZC4KClByaW1hcnkgZGFzaGJvYXJkIFVSTDoKYGBgdGV4dApodHRwOi8vMTI3LjAuMC4xOjQxNzMvZGFzaGJvYXJkI2NoYXQKYGBgCgpIZWFsdGggZW5kcG9pbnQ6CmBgYHRleHQKaHR0cDovLzEyNy4wLjAuMTo0MTczL2hlYWx0aHoKYGBgCgojIyMgQ29tbW9uIGxpZmVjeWNsZSBjb21tYW5kcwpgYGBiYXNoCmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKaW5mcmluZyBnYXRld2F5IHN0b3AKaW5mcmluZyBnYXRld2F5IHJlc3RhcnQKYGBgCgojIyMgSW5zdGFsbCBtb2RlcwotIGAtLW1pbmltYWxgIOKAlCBDTEkgKyBkYWVtb24gd3JhcHBlcnMKLSBgLS1mdWxsYCDigJQgZnVsbCBydW50aW1lIGJvb3RzdHJhcAotIGAtLXB1cmVgIOKAlCBSdXN0LW9ubHkgcnVudGltZSBzdXJmYWNlCi0gYC0tdGlueS1tYXhgIOKAlCBzbWFsbGVzdCBwdXJlIHByb2ZpbGUKLSBgLS1yZXBhaXJgIOKAlCBjbGVhbiByZWluc3RhbGwgLyBzdGFsZS1hcnRpZmFjdCBjbGVhbnVwCgpFeGFtcGxlczoKYGBgYmFzaAojIHB1cmUgcHJvZmlsZQpjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLXB1cmUKCiMgdGlueS1tYXggcHJvZmlsZQpjdXJsIC1mc1NMIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wcm90aGV1c2xhYnMvSW5mUmluZy9tYWluL2luc3RhbGwuc2ggfCBzaCAtcyAtLSAtLXRpbnktbWF4CgojIHJlcGFpciArIGZ1bGwKY3VybCAtZnNTTCBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHJvdGhldXNsYWJzL0luZlJpbmcvbWFpbi9pbnN0YWxsLnNoIHwgc2ggLXMgLS0gLS1yZXBhaXIgLS1mdWxsCgojIGluLXBsYWNlIHVwZGF0ZQppbmZyaW5nIHVwZGF0ZSAtLXJlcGFpciAtLWZ1bGwKYGBgCgotLS0KCiMjIENMSSBHdWlkZQoKIyMjIFByaW1hcnkgZW50cnlwb2ludHMKLSBgaW5mcmluZ2Ag4oCUIG1haW4gb3BlcmF0b3IgZW50cnlwb2ludAotIGBpbmZyaW5nY3RsYCDigJQgd3JhcHBlci9jb250cm9sIHN1cmZhY2UKLSBgaW5mcmluZ2RgIOKAlCBkYWVtb24tb3JpZW50ZWQgd3JhcHBlcgoKIyMjIEV2ZXJ5ZGF5IGNvbW1hbmRzCmBgYGJhc2gKaW5mcmluZyBoZWxwCmluZnJpbmcgbGlzdAppbmZyaW5nIHZlcnNpb24KaW5mcmluZyBnYXRld2F5CmluZnJpbmcgZ2F0ZXdheSBzdGF0dXMKaW5mcmluZyBnYXRld2F5IHN0b3AKaW5mcmluZyBnYXRld2F5IHJlc3RhcnQKYGBgCgojIyMgT3BlcmF0aW9uYWwgZmFsbGJhY2sgc3VyZmFjZQpXaGVuIE5vZGUuanMgaXMgdW5hdmFpbGFibGUsIEluZnJpbmcgc3RpbGwgZXhwb3NlcyBhIHJlZHVjZWQgUnVzdC1iYWNrZWQgc3VyZmFjZS4KCkF2YWlsYWJsZSBmYWxsYmFjayBmYW1pbGllcyBpbmNsdWRlOgotIGBnYXRld2F5IFtzdGFydHxzdG9wfHJlc3RhcnR8c3RhdHVzXWAKLSBgdXBkYXRlYAotIGB2ZXJpZnktZ2F0ZXdheWAKLSBgc3RhcnRgLCBgc3RvcGAsIGByZXN0YXJ0YAotIGBkYXNoYm9hcmRgLCBgc3RhdHVzYAotIGBzZXNzaW9uYAotIGByYWdgCi0gYG1lbW9yeWAKLSBgYWRhcHRpdmVgCi0gYGVudGVycHJpc2UtaGFyZGVuaW5nYAotIGBiZW5jaG1hcmtgCi0gYGFscGhhLWNoZWNrYAotIGByZXNlYXJjaGAKLSBgaGVscGAsIGBsaXN0YCwgYHZlcnNpb25gCgpOb3QgYXZhaWxhYmxlIGluIE5vZGUtZnJlZSBmYWxsYmFjazoKLSBgYXNzaW1pbGF0ZWAKCiMjIyBGdWxsIC8gZXhwZXJpbWVudGFsIHN1cmZhY2UKYGFzc2ltaWxhdGVgIHJlcXVpcmVzIHRoZSBmdWxsIE5vZGUuanMtYXNzaXN0ZWQgc3VyZmFjZSBhbmQgc2hvdWxkIGJlIHRyZWF0ZWQgYXMgZXhwZXJpbWVudGFsLgoKRXhhbXBsZToKYGBgYmFzaAppbmZyaW5nIGFzc2ltaWxhdGUgdGFyZ2V0LW5hbWUgLS1wbGFuLW9ubHk9MSAtLWpzb249MQpgYGAKClVzZWZ1bCBmbGFnczoKLSBgLS1wbGFuLW9ubHk9MWAg4oCUIGVtaXQgdGhlIHBsYW5uaW5nIGNoYWluIHdpdGhvdXQgZXhlY3V0aW5nIG11dGF0aW9ucwotIGAtLWpzb249MWAg4oCUIHN0cnVjdHVyZWQgb3V0cHV0Ci0gYC0tc3RyaWN0PTFgIOKAlCB0aWdodGVyIGVuZm9yY2VtZW50Ci0gYC0tYWxsb3ctbG9jYWwtc2ltdWxhdGlvbj0xYCDigJQgdGVzdC1vbmx5IGxvY2FsIHNpbXVsYXRpb24gcGF0aAoKIyMjIENvbnRyaWJ1dG9yIC8gcmVwb3NpdG9yeSB3b3JrZmxvd3MKSWYgeW91IGFyZSB3b3JraW5nIGZyb20gdGhlIHJlcG9zaXRvcnkgZGlyZWN0bHksIHRoZXNlIGFyZSB0aGUgY2Fub25pY2FsIHdvcmtzcGFjZSBlbnRyeXBvaW50czoKYGBgYmFzaApucG0gcnVuIC1zIHdvcmtzcGFjZTpjb21tYW5kcwpucG0gcnVuIC1zIHRvb2xpbmc6bGlzdApucG0gcnVuIC1zIHdvcmtzcGFjZTpkZXYKbnBtIHJ1biAtcyB3b3Jrc3BhY2U6dmVyaWZ5Cm5wbSBydW4gLXMgbGFuZTpsaXN0IC0tIC0tanNvbj0xCmBgYAoKLS0tCgojIyBVSSBHdWlkZQoKIyMjIFdoYXQgdGhlIGRhc2hib2FyZCBpcyBmb3IKVGhlIGRhc2hib2FyZCBpcyB0aGUgcHJpbWFyeSBpbnRlcmFjdGl2ZSBvcGVyYXRvciBzdXJmYWNlIGluIHRoZSAqKnJpY2gqKiBwcm9maWxlLiBJdCBpcyB0aGUgcmlnaHQgcGxhY2UgdG86Ci0gd29yayBpbnRlcmFjdGl2ZWx5Ci0gaW5zcGVjdCBzdGF0dXMgYW5kIG91dHB1dHMKLSB1c2UgdGhlIGNoYXQvb3BlcmF0b3Igc3VyZmFjZQotIHJlYWQgYnVpbHQtaW4gaGVscAotIHZhbGlkYXRlIHRoYXQgdGhlIHJ1bnRpbWUgaXMgdXAgYmVmb3JlIHlvdSBtb3ZlIGludG8gZGVlcGVyIENMSS9vcHMgd29yawoKIyMjIFdoYXQgdGhlIGRhc2hib2FyZCBpcyBub3QKVGhlIGRhc2hib2FyZCBpcyAqKm5vdCoqIHRoZSBzeXN0ZW3igJlzIHNvdXJjZSBvZiB0cnV0aC4gSWYgdGhlIFVJIGFuZCB0aGUgcnVudGltZSBkaXNhZ3JlZSwgdHJ1c3QgdGhlIHJ1bnRpbWXigJlzIHJlY2VpcHRzLCBzdGF0dXMgY29tbWFuZHMsIGFuZCBzdXBwb3J0IGFydGlmYWN0cy4KCiMjIyBSZWNvbW1lbmRlZCBvcGVyYXRvciB3b3JrZmxvdwoxLiBTdGFydCB0aGUgc3lzdGVtIHdpdGggYGluZnJpbmcgZ2F0ZXdheWAuCjIuIE9wZW4gdGhlIGRhc2hib2FyZC4KMy4gVXNlIHRoZSBjaGF0L29wZXJhdG9yIHN1cmZhY2UgZm9yIGludGVyYWN0aXZlIHdvcmsuCjQuIFVzZSBDTEkgc3RhdHVzIGNvbW1hbmRzIGZvciB2ZXJpZmljYXRpb24gd2hlbiBuZWVkZWQuCjUuIFVzZSBzdXBwb3J0L2V4cG9ydCB0b29saW5nIHdoZW4gZGlhZ25vc2luZyBpbmNpZGVudHMgb3IgZmlsaW5nIGlzc3Vlcy4KCiMjIyBSaWNoIHZzIHB1cmUgcHJvZmlsZXMKLSAqKnJpY2gqKjogZGFzaGJvYXJkIGF2YWlsYWJsZQotICoqcHVyZSAvIHRpbnktbWF4Kio6IGludGVudGlvbmFsbHkgbm8gcmljaCBnYXRld2F5IFVJIHN1cmZhY2UKCklmIHlvdSBhcmUgb24gYC0tcHVyZWAgb3IgYC0tdGlueS1tYXhgLCB1c2UgdGhlIENMSSBpbnN0ZWFkIG9mIGV4cGVjdGluZyB0aGUgZGFzaGJvYXJkLgoKIyMjIEFjY2Vzc2liaWxpdHkgZXhwZWN0YXRpb25zClRoZSBVSSBjb250cmFjdCBleHBlY3RzOgotIGtleWJvYXJkIG5hdmlnYXRpb24gZm9yIHByaW1hcnkgYWN0aW9ucwotIHZpc2libGUgZm9jdXMgaW5kaWNhdG9ycwotIHN1ZmZpY2llbnQgY29udHJhc3QgZm9yIGNyaXRpY2FsIHRleHQKLSBkb2N1bWVudGVkIGRpc2NvdmVyYWJpbGl0eSBmb3IgdGhlIGNvbW1hbmQgcGFsZXR0ZSAvIHByaW1hcnkgYWN0aW9ucwoKLS0tCgojIyBUb29scyArIEV2aWRlbmNlCgojIyMgV2hhdCB0b29scyBtZWFuIGluIEluZnJpbmcKQSB0b29sIGlzIGFuIG9wZXJhdG9yLXVzYWJsZSBsYW5lIHRoYXQgcGVyZm9ybXMgYSBnb3Zlcm5lZCBhY3Rpb24gdGhyb3VnaCB0aGUgcnVudGltZS4gSW5mcmluZyBpcyBkZXNpZ25lZCBzbyBpbXBvcnRhbnQgYWN0aW9ucyBhcmUgcG9saWN5LWdvdmVybmVkIGFuZCBldmlkZW5jZS1iYWNrZWQgaW5zdGVhZCBvZiBiZWluZyBvcGFxdWUgc2lkZSBlZmZlY3RzLgoKIyMjIFdoYXQgZXZpZGVuY2UgbWVhbnMKRXZpZGVuY2UgaXMgdGhlIHN1cHBvcnRpbmcgcmVjb3JkIGZvciBhIGNsYWltLCByZXN1bHQsIG9yIGFjdGlvbi4gSW5mcmluZ+KAmXMgZG9jdW1lbnRhdGlvbiBwb2xpY3kgaXMgZXhwbGljaXQ6IG1lYXN1cmFibGUsIGNvbXBhcmF0aXZlLCBzZWN1cml0eS1zZW5zaXRpdmUsIG9yIGN1c3RvbWVyLWltcGFjdGluZyBjbGFpbXMgbXVzdCBoYXZlIGxpbmtlZCBldmlkZW5jZS4KCkV4YW1wbGVzIG9mIGV2aWRlbmNlIGluY2x1ZGU6Ci0gcmVjZWlwdHMKLSBiZW5jaG1hcmsgYXJ0aWZhY3RzCi0gdmVyaWZpY2F0aW9uIG91dHB1dHMKLSBkcmlsbCAvIHJlY292ZXJ5IGFydGlmYWN0cwotIHN1cHBvcnQgYnVuZGxlcwotIGxvZ3MgYW5kIHN0YXRlIGFydGlmYWN0cyB3aGVuIHNoYXJlYWJsZSBhbmQgYXBwcm9wcmlhdGUKCiMjIyBIb3cgdG8gaW50ZXJwcmV0IG91dHB1dHMKV2hlbiByZWFkaW5nIGEgcmVzdWx0LCBhc2s6Ci0gV2hhdCBoYXBwZW5lZD8KLSBXaGF0IGV2aWRlbmNlIHN1cHBvcnRzIGl0PwotIFdhcyB0aGUgYWN0aW9uIHN1Y2Nlc3NmdWwsIGRlZ3JhZGVkLCBibG9ja2VkLCBvciBmYWlsLWNsb3NlZD8KLSBJcyB0aGVyZSBhIHJlY2VpcHQsIGFydGlmYWN0LCBvciBzdGF0dXMgcmVjb3JkIEkgY2FuIGluc3BlY3Q/CgojIyMgUHJhY3RpY2FsIHJ1bGUKSWYgeW91IHdhbnQgdG8gbWFrZSBhIHB1YmxpYyBjbGFpbSBhYm91dCBwZXJmb3JtYW5jZSwgcmVsaWFiaWxpdHksIG9yIHNlY3VyaXR5LCBkbyBub3QgcmVseSBvbiBVSSB0ZXh0IGFsb25lLiBMaW5rIHRoZSBzdXBwb3J0aW5nIGFydGlmYWN0LgoKIyMjIFVzZWZ1bCBldmlkZW5jZS9vcHMgY29tbWFuZHMKYGBgYmFzaApucG0gcnVuIC1zIG9wczpwcm9kdWN0aW9uLXRvcG9sb2d5OnN0YXR1cwpucG0gcnVuIC1zIG9wczp0cmFuc3BvcnQ6c3Bhd24tYXVkaXQKbnBtIHJ1biAtcyBvcHM6c3VwcG9ydC1idW5kbGU6ZXhwb3J0Cm5wbSBydW4gLXMgb3BzOnJlbGVhc2U6dmVyZGljdApgYGAKCi0tLQoKIyMgTWVtb3J5ICsgU2Vzc2lvbnMKCiMjIyBTZXNzaW9ucwpVc2Ugc2Vzc2lvbnMgZm9yIGFjdGl2ZSBvcGVyYXRvciB3b3JrIGFuZCBsaXZlIHJ1bnRpbWUgY29udGV4dC4KCiMjIyBNZW1vcnkKVXNlIG1lbW9yeSBzdXJmYWNlcyBmb3IgcGVyc2lzdGVkIHJ1bnRpbWUgc3RhdGUgYW5kIHJldHJpZXZhbC1vcmllbnRlZCB3b3JrZmxvd3MuCgojIyMgUkFHIC8gcmV0cmlldmFsClVzZSBgcmFnYCB3aGVuIHlvdSB3YW50IHJldHJpZXZhbC1zdHlsZSBiZWhhdmlvciBvdmVyIGluZGV4ZWQgb3IgbWVtb3J5LWJhY2tlZCBjb250ZW50LgoKIyMjIFNlc3Npb24gYW5kIG1lbW9yeSBjb21tYW5kIGZhbWlsaWVzCmBgYGJhc2gKaW5mcmluZyBzZXNzaW9uCmluZnJpbmcgbWVtb3J5CmluZnJpbmcgcmFnCmBgYAoKIyMjIE9wZXJhdG9yIGd1aWRhbmNlCi0gVHJlYXQgc2Vzc2lvbnMgYXMgYWN0aXZlIHdvcmtpbmcgY29udGV4dC4KLSBUcmVhdCBtZW1vcnkgYXMgYSBnb3Zlcm5lZCBzeXN0ZW0gc3VyZmFjZSwgbm90IGEgc2NyYXRjaHBhZCB5b3UgY2FuIGFzc3VtZSBpcyB1bmJvdW5kZWQuCi0gSWYgYSB3b3JrZmxvdyBtYXR0ZXJzLCB2YWxpZGF0ZSBpdCB0aHJvdWdoIHJlY2VpcHRzL2FydGlmYWN0cyBpbnN0ZWFkIG9mIGFzc3VtaW5nIGEgVUktb25seSBzdGF0ZSBpcyBkdXJhYmxlLgotIElmIHlvdSBhcmUgdHJvdWJsZXNob290aW5nIGEgc2Vzc2lvbiBwcm9ibGVtLCBwcmVmZXIgcnVudGltZSBzdGF0dXMgYW5kIHN1cHBvcnQtYnVuZGxlIGV4cG9ydCBvdmVyIGd1ZXNzaW5nIGZyb20gc3RhbGUgVUkgc3RhdGUuCgotLS0KCiMjIFNhZmV0eSBNb2RlbAoKSW5mcmluZ+KAmXMgc2FmZXR5IG1vZGVsIGlzIG9uZSBvZiBpdHMgZGVmaW5pbmcgdHJhaXRzLgoKIyMjIENvcmUgcnVsZXMKLSBTYWZldHkgYXV0aG9yaXR5IHN0YXlzIGRldGVybWluaXN0aWMgYW5kIGZhaWwtY2xvc2VkLgotIEFJL3Byb2JhYmlsaXN0aWMgbG9naWMgaXMgbm90IHRoZSByb290IG9mIGNvcnJlY3RuZXNzLgotIENvcmUgdHJ1dGggbGl2ZXMgaW4gdGhlIGF1dGhvcml0YXRpdmUgY29yZS4KLSBCb3VuZGFyeSBjcm9zc2luZyBpcyBleHBsaWNpdCBhbmQgZ292ZXJuZWQuCi0gVW5zdXBwb3J0ZWQgb3IgdW5hZG1pdHRlZCBhY3Rpb25zIHNob3VsZCBzdG9wIG9yIGRlZ3JhZGUgc2FmZWx5LgoKIyMjIFdoYXQgdGhhdCBtZWFucyBmb3Igb3BlcmF0b3JzCi0gSWYgYSBjb21tYW5kIGlzIGJsb2NrZWQsIHRoYXQgaXMgb2Z0ZW4gdGhlIGNvcnJlY3QgYmVoYXZpb3IuCi0gRXhwZXJpbWVudGFsIGZlYXR1cmVzIG1heSByZXF1aXJlIGV4cGxpY2l0IGZsYWdzIGFuZCBleHRyYSB2YWxpZGF0aW9uLgotIFByb2R1Y3Rpb24gcmVsZWFzZSBjaGFubmVscyBhcmUgcmVzaWRlbnQtSVBDIGF1dGhvcml0YXRpdmUuCi0gTGVnYWN5IHByb2Nlc3MgdHJhbnNwb3J0IGlzIG5vdCBhIHN1cHBvcnRlZCBwcm9kdWN0aW9uIHBhdGguCgojIyMgU2VjdXJpdHkgcG9zdHVyZQpUaGUgcmVwb3NpdG9yeeKAmXMgc2VjdXJpdHkgcG9zdHVyZSBlbXBoYXNpemVzOgotIGZhaWwtY2xvc2VkIHBvbGljeSBjaGVja3MKLSBkZXRlcm1pbmlzdGljIHJlY2VpcHRzIG9uIGNyaXRpY2FsIGxhbmVzCi0gbGVhc3QtYXV0aG9yaXR5IGNvbW1hbmQgcm91dGluZwotIHJlbGVhc2UtdGltZSBldmlkZW5jZSBzdWNoIGFzIFNCT01zLCBDb2RlUUwsIGFuZCB2ZXJpZmljYXRpb24gYXJ0aWZhY3RzCgojIyMgVnVsbmVyYWJpbGl0eSByZXBvcnRpbmcKRG8gKipub3QqKiBmaWxlIHB1YmxpYyBHaXRIdWIgaXNzdWVzIGZvciBzZWN1cml0eSB2dWxuZXJhYmlsaXRpZXMuIFVzZSBwcml2YXRlIHJlcG9ydGluZyBpbnN0ZWFkLgoKLS0tCgojIyBUcm91Ymxlc2hvb3RpbmcKCiMjIyBgaW5mcmluZ2AgY29tbWFuZCBub3QgZm91bmQKUmVsb2FkIHlvdXIgc2hlbGwgZW52aXJvbm1lbnQ6CmBgYGJhc2gKLiAiJEhPTUUvLmluZnJpbmcvZW52LnNoIgpoYXNoIC1yIDI+L2Rldi9udWxsIHx8IHRydWUKaW5mcmluZyAtLWhlbHAKYGBgCgpEaXJlY3QtcGF0aCBmYWxsYmFjazoKYGBgYmFzaAoiJEhPTUUvLmluZnJpbmcvYmluL2luZnJpbmciIC0taGVscApgYGAKCiMjIyBHYXRld2F5L2Rhc2hib2FyZCBpcyBub3QgYXZhaWxhYmxlCkNoZWNrIHN0YXR1czoKYGBgYmFzaAppbmZyaW5nIGdhdGV3YXkgc3RhdHVzCmBgYAoKQ2hlY2sgaGVhbHRoIGVuZHBvaW50OgpgYGB0ZXh0Cmh0dHA6Ly8xMjcuMC4wLjE6NDE3My9oZWFsdGh6CmBgYAoKUmVzdGFydDoKYGBgYmFzaAppbmZyaW5nIGdhdGV3YXkgcmVzdGFydApgYGAKCiMjIyBZb3UgbmVlZCBhIGRlZXBlciBpbmNpZGVudCBwYXRoClVzZSB0aGUgb3BlcmF0b3IgcnVuYm9vayBhbmQgZXhwb3J0IGEgc3VwcG9ydCBidW5kbGUuCgpVc2VmdWwgY29tbWFuZHM6CmBgYGJhc2gKbnBtIHJ1biAtcyBvcHM6c3VwcG9ydC1idW5kbGU6ZXhwb3J0Cm5wbSBydW4gLXMgb3BzOnN0YXR1czpwcm9kdWN0aW9uCm5wbSBydW4gLXMgb3BzOnByb2R1Y3Rpb24tdG9wb2xvZ3k6c3RhdHVzCmBgYAoKIyMjIFN0cmljdCBjaGVja3MgYXJlIGZhaWxpbmcgaW4gbG9jYWwgcmVwbyB3b3JrClJ1biB0aGUgY2Fub25pY2FsIHZlcmlmaWNhdGlvbiBwYXRoOgpgYGBiYXNoCm5wbSBydW4gLXMgd29ya3NwYWNlOnZlcmlmeQpgYGAKCkZvciBzdXJmYWNlL2RvY3MgY2hlY2tzOgpgYGBiYXNoCm5vZGUgY2xpZW50L3J1bnRpbWUvc3lzdGVtcy9vcHMvZG9jc19zdXJmYWNlX2NvbnRyYWN0LnRzIGNoZWNrIC0tc3RyaWN0PTEKbm9kZSBjbGllbnQvcnVudGltZS9zeXN0ZW1zL29wcy9yb290X3N1cmZhY2VfY29udHJhY3QudHMgY2hlY2sgLS1zdHJpY3Q9MQpgYGAKCi0tLQoKIyMgUmVwb3J0aW5nIElzc3VlcwoKIyMjIEJlZm9yZSBmaWxpbmcKUGxlYXNlIGdhdGhlcjoKLSBzdW1tYXJ5IG9mIHRoZSBwcm9ibGVtCi0gcmVwcm9kdWN0aW9uIHN0ZXBzCi0gZXhwZWN0ZWQgYmVoYXZpb3IKLSBlbnZpcm9ubWVudCBkZXRhaWxzIChPUywgTm9kZSwgUnVzdCwgQ0xJIHZlcnNpb24sIHJlbGV2YW50IGNvbmZpZykKCiMjIyBQdWJsaWMgYnVnIHJlcG9ydHMKVXNlIHRoZSBHaXRIdWIgYnVnIHJlcG9ydCB0ZW1wbGF0ZS4KCkluY2x1ZGU6Ci0gd2hhdCBoYXBwZW5lZAotIGhvdyB0byByZXByb2R1Y2UgaXQKLSB3aGF0IHlvdSBleHBlY3RlZCBpbnN0ZWFkCi0gZW52aXJvbm1lbnQgZGV0YWlscwoKIyMjIEZlYXR1cmUgcmVxdWVzdHMKVXNlIHRoZSBmZWF0dXJlIHJlcXVlc3QgdGVtcGxhdGUuCgpJbmNsdWRlOgotIHRoZSBwcm9ibGVtIHlvdSBhcmUgdHJ5aW5nIHRvIHNvbHZlCi0gdGhlIHByb3Bvc2VkIHNvbHV0aW9uCi0gYWx0ZXJuYXRpdmVzIGNvbnNpZGVyZWQKLSBleHBlY3RlZCBpbXBhY3QKCiMjIyBTZWN1cml0eSBpc3N1ZXMKRG8gKipub3QqKiBvcGVuIGEgcHVibGljIGlzc3VlIGZvciBhIHZ1bG5lcmFiaWxpdHkuCgpVc2UgdGhlIHByaXZhdGUgc2VjdXJpdHkgZGlzY2xvc3VyZSBwYXRoIGFuZCBpbmNsdWRlOgotIGltcGFjdCBzdW1tYXJ5Ci0gcmVwcm9kdWN0aW9uIHN0ZXBzCi0gYWZmZWN0ZWQgZmlsZXMvbW9kdWxlcwotIHN1Z2dlc3RlZCBtaXRpZ2F0aW9uIGlmIGtub3duCi0gc2V2ZXJpdHkgZXN0aW1hdGUgYW5kIGJsYXN0IHJhZGl1cwoKIyMjIEdvb2QgaXNzdWUgaHlnaWVuZQpBIGdvb2QgaXNzdWUgcmVwb3J0IG1ha2VzIGl0IGVhc2llciB0byBoZWxwIHlvdSBxdWlja2x5OgotIGtlZXAgaXQgc3BlY2lmaWMKLSBhdHRhY2ggdGhlIGV4YWN0IGNvbW1hbmQgb3Igd29ya2Zsb3cKLSBpbmNsdWRlIHJlbGV2YW50IHJlY2VpcHRzL2FydGlmYWN0cyBpZiBzYWZlIHRvIHNoYXJlCi0gbm90ZSB3aGV0aGVyIHlvdSBhcmUgb24gcmljaCwgcHVyZSwgb3IgdGlueS1tYXgKLSBzYXkgd2hldGhlciB0aGUgcHJvYmxlbSBpcyByZXByb2R1Y2libGUgb3IgaW50ZXJtaXR0ZW50CgotLS0KCiMjIFF1aWNrIFJlZmVyZW5jZQoKIyMjIFN0YXJ0IC8gc3RvcApgYGBiYXNoCmluZnJpbmcgZ2F0ZXdheQppbmZyaW5nIGdhdGV3YXkgc3RhdHVzCmluZnJpbmcgZ2F0ZXdheSBzdG9wCmluZnJpbmcgZ2F0ZXdheSByZXN0YXJ0CmBgYAoKIyMjIFZlcmlmeSBpbnN0YWxsYXRpb24KYGBgYmFzaAppbmZyaW5nIC0taGVscAppbmZyaW5nIGxpc3QKYGBgCgojIyMgVXBkYXRlCmBgYGJhc2gKaW5mcmluZyB1cGRhdGUgLS1yZXBhaXIgLS1mdWxsCmBgYAoKIyMjIFN1cHBvcnQgLyBkaWFnbm9zdGljcwpgYGBiYXNoCm5wbSBydW4gLXMgb3BzOnN0YXR1czpwcm9kdWN0aW9uCm5wbSBydW4gLXMgb3BzOnByb2R1Y3Rpb24tdG9wb2xvZ3k6c3RhdHVzCm5wbSBydW4gLXMgb3BzOnN1cHBvcnQtYnVuZGxlOmV4cG9ydApgYGAKCiMjIyBJbXBvcnRhbnQgVVJMcwotIERhc2hib2FyZDogYGh0dHA6Ly8xMjcuMC4wLjE6NDE3My9kYXNoYm9hcmQjY2hhdGAKLSBIZWFsdGg6IGBodHRwOi8vMTI3LjAuMC4xOjQxNzMvaGVhbHRoemAKCi0tLQoKIyMgRmluYWwgTm90ZXMKCklmIHlvdSBhcmUgdW5zdXJlIHdoZXRoZXIgdG8gdHJ1c3QgdGhlIFVJIG9yIHRoZSBydW50aW1lLCB0cnVzdCB0aGUgcnVudGltZS4KCklmIGEgbGFuZSBmYWlscyBjbG9zZWQsIHRyZWF0IHRoYXQgYXMgYSBwcm90ZWN0aXZlIGJlaGF2aW9yIGZpcnN0LCBub3QgYSBwcm9kdWN0IGZhaWx1cmUgZmlyc3QuCgpJZiB5b3UgYXJlIG1ha2luZyBhIHN0cm9uZyBjbGFpbSwgbGluayB0aGUgZXZpZGVuY2UuCg==';
      try {
        if (typeof atob === 'function') return atob(encoded);
        if (typeof Buffer !== 'undefined') return Buffer.from(encoded, 'base64').toString('utf-8');
      } catch(_) {}
      return '# Infring Manual\n\nManual content unavailable.';
    },

    manualDocumentHtml() {
      var markdown = this.manualDocumentMarkdown();
      if (typeof renderMarkdown === 'function') {
        return renderMarkdown(markdown);
      }
      return escapeHtml(markdown);
    },

    toggleTaskbarHeroMenu() {
      if (this.taskbarHeroActionPending) return;
      if (!this.taskbarHeroMenuOpen) this.closeTaskbarTextMenu();
      this.taskbarHeroMenuOpen = !this.taskbarHeroMenuOpen;
    },

    requestTaskbarRefresh() {
      this.closeTaskbarHeroMenu();
      var appStore = this.getAppStore ? this.getAppStore() : null;
      if (appStore && typeof appStore.bumpTaskbarRefreshTurn === 'function') {
        appStore.bumpTaskbarRefreshTurn();
      }
      if (this._taskbarRefreshOverlayTimer) {
        clearTimeout(this._taskbarRefreshOverlayTimer);
        this._taskbarRefreshOverlayTimer = 0;
      }
      if (this._taskbarRefreshReloadTimer) {
        clearTimeout(this._taskbarRefreshReloadTimer);
        this._taskbarRefreshReloadTimer = 0;
      }
      var self = this;
      this._taskbarRefreshOverlayTimer = window.setTimeout(function() {
        self.bootSplashVisible = true;
        self._bootSplashStartedAt = Date.now();
        if (typeof self.resetBootProgress === 'function') self.resetBootProgress();
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        self._taskbarRefreshOverlayTimer = 0;
      }, 1000);
      this._taskbarRefreshReloadTimer = window.setTimeout(function() {
        self._taskbarRefreshReloadTimer = 0;
        try {
          window.location.reload();
        } catch (_) {
          try {
            window.location.href = window.location.href;
          } catch (_) {}
        }
      }, 1100);
    },

    async postTaskbarHeroSystemRoute(route, body, options) {
      var opts = (options && typeof options === 'object') ? options : {};
      var timeoutMs = Number(opts.timeoutMs);
      if (!Number.isFinite(timeoutMs) || timeoutMs < 250) timeoutMs = 1800;
      var allowTransientSuccess = opts.allowTransientSuccess === true;
      var controller = null;
      try {
        if (typeof AbortController !== 'undefined') controller = new AbortController();
      } catch (_) {
        controller = null;
      }
      var timer = 0;
      if (controller && typeof window !== 'undefined' && typeof window.setTimeout === 'function') {
        timer = window.setTimeout(function() {
          try {
            controller.abort();
          } catch (_) {}
        }, timeoutMs);
      }
      try {
        var headers = { 'Content-Type': 'application/json' };
        try {
          var token = String(localStorage.getItem('infring-api-key') || '').trim();
          if (token) headers.Authorization = 'Bearer ' + token;
        } catch (_) {}
        var response = await fetch(route, {
          method: 'POST',
          headers: headers,
          body: JSON.stringify(body || {}),
          signal: controller ? controller.signal : undefined
        });
        var text = '';
        try {
          text = await response.text();
        } catch (_) {
          text = '';
        }
        var parsed = {};
        try {
          parsed = text ? JSON.parse(text) : {};
        } catch (_) {
          parsed = {};
        }
        if (!response.ok) {
          var error = new Error(String((parsed && (parsed.error || parsed.message)) || ('system_route_http_' + response.status)));
          error.status = response.status;
          error.payload = parsed;
          throw error;
        }
        return parsed && typeof parsed === 'object' ? parsed : {};
      } catch (error) {
        var message = String(error && error.message ? error.message : '');
        var aborted = !!(controller && controller.signal && controller.signal.aborted) || (error && error.name === 'AbortError');
        var disconnected =
          error &&
          error.name === 'TypeError' &&
          (message.indexOf('Failed to fetch') >= 0 || message.indexOf('fetch failed') >= 0);
        if (allowTransientSuccess && (aborted || disconnected)) {
          return {
            ok: true,
            type: 'dashboard_system_action_assumed',
            accepted_transient_disconnect: true
          };
        }
        throw error;
      } finally {
        if (timer) {
          try {
            clearTimeout(timer);
          } catch (_) {}
        }
      }
    },

    async runTaskbarHeroCommand(action) {
      var actionKey = String(action || '').trim().toLowerCase();
      if (!actionKey || this.taskbarHeroActionPending) return;
      var dashboardAction = '';
      var legacyRoute = '';
      var body = {};
      if (actionKey === 'restart') {
        dashboardAction = 'dashboard.system.restart';
        legacyRoute = '/api/system/restart';
      }
      else if (actionKey === 'shutdown') {
        dashboardAction = 'dashboard.system.shutdown';
        legacyRoute = '/api/system/shutdown';
      }
      else if (actionKey === 'update') {
        dashboardAction = 'dashboard.update.apply';
        legacyRoute = '/api/system/update';
        body = { apply: true };
      } else {
        return;
      }
      this.taskbarHeroActionPending = actionKey;
      try {
        var result = null;
        try {
          result = await this.postTaskbarHeroSystemRoute(legacyRoute, body, {
            timeoutMs: actionKey === 'update' ? 12000 : 1400,
            allowTransientSuccess: actionKey === 'restart' || actionKey === 'shutdown'
          });
        } catch (routeError) {
          var routeStatus = Number(routeError && routeError.status || 0);
          var routeMessage = String(routeError && routeError.message ? routeError.message : '').toLowerCase();
          var canFallbackToActionBus =
            !!dashboardAction &&
            (
              routeStatus === 404 ||
              routeStatus === 400 ||
              routeMessage.indexOf('unknown_action') >= 0 ||
              routeMessage.indexOf('resource not found') >= 0
            );
          if (!canFallbackToActionBus) throw routeError;
          result = await InfringAPI.post('/api/dashboard/action', {
            action: dashboardAction,
            payload: body
          });
        }
        var payload =
          result && result.lane && typeof result.lane === 'object'
            ? result.lane
            : (
              result && result.payload && typeof result.payload === 'object'
                ? result.payload
                : result
            );
        if (result && result.ok === false) {
          throw new Error(String(result.error || payload.error || (actionKey + '_failed')));
        }
        this.closeTaskbarHeroMenu();
        if (actionKey === 'restart') {
          InfringToast.success('Restart requested');
          this.requestTaskbarRefresh();
        } else if (actionKey === 'shutdown') {
          InfringToast.success('Shut down requested');
          this.connected = false;
          this.connectionState = 'disconnected';
          this.wsConnected = false;
        } else {
          var updateAvailable = payload.update_available;
          if (updateAvailable == null && payload.post_check && typeof payload.post_check === 'object') {
            updateAvailable = payload.post_check.has_update;
          }
          if (updateAvailable === false) {
            InfringToast.success('Already up to date');
          } else {
            InfringToast.success('Update requested');
          }
          this.requestTaskbarRefresh();
        }
      } catch (e) {
        InfringToast.error('Failed to ' + actionKey.replace(/_/g, ' ') + ': ' + (e && e.message ? e.message : 'unknown error'));
      } finally {
        this.taskbarHeroActionPending = '';
      }
    },

    normalizeDashboardHealthSummary(payload) {
      var summary = payload && typeof payload === 'object' ? payload : {};
      var agents = Array.isArray(summary.agents) ? summary.agents : [];
      return {
        ok: summary.ok === true,
        ts: Number(summary.ts || Date.now()),
        durationMs: Number(summary.durationMs != null ? summary.durationMs : summary.duration_ms || 0),
        heartbeatSeconds: Number(summary.heartbeatSeconds != null ? summary.heartbeatSeconds : summary.heartbeat_seconds || 0),
        defaultAgentId: String(summary.defaultAgentId || summary.default_agent_id || ''),
        agent_count: Number(summary.agent_count || agents.length || 0),
        agents: agents
      };
    },

    async loadDashboardHealthSummary(force) {
      var now = Date.now();
      if (!force && this._healthSummaryLoading) return this._healthSummaryLoading;
      if (!force && this._healthSummaryLoadedAt && (now - Number(this._healthSummaryLoadedAt || 0)) < 15000) {
        return this.healthSummary;
      }
      var seq = Number(this._healthSummaryLoadSeq || 0) + 1;
      this._healthSummaryLoadSeq = seq;
      var self = this;
      this._healthSummaryLoading = (async function() {
        try {
          var payload = await InfringAPI.get('/api/health');
          if (seq !== Number(self._healthSummaryLoadSeq || 0)) return self.healthSummary;
          self.healthSummary = self.normalizeDashboardHealthSummary(payload);
          self.healthSummaryError = '';
        } catch (e) {
          if (seq !== Number(self._healthSummaryLoadSeq || 0)) return self.healthSummary;
          self.healthSummary = self.normalizeDashboardHealthSummary(null);
          self.healthSummaryError = String(e && e.message ? e.message : 'health_unavailable');
        } finally {
          if (seq === Number(self._healthSummaryLoadSeq || 0)) {
            self._healthSummaryLoadedAt = Date.now();
            self._healthSummaryLoading = null;
          }
        }
        return self.healthSummary;
      })();
      return this._healthSummaryLoading;
    },

    async pollStatus(opts) {
      var force = !!(opts && opts.force);
      if (this._pollStatusInFlight) {
        this._pollStatusQueued = true;
        return this._pollStatusInFlight;
      }
      var self = this;
      this._pollStatusInFlight = (async function() {
        var store = self.getAppStore();
        if (!store) {
          self.connected = false;
          self.connectionState = 'connecting';
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_retrying');
          return;
        }
        if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('status_requesting');
        if (typeof store.checkStatus === 'function') await store.checkStatus();
        if (typeof self.setBootProgressEvent === 'function') {
          self.setBootProgressEvent(
            store && store.connectionState === 'connected' ? 'status_connected' : 'status_retrying',
            { bootStage: store && store.bootStage }
          );
        }
        var shouldHydrateHealth = force || store.connectionState !== 'connected' || !store.runtimeSync;
        if (shouldHydrateHealth) {
          Promise.resolve(self.loadDashboardHealthSummary(store.connectionState !== 'connected')).catch(function() {});
        }
        var now = Date.now();
        var shouldRefreshAgents =
          force ||
          !store.agentsHydrated ||
          (store.connectionState !== 'connected') ||
          (now - Number(store._lastAgentsRefreshAt || 0)) >= 12000;
        if (shouldRefreshAgents) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('agents_refresh_started');
          if (typeof store.refreshAgents === 'function') await store.refreshAgents();
        }
        if (store.agentsHydrated && !store.agentsLoading) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('agents_hydrated');
        }
        if (typeof self.syncChatSidebarTopologyOrderFromAgents === 'function') {
          self.syncChatSidebarTopologyOrderFromAgents();
        }
        self.connected = store.connected;
        self.version = store.version;
        self.agentCount = store.agentCount;
        self.connectionState = store.connectionState || (store.connected ? 'connected' : 'disconnected');
        self.queueConnectionIndicatorState(self.connectionState);
        self.wsConnected = InfringAPI.isWsConnected();
        if (!self.bootSelectionApplied && store.agentsHydrated && !store.agentsLoading) {
          await self.applyBootChatSelection();
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('selection_applied');
        }
        self.scheduleSidebarScrollIndicators();
        if (store.booting === false && store.agentsHydrated && !store.agentsLoading) {
          if (typeof self.setBootProgressEvent === 'function') self.setBootProgressEvent('releasing', { bootStage: store.bootStage });
        }
        self.releaseBootSplash(false);
      })();
      try {
        await this._pollStatusInFlight;
      } finally {
        this._pollStatusInFlight = null;
        if (this._pollStatusQueued) {
          this._pollStatusQueued = false;
          window.setTimeout(function() { self.pollStatus({ force: true }); }, 0);
        }
      }
    }
  };
}
