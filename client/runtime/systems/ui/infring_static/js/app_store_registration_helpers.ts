// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringRegisterAppStoreOnAlpineInit() {
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
}
