// Layer ownership: client/runtime/systems/ui (dashboard static UX surface only; no runtime authority).
function infringShellDelegateMethods() {
  return {
    init: function() {
      return infringInitAppShell(this);
    },
    releaseBootSplash: function(force) {
      return infringReleaseBootSplash(this, force);
    },
    normalizeNavigablePage: function(pageId) {
      return infringNormalizeNavigablePage(pageId);
    },
    isKnownNavigablePage: function(pageId) {
      return infringIsKnownNavigablePage(this, pageId);
    },
    syncPageHistory: function(nextPage) {
      return infringSyncPageHistory(this, nextPage);
    },
    canNavigateBack: function() {
      return infringCanNavigateBack(this);
    },
    canNavigateForward: function() {
      return infringCanNavigateForward(this);
    },
    navigateBackPage: function() {
      return infringNavigateBackPage(this);
    },
    navigateForwardPage: function() {
      return infringNavigateForwardPage(this);
    },
    navigate: function(p) {
      return infringNavigate(this, p);
    },
    setTheme: function(mode) {
      return infringSetTheme(this, mode);
    },
    isChatSidebarSearchActive: function() {
      return infringIsChatSidebarSearchActive(this);
    },
    clearChatSidebarSearch: function() {
      return infringClearChatSidebarSearch(this);
    },
    onChatSidebarQueryInput: function(value) {
      return infringOnChatSidebarQueryInput(this, value);
    },
    scheduleChatSidebarSearch: function() {
      return infringScheduleChatSidebarSearch(this);
    },
    runChatSidebarSearch: async function(seq) {
      return infringRunChatSidebarSearch(this, seq);
    },
    overlayGlassTemplateNormalized: function(modeRaw) {
      return infringOverlayGlassTemplateNormalized(modeRaw);
    },
    applyOverlayGlassTemplate: function(modeRaw, persistRaw) {
      return infringApplyOverlayGlassTemplate(this, modeRaw, persistRaw);
    },
    uiBackgroundTemplateNormalized: function(modeRaw) {
      return infringUiBackgroundTemplateNormalized(this, modeRaw);
    },
    applyUiBackgroundTemplate: function(modeRaw, persistRaw) {
      return infringApplyUiBackgroundTemplate(this, modeRaw, persistRaw);
    },
    beginInstantThemeFlip: function() {
      return infringBeginInstantThemeFlip(this);
    },
    toggleTheme: function() {
      return infringToggleTheme(this);
    },
    toggleSidebar: function() {
      return infringToggleSidebar(this);
    },
    runtimeFacadeHealthSummary: function() {
      return infringRuntimeFacadeHealthSummary(this);
    },
    runtimeFacadeState: function() {
      return infringRuntimeFacadeState(this);
    },
    runtimeFacadeClass: function() {
      return infringRuntimeFacadeClass(this);
    },
    runtimeFacadeLabel: function() {
      return infringRuntimeFacadeLabel(this);
    },
    runtimeFacadeDisplayLabel: function() {
      return infringRuntimeFacadeDisplayLabel(this);
    },
    runtimeResponseP95Ms: function() {
      return infringRuntimeResponseP95Ms(this);
    },
    runtimeConfidencePercent: function() {
      return infringRuntimeConfidencePercent(this);
    },
    runtimeEtaSeconds: function() {
      return infringRuntimeEtaSeconds(this);
    },
    runtimeFacadeDetail: function() {
      return infringRuntimeFacadeDetail(this);
    },
    runtimeFacadeTitle: function() {
      return infringRuntimeFacadeTitle(this);
    },
    taskbarClockParts: function() {
      return infringTaskbarClockParts(this);
    },
    taskbarClockMainLabel: function() {
      return infringTaskbarClockMainLabel(this);
    },
    taskbarClockMeridiemLabel: function() {
      return infringTaskbarClockMeridiemLabel(this);
    },
    taskbarClockLabel: function() {
      return infringTaskbarClockLabel(this);
    },
    agentAutoTerminateEnabled: function(agent) {
      return infringAgentAutoTerminateEnabled(this, agent);
    },
    agentContractRemainingMs: function(agent) {
      return infringAgentContractRemainingMs(this, agent);
    },
    agentContractHasFiniteExpiry: function(agent) {
      return infringAgentContractHasFiniteExpiry(this, agent);
    },
    agentContractTerminationGraceMs: function() {
      return infringAgentContractTerminationGraceMs(this);
    },
    isAgentPendingTermination: function(agent) {
      return infringIsAgentPendingTermination(this, agent);
    },
    shouldShowInfinityLifespan: function(agent) {
      return infringShouldShowInfinityLifespan(this, agent);
    },
    shouldShowExpiryCountdown: function(agent) {
      return infringShouldShowExpiryCountdown(this, agent);
    },
    expiryCountdownLabel: function(agent) {
      return infringExpiryCountdownLabel(this, agent);
    },
    expiryCountdownCritical: function(agent) {
      return infringExpiryCountdownCritical(this, agent);
    },
    agentContractTotalMs: function(agent) {
      return infringAgentContractTotalMs(this, agent);
    },
    agentHeartStates: function(agent) {
      return infringAgentHeartStates(this, agent);
    },
    agentHeartShowsInfinity: function(agent) {
      return infringAgentHeartShowsInfinity(this, agent);
    },
    agentHeartMeterLabel: function(agent) {
      return infringAgentHeartMeterLabel(this, agent);
    },
    closeTaskbarHeroMenu: function() {
      return infringCloseTaskbarHeroMenu(this);
    },
    closeTaskbarTextMenu: function() {
      return infringCloseTaskbarTextMenu(this);
    },
    taskbarTextMenuIsOpen: function(menuName) {
      return infringTaskbarTextMenuIsOpen(this, menuName);
    },
    toggleTaskbarTextMenu: function(menuName) {
      return infringToggleTaskbarTextMenu(this, menuName);
    },
    handleTaskbarHelpManual: function() {
      return infringHandleTaskbarHelpManual(this);
    },
    handleTaskbarHelpReportIssue: function() {
      return infringHandleTaskbarHelpReportIssue(this);
    },
    submitReportIssueDraft: async function() {
      return infringSubmitReportIssueDraft(this);
    },
    manualDocumentMarkdown: function() {
      return infringManualDocumentMarkdown(this);
    },
    manualDocumentHtml: function() {
      return infringManualDocumentHtml(this);
    },
    toggleTaskbarHeroMenu: function() {
      return infringToggleTaskbarHeroMenu(this);
    },
    requestTaskbarRefresh: function() {
      return infringRequestTaskbarRefresh(this);
    },
    postTaskbarHeroSystemRoute: async function(route, body, options) {
      return infringPostTaskbarHeroSystemRoute(this, route, body, options);
    },
    runTaskbarHeroCommand: async function(action) {
      return infringRunTaskbarHeroCommand(this, action);
    },
    normalizeDashboardHealthSummary: function(payload) {
      return infringNormalizeDashboardHealthSummary(this, payload);
    },
    loadDashboardHealthSummary: async function(force) {
      return infringLoadDashboardHealthSummary(this, force);
    },
    pollStatus: async function(opts) {
      return infringPollStatus(this, opts);
    }
  };
}
